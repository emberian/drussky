//! One independent, offline ASR operation. No lesson text is sent to this worker.
use crate::recognition::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, sleep_until, timeout};

const STDERR_LIMIT: usize = 262_144;
const CLEANUP_SECONDS: u64 = 8;
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Default)]
pub struct RecognitionCancellation {
    state: Arc<CancellationState>,
}
#[derive(Default)]
struct CancellationState {
    cancelled: AtomicBool,
    notify: Notify,
}
impl RecognitionCancellation {
    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::Release);
        self.state.notify.notify_waiters();
    }
    async fn cancelled(&self) {
        let notified = self.state.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if self.state.cancelled.load(Ordering::Acquire) {
            return;
        }
        notified.await;
    }
}

struct Configuration {
    python: PathBuf,
    worker: PathBuf,
    manifest: PathBuf,
    worker_sha256: String,
    manifest_sha256: String,
    deadline: Duration,
    max_requests: u64,
}

#[derive(Clone)]
pub struct RecognitionService {
    config: Option<Arc<Configuration>>,
    gate: Arc<Semaphore>,
    requests: Arc<AtomicU64>,
}

impl Default for RecognitionService {
    fn default() -> Self {
        Self::disabled()
    }
}
impl RecognitionService {
    pub fn disabled() -> Self {
        Self {
            config: None,
            gate: Arc::new(Semaphore::new(1)),
            requests: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Explicit private-server configuration; no model import, inference,
    /// credential inspection, network request or runtime download occurs here.
    pub fn from_env() -> Result<Self, RecognitionError> {
        match std::env::var("RUFREN_ASR_PROVIDER")
            .unwrap_or_default()
            .as_str()
        {
            "" | "disabled" => return Ok(Self::disabled()),
            "local_qwen3_asr" => (),
            _ => return Err(RecognitionError::Configuration),
        }
        let path = |key| {
            std::env::var_os(key)
                .map(PathBuf::from)
                .ok_or(RecognitionError::Configuration)
        };
        let count = std::env::var("RUFREN_ASR_MAX_REQUESTS")
            .map(|n| {
                n.parse::<u64>()
                    .map_err(|_| RecognitionError::Configuration)
            })
            .unwrap_or(Ok(20))?;
        Self::configured(
            path("RUFREN_ASR_PYTHON")?,
            path("RUFREN_ASR_WORKER")?,
            path("RUFREN_ASR_MANIFEST")?,
            count,
            Duration::from_secs(RECOGNITION_DEADLINE_SECONDS),
        )
    }

    fn configured(
        python: PathBuf,
        worker: PathBuf,
        manifest: PathBuf,
        max_requests: u64,
        deadline: Duration,
    ) -> Result<Self, RecognitionError> {
        if !cfg!(unix)
            || !(1..=1000).contains(&max_requests)
            || deadline.is_zero()
            || [&python, &worker, &manifest]
                .iter()
                .any(|p| !p.is_absolute() || !p.is_file())
        {
            return Err(RecognitionError::Configuration);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if python
                .metadata()
                .map_err(|_| RecognitionError::Configuration)?
                .permissions()
                .mode()
                & 0o111
                == 0
            {
                return Err(RecognitionError::Configuration);
            }
        }
        let worker_sha256 = fingerprint(&worker, 1_048_576)?;
        let manifest_bytes = read_small_file(&manifest, 65_536)?;
        let model: Value =
            serde_json::from_slice(&manifest_bytes).map_err(|_| RecognitionError::Configuration)?;
        if model.get("format").and_then(Value::as_str) != Some("rufren-asr-model-v1")
            || model.get("model").and_then(Value::as_str) != Some(RECOGNITION_MODEL)
            || model.get("revision").and_then(Value::as_str) != Some(RECOGNITION_REVISION)
            || model.get("runtime").and_then(Value::as_str) != Some(RECOGNITION_RUNTIME)
        {
            return Err(RecognitionError::Configuration);
        }
        Ok(Self {
            config: Some(Arc::new(Configuration {
                python,
                worker,
                manifest,
                worker_sha256,
                manifest_sha256: digest(&manifest_bytes),
                deadline,
                max_requests,
            })),
            gate: Arc::new(Semaphore::new(1)),
            requests: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn status(&self) -> RecognitionStatus {
        let enabled = self.config.is_some();
        RecognitionStatus {
            enabled, provider: enabled.then(|| "local_qwen3_asr".into()), model: enabled.then(|| RECOGNITION_MODEL.into()),
            revision: enabled.then(|| RECOGNITION_REVISION.into()),
            message: if enabled { "Local Russian transcription is configured. The first request verifies and loads the offline model; transcripts need confirmation. This process's request allowance resets on restart." }
                else { "Local Russian transcription is not configured. Saved recordings and playback remain available." }.into(),
            data_sent: "The exact selected original WAV and crop are sent to your private relay's offline worker. No lesson, intended answer or hotwords are supplied; the recognizer uses installed model files.".into(),
            max_audio_bytes: RECOGNITION_MAX_AUDIO_BYTES, max_duration_seconds: RECOGNITION_MAX_DURATION_SECONDS,
            remaining_requests: self.config.as_ref().map_or(0, |c| c.max_requests.saturating_sub(self.requests.load(Ordering::Acquire))),
        }
    }

    pub async fn recognize(
        &self,
        request: &RecognitionRequest,
    ) -> Result<RecognitionResponse, RecognitionError> {
        self.recognize_cancellable(request, RecognitionCancellation::default())
            .await
    }

    pub async fn recognize_cancellable(
        &self,
        request: &RecognitionRequest,
        cancellation: RecognitionCancellation,
    ) -> Result<RecognitionResponse, RecognitionError> {
        let config = self.config.as_ref().ok_or(RecognitionError::Disabled)?;
        let started = Instant::now();
        request.validate()?;
        let permit = self
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| RecognitionError::Busy)?;
        if cancellation.state.cancelled.load(Ordering::Acquire) {
            return Err(RecognitionError::Cancelled);
        }
        let original = validate_original(request)?;
        let encoded = serde_json::to_vec(request).map_err(|_| RecognitionError::InvalidRequest)?;
        if encoded.len() > RECOGNITION_MAX_REQUEST_BYTES {
            return Err(RecognitionError::ResponseTooLarge);
        }
        if fingerprint(&config.worker, 1_048_576)? != config.worker_sha256
            || fingerprint(&config.manifest, 65_536)? != config.manifest_sha256
        {
            return Err(RecognitionError::Configuration);
        }
        self.requests
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < config.max_requests).then_some(n + 1)
            })
            .map_err(|_| RecognitionError::BudgetExceeded)?;
        let directory = RunDirectory::new()?;
        let mut command = Command::new(&config.python);
        command
            .arg("-B")
            .arg(&config.worker)
            .current_dir(&directory.0)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false);
        for key in [
            "HOME", "USER", "LOGNAME", "PATH", "TMPDIR", "LANG", "LC_ALL",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        for (key, value) in [
            ("HF_HUB_OFFLINE", "1"),
            ("TRANSFORMERS_OFFLINE", "1"),
            ("HF_HUB_DISABLE_TELEMETRY", "1"),
            ("PYTHONNOUSERSITE", "1"),
            ("PYTHONDONTWRITEBYTECODE", "1"),
            ("TOKENIZERS_PARALLELISM", "false"),
            ("OMP_NUM_THREADS", "2"),
            ("OPENBLAS_NUM_THREADS", "2"),
            ("MKL_NUM_THREADS", "2"),
            ("VECLIB_MAXIMUM_THREADS", "2"),
            ("NUMEXPR_NUM_THREADS", "2"),
            ("RAYON_NUM_THREADS", "2"),
        ] {
            command.env(key, value);
        }
        let child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                directory.cleanup();
                return Err(RecognitionError::Unavailable);
            }
        };
        let mut owned = OwnedWorker {
            child: Some(child),
            directory: Some(directory),
            permit: Some(permit),
        };
        let pid = owned
            .child
            .as_ref()
            .and_then(Child::id)
            .ok_or(RecognitionError::Unavailable)?;
        if std::fs::write(
            owned.directory.as_ref().unwrap().0.join("worker.pid"),
            pid.to_string(),
        )
        .is_err()
        {
            owned.stop().await;
            return Err(RecognitionError::Unavailable);
        }
        // Eight seconds are reserved inside the total deadline so SIGTERM can
        // let the Python supervisor stop/reap its separate model process group.
        let cleanup = config.deadline.min(Duration::from_secs(CLEANUP_SECONDS));
        let work_deadline = started
            + config
                .deadline
                .saturating_sub(cleanup.min(config.deadline / 4));
        let result = {
            let child = owned.child.as_mut().unwrap();
            let stdin = child.stdin.take().ok_or(RecognitionError::Unavailable)?;
            let stdout = child.stdout.take().ok_or(RecognitionError::Unavailable)?;
            let stderr = child.stderr.take().ok_or(RecognitionError::Unavailable)?;
            let pipeline = async {
                let ((), bytes, (), status) = tokio::try_join!(
                    write_request(stdin, &encoded),
                    read_output(stdout),
                    drain_errors(stderr),
                    async {
                        child
                            .wait()
                            .await
                            .map_err(|_| RecognitionError::Unavailable)
                    }
                )?;
                if !status.success() {
                    return Err(RecognitionError::Unavailable);
                }
                let response: RecognitionResponse = serde_json::from_slice(&bytes)
                    .map_err(|_| RecognitionError::InvalidResponse)?;
                response.validate_for(request)?;
                if response.original_audio != original.original_audio
                    || response.source_pcm_sha256 != original.source_pcm_sha256
                    || response.provenance.worker_sha256 != config.worker_sha256
                    || response.provenance.manifest_sha256 != config.manifest_sha256
                {
                    return Err(RecognitionError::InvalidResponse);
                }
                Ok(response)
            };
            tokio::select! {
                result = pipeline => result,
                _ = sleep_until(work_deadline) => Err(RecognitionError::Timeout),
                _ = cancellation.cancelled() => Err(RecognitionError::Cancelled),
            }
        };
        owned.stop().await;
        result
    }
}

async fn write_request(
    mut stdin: tokio::process::ChildStdin,
    bytes: &[u8],
) -> Result<(), RecognitionError> {
    stdin
        .write_all(bytes)
        .await
        .map_err(|_| RecognitionError::Unavailable)?;
    stdin
        .shutdown()
        .await
        .map_err(|_| RecognitionError::Unavailable)
}
async fn read_output(mut stdout: impl AsyncRead + Unpin) -> Result<Vec<u8>, RecognitionError> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let size = stdout
            .read(&mut buffer)
            .await
            .map_err(|_| RecognitionError::Unavailable)?;
        if size == 0 {
            return Ok(bytes);
        }
        if bytes.len() + size > RECOGNITION_MAX_RESPONSE_BYTES {
            return Err(RecognitionError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&buffer[..size]);
    }
}
async fn drain_errors(mut stderr: impl AsyncRead + Unpin) -> Result<(), RecognitionError> {
    let mut total = 0;
    let mut buffer = [0; 8192];
    loop {
        let size = stderr
            .read(&mut buffer)
            .await
            .map_err(|_| RecognitionError::Unavailable)?;
        if size == 0 {
            return Ok(());
        }
        total += size;
        if total > STDERR_LIMIT {
            return Err(RecognitionError::ResponseTooLarge);
        }
        // Discard bounded diagnostics; paths, recordings and transcripts do not
        // enter returned errors or gateway logs.
    }
}

fn read_small_file(path: &Path, limit: usize) -> Result<Vec<u8>, RecognitionError> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|_| RecognitionError::Configuration)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| RecognitionError::Configuration)?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(RecognitionError::Configuration);
    }
    Ok(bytes)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn fingerprint(path: &Path, limit: usize) -> Result<String, RecognitionError> {
    Ok(digest(&read_small_file(path, limit)?))
}

fn validate_original(
    request: &RecognitionRequest,
) -> Result<RecognitionOriginalEvidence, RecognitionError> {
    let bytes = STANDARD
        .decode(&request.wav_base64)
        .map_err(|_| RecognitionError::InvalidRequest)?;
    if bytes.len() > RECOGNITION_MAX_AUDIO_BYTES {
        return Err(RecognitionError::UnsupportedAudio);
    }
    if digest(&bytes) != request.audio_sha256 {
        return Err(RecognitionError::HashMismatch);
    }
    let (evidence, crop_rms) = inspect_recognition_wav_crop(&bytes, request.span.as_ref())?;
    if crop_rms < 1e-8 {
        return Err(RecognitionError::UnsupportedAudio);
    }
    Ok(evidence)
}

struct RunDirectory(PathBuf);
impl RunDirectory {
    fn new() -> Result<Self, RecognitionError> {
        for _ in 0..32 {
            let serial = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("rufren-asr-{}-{serial}", std::process::id()));
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(RecognitionError::Unavailable),
            }
        }
        Err(RecognitionError::Unavailable)
    }
    fn cleanup(self) {
        let _ = std::fs::remove_dir_all(self.0);
    }
}

struct OwnedWorker {
    child: Option<Child>,
    directory: Option<RunDirectory>,
    permit: Option<OwnedSemaphorePermit>,
}
impl OwnedWorker {
    async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if !terminate(&mut child, false).await {
                // Preserve the permit if an OS-level wait remains outstanding.
                let directory = self.directory.take();
                let permit = self.permit.take();
                tokio::spawn(async move {
                    let _ = child.wait().await;
                    if let Some(d) = directory {
                        d.cleanup();
                    }
                    drop(permit);
                });
                return;
            }
        }
        if let Some(directory) = self.directory.take() {
            directory.cleanup();
        }
        self.permit.take();
    }
}
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Send TERM synchronously even during task/runtime shutdown, giving
            // the supervisor its chance to clean its separately owned group.
            signal_term(&child);
            let directory = self.directory.take();
            let permit = self.permit.take();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    if !terminate(&mut child, true).await {
                        let _ = child.wait().await;
                    }
                    if let Some(d) = directory {
                        d.cleanup();
                    }
                    drop(permit);
                });
            }
            // With no runtime, leave the PID directory intact; never delete a
            // running worker's directory. The TERM handler still cleans its model.
        }
    }
}
fn signal_term(child: &Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // This live, unreaped child handle owns the PID; it cannot be reused.
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGTERM);
        }
    }
}
async fn terminate(child: &mut Child, term_already_sent: bool) -> bool {
    if child.try_wait().ok().flatten().is_some() {
        return true;
    }
    if !term_already_sent {
        signal_term(child);
    }
    match timeout(Duration::from_secs(7), child.wait()).await {
        Ok(Ok(_)) => true,
        Ok(Err(_)) => false,
        Err(_) => {
            let _ = child.start_kill();
            matches!(
                timeout(Duration::from_secs(1), child.wait()).await,
                Ok(Ok(_))
            )
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    fn request() -> RecognitionRequest {
        let samples: Vec<i16> = (0..32)
            .map(|i| if i % 2 == 0 { 8192 } else { -8192 })
            .collect();
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&(36 + samples.len() as u32 * 2).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&8000_u32.to_le_bytes());
        wav.extend_from_slice(&16000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        RecognitionRequest {
            wav_base64: STANDARD.encode(&wav),
            audio_sha256: digest(&wav),
            ..crate::recognition::tests::sample_request()
        }
    }

    struct Fixture {
        path: PathBuf,
        service: RecognitionService,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    fn fixture(mode: &str, max_requests: u64, deadline: Duration) -> Fixture {
        let directory = RunDirectory::new().unwrap();
        let worker = directory.0.join("mock_asr.py");
        let manifest = directory.0.join("manifest.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(
                &json!({"format":"rufren-asr-model-v1","model":RECOGNITION_MODEL,
            "revision":RECOGNITION_REVISION,"runtime":RECOGNITION_RUNTIME,"files":[]}),
            )
            .unwrap(),
        )
        .unwrap();
        let original = request();
        let mut response = crate::recognition::tests::sample_response(&original);
        // Independently known PCM16 samples become exact +/-0.25 f32 values.
        let mut pcm = Vec::new();
        for n in 0..32 {
            pcm.extend_from_slice(&(if n % 2 == 0 { 0.25_f32 } else { -0.25_f32 }).to_le_bytes());
        }
        response.source_pcm_sha256 = digest(&pcm);
        let response = serde_json::to_string(&response).unwrap();
        let path = directory.0.to_str().unwrap();
        let program = format!(
            r#"import hashlib, json, os, signal, sys, time
from pathlib import Path
mode={mode:?}
root=Path({path:?})
def terminate(sig, frame):
    time.sleep(0.05)
    (root/'terminated').write_text('graceful')
    sys.exit(0)
signal.signal(signal.SIGTERM,terminate)
(root/'child.pid').write_text(str(os.getpid()))
assert os.environ['HF_HUB_OFFLINE']=='1' and os.environ['TRANSFORMERS_OFFLINE']=='1'
assert os.environ['PYTHONNOUSERSITE']=='1' and 'PYTHONPATH' not in os.environ
assert sys.argv[1:]==[]
if mode=='stderr_before_stdin':
    sys.stderr.write('PRIVATE-RECORDING-DIAGNOSTIC'*12000);sys.stderr.flush();time.sleep(5)
r=json.load(sys.stdin)
(root/'received.json').write_text(json.dumps(r))
with (root/'calls').open('a') as file:file.write('1\n')
if mode in ['timeout','cancel']:time.sleep(10)
if mode=='stdout_flood':
    sys.stdout.write('x'*65537);sys.stdout.flush();time.sleep(5)
if mode=='bad_json':print('PRIVATE-RECORDING invalid json');sys.exit(1)
response=json.loads({response:?})
response['provenance']['worker_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
response['provenance']['manifest_sha256']=hashlib.sha256((root/'manifest.json').read_bytes()).hexdigest()
if mode=='binding':response['attempt_id']='different-attempt'
if mode=='crop':response['span']['end_frame']=21
if mode=='source_hash':response['source_pcm_sha256']='f'*64
if mode=='shape':response['original_audio']['frames']=64
if mode=='worker_hash':response['provenance']['worker_sha256']='f'*64
if mode=='manifest_hash':response['provenance']['manifest_sha256']='f'*64
if mode=='target':response['target_conditioned']=True
if mode=='alignment':response['word_timestamps']=[{{'word':'Привет','start':0.0,'end':1.0}}]
print(json.dumps(response,ensure_ascii=False))
"#
        );
        std::fs::write(&worker, program).unwrap();
        std::fs::set_permissions(&worker, std::fs::Permissions::from_mode(0o600)).unwrap();
        let service = RecognitionService::configured(
            PathBuf::from("/usr/bin/python3"),
            worker,
            manifest,
            max_requests,
            deadline,
        )
        .unwrap();
        Fixture {
            path: directory.0,
            service,
        }
    }

    async fn wait_for(path: &Path) {
        for _ in 0..100 {
            if path.is_file() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("mock worker did not reach expected checkpoint");
    }
    fn assert_reaped(path: &Path) {
        let pid = std::fs::read_to_string(path).unwrap();
        let out = std::process::Command::new("/bin/ps")
            .args(["-p", &pid, "-o", "pid="])
            .output()
            .unwrap();
        assert!(out.stdout.is_empty(), "owned worker must be reaped");
    }

    #[tokio::test]
    async fn recognition_server_exact_wav_crop_provenance_and_process_budget() {
        let fixture = fixture("normal", 2, Duration::from_secs(3));
        let request = request();
        let response = fixture.service.recognize(&request).await.unwrap();
        assert_eq!(response.transcript, "Привет!");
        assert_eq!(response.span, request.span.clone().unwrap());
        assert_eq!(response.audio_sha256, request.audio_sha256);
        assert_eq!(response.attempt_id, request.attempt_id);
        assert_eq!(fixture.service.status().remaining_requests, 1);
        let observed: Value =
            serde_json::from_slice(&std::fs::read(fixture.path.join("received.json")).unwrap())
                .unwrap();
        assert_eq!(observed, serde_json::to_value(&request).unwrap());
        let mut invalid = request.clone();
        invalid.audio_sha256 = "f".repeat(64);
        assert!(matches!(
            fixture.service.recognize(&invalid).await,
            Err(RecognitionError::HashMismatch)
        ));
        assert_eq!(fixture.service.status().remaining_requests, 1);
        fixture.service.recognize(&request).await.unwrap();
        assert!(matches!(
            fixture.service.recognize(&request).await,
            Err(RecognitionError::BudgetExceeded)
        ));
        assert_eq!(
            std::fs::read_to_string(fixture.path.join("calls"))
                .unwrap()
                .lines()
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn recognition_server_rejects_stale_or_unearned_evidence() {
        for mode in [
            "binding",
            "crop",
            "source_hash",
            "shape",
            "worker_hash",
            "manifest_hash",
            "target",
            "alignment",
        ] {
            let fixture = fixture(mode, 2, Duration::from_secs(3));
            assert!(
                matches!(
                    fixture.service.recognize(&request()).await,
                    Err(RecognitionError::InvalidResponse)
                ),
                "{mode}"
            );
        }
    }

    #[tokio::test]
    async fn recognition_server_bounds_stdout_and_drains_stderr_before_stdin_without_leaks() {
        for mode in ["stdout_flood", "stderr_before_stdin"] {
            let fixture = fixture(mode, 2, Duration::from_secs(3));
            assert!(
                matches!(
                    fixture.service.recognize(&request()).await,
                    Err(RecognitionError::ResponseTooLarge)
                ),
                "{mode}"
            );
            assert_reaped(&fixture.path.join("child.pid"));
        }
        let fixture = fixture("bad_json", 2, Duration::from_secs(3));
        let error = fixture.service.recognize(&request()).await.unwrap_err();
        assert!(!error.to_string().contains("PRIVATE"));
    }

    #[tokio::test]
    async fn recognition_server_timeout_gracefully_stops_and_reaps_its_worker() {
        let fixture = fixture("timeout", 2, Duration::from_millis(600));
        let started = Instant::now();
        assert!(matches!(
            fixture.service.recognize(&request()).await,
            Err(RecognitionError::Timeout)
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(fixture.path.join("terminated").is_file());
        assert_reaped(&fixture.path.join("child.pid"));
    }

    #[tokio::test]
    async fn recognition_server_cancellation_and_dropped_future_hold_single_inference_permit() {
        let fixture = fixture("cancel", 4, Duration::from_secs(4));
        let service = fixture.service.clone();
        let cancellation = RecognitionCancellation::default();
        let cancel = cancellation.clone();
        let task =
            tokio::spawn(async move { service.recognize_cancellable(&request(), cancel).await });
        wait_for(&fixture.path.join("received.json")).await;
        assert!(matches!(
            fixture.service.recognize(&request()).await,
            Err(RecognitionError::Busy)
        ));
        cancellation.cancel();
        assert!(matches!(
            task.await.unwrap(),
            Err(RecognitionError::Cancelled)
        ));
        assert_reaped(&fixture.path.join("child.pid"));
        std::fs::remove_file(fixture.path.join("received.json")).unwrap();
        std::fs::remove_file(fixture.path.join("terminated")).unwrap();
        let service = fixture.service.clone();
        let task = tokio::spawn(async move { service.recognize(&request()).await });
        wait_for(&fixture.path.join("received.json")).await;
        task.abort();
        let _ = task.await;
        wait_for(&fixture.path.join("terminated")).await;
        for _ in 0..100 {
            if fixture.service.gate.available_permits() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(fixture.service.gate.available_permits(), 1);
        assert_reaped(&fixture.path.join("child.pid"));
    }

    #[test]
    fn recognition_server_rejects_malformed_unsupported_and_silent_originals_before_spawn() {
        let source = request();
        let original = STANDARD.decode(&source.wav_base64).unwrap();
        for prefix in 0..original.len() {
            let mut request = source.clone();
            request.wav_base64 = STANDARD.encode(&original[..prefix]);
            request.audio_sha256 = digest(&original[..prefix]);
            assert!(validate_original(&request).is_err(), "prefix {prefix}");
        }
        for offset in [20, 22, 24, 28, 32, 34, 40] {
            let mut bytes = original.clone();
            bytes[offset] = 255;
            let mut request = source.clone();
            request.audio_sha256 = digest(&bytes);
            request.wav_base64 = STANDARD.encode(bytes);
            assert!(validate_original(&request).is_err(), "offset {offset}");
        }
        let mut silent = original.clone();
        silent[44..].fill(0);
        let mut request = source.clone();
        request.audio_sha256 = digest(&silent);
        request.wav_base64 = STANDARD.encode(silent);
        assert!(matches!(
            validate_original(&request),
            Err(RecognitionError::UnsupportedAudio)
        ));
        let mut request = source;
        request.span.as_mut().unwrap().end_frame = u64::MAX;
        assert!(matches!(
            validate_original(&request),
            Err(RecognitionError::InvalidRequest)
        ));
    }
}
