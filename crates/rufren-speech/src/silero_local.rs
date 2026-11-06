//! Pinned, offline, CPU-only Silero subprocess. Model selection precedes launch.
use crate::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    sync::{Semaphore, oneshot},
};

const MODEL_SHA: &str = "04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e";
const SOURCE_REVISION: &str = "e382298387332f2bf461fe6eadfde99104611b0c";
const MODEL: &str = "v5_6_ru@04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e;repo=e382298387332f2bf461fe6eadfde99104611b0c";
const MODEL_BYTES: u64 = 147_664_410;
const DEADLINE: Duration = Duration::from_secs(35);
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub(crate) struct SileroLocal {
    root: PathBuf,
    python: PathBuf,
    worker: PathBuf,
    gate: Arc<Semaphore>,
}

impl SileroLocal {
    pub(crate) fn installed() -> Result<Self, SpeechError> {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .map_err(|_| SpeechError::Configuration)?;
        let root = project.join(".local/silero-tts");
        let this = Self {
            python: root.join("venv/bin/python"),
            worker: project.join("scripts/speech/silero_gateway.py"),
            root,
            gate: Arc::new(Semaphore::new(1)),
        };
        if !this.ready() {
            return Err(SpeechError::Configuration);
        }
        Ok(this)
    }

    /// File/manifest readiness only: no Python import, model load or inference.
    /// Every actual operation verifies the complete pinned model SHA in Python.
    pub(crate) fn ready(&self) -> bool {
        let valid = || -> Option<()> {
            if !self.python.is_file() || !self.worker.is_file() {
                return None;
            }
            let model = std::fs::symlink_metadata(self.root.join("models/v5_6_ru.pt")).ok()?;
            if !model.is_file() || model.file_type().is_symlink() || model.len() != MODEL_BYTES {
                return None;
            }
            let manifest_path = self.root.join("model-manifest.json");
            let metadata = std::fs::symlink_metadata(&manifest_path).ok()?;
            if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 65_536 {
                return None;
            }
            use std::io::Read;
            let mut bytes = Vec::new();
            std::fs::File::open(manifest_path)
                .ok()?
                .take(65_537)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() > 65_536 {
                return None;
            }
            let manifest: Value = serde_json::from_slice(&bytes).ok()?;
            if manifest["format"] != "rufren-silero-model-v1"
                || manifest["model"] != "v5_6_ru"
                || manifest["model_sha256"] != MODEL_SHA
                || manifest["source_revision"] != SOURCE_REVISION
                || manifest["runtime"] != "torch==2.15.0"
            {
                return None;
            }
            Some(())
        };
        valid().is_some()
    }

    pub(crate) async fn generate(
        &self,
        request: &SpeechRequest,
        explicit: bool,
    ) -> Result<SpeechResponse, SpeechError> {
        request.validate()?;
        let expected = synthesis_input(&request.text, explicit)?;
        if !self.ready() {
            return Err(SpeechError::Configuration);
        }
        let permit = self
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| SpeechError::RateLimited)?;
        let encoded = serde_json::to_vec(&json!({"schema":"rufren-silero-worker-v1", "text":expected.synthesis_text,
            "request_id":request.request_id,"revision_id":request.revision_id,"source_text":request.text,
            "source_text_sha256":expected.source_text_sha256,"mode":if explicit {"explicit_stress"} else {"automatic"}}))
            .map_err(|_| SpeechError::InvalidRequest)?;
        if encoded.len() > 8192 {
            return Err(SpeechError::InvalidRequest);
        }
        let local = self.clone();
        let (mut sender, receiver) = oneshot::channel();
        // The owned supervisor retains admission until its exact child is killed
        // and reaped, even when HTTP cancellation drops the caller's future.
        tokio::spawn(async move {
            let _permit = permit;
            let result = exchange(
                &local.python,
                &local.worker,
                &local.root,
                encoded,
                DEADLINE,
                &mut sender,
            )
            .await;
            let _ = sender.send(result);
        });
        let bytes = receiver.await.map_err(|_| SpeechError::Unavailable)??;
        let response: SpeechResponse =
            serde_json::from_slice(&bytes).map_err(|_| SpeechError::InvalidResponse)?;
        validate_response(request, &expected, explicit, &response)?;
        Ok(response)
    }
}

fn synthesis_input(text: &str, explicit: bool) -> Result<SpeechSynthesisInput, SpeechError> {
    if text.contains('+') {
        return Err(SpeechError::InvalidRequest);
    }
    let expected = if explicit {
        crate::qwen_frontend::silero_input(text)?
    } else {
        if text.contains('\u{301}') {
            return Err(SpeechError::UnsupportedStress);
        }
        crate::qwen_frontend::input(text)?
    };
    expected.validate_for(text)?;
    Ok(expected)
}

fn validate_response(
    request: &SpeechRequest,
    expected: &SpeechSynthesisInput,
    explicit: bool,
    response: &SpeechResponse,
) -> Result<(), SpeechError> {
    if response.request_id != request.request_id
        || response.revision_id != request.revision_id
        || response.source_text != request.text
        || response.sample_rate != SAMPLE_RATE
        || response.channels != 1
        || response.provenance.kind != SpeechProvenanceKind::AiGeneratedSpeech
        || response.provenance.provider != "local_silero_tts"
        || response.provenance.model != MODEL
        || response.provenance.voice != "xenia"
        || response.provenance.prompt_version
            != if explicit {
                "rufren-silero-explicit-v1"
            } else {
                "rufren-silero-automatic-v1"
            }
        || response.provenance.synthesis_input.as_ref() != Some(expected)
        || response.usage.reserved_micro_usd != 0
        || response.usage.estimated_micro_usd != Some(0)
    {
        return Err(SpeechError::InvalidResponse);
    }
    if response.wav_base64.len() > (MAX_PCM_BYTES + 4096).div_ceil(3) * 4 {
        return Err(SpeechError::ResponseTooLarge);
    }
    let wav = STANDARD
        .decode(&response.wav_base64)
        .map_err(|_| SpeechError::InvalidResponse)?;
    let pcm = crate::wav::decode(&wav)?;
    if response.frames != pcm.len() as u64 / 2
        || response.sha256 != format!("{:x}", Sha256::digest(&wav))
        || crate::wav::encode(pcm) != wav
    {
        return Err(SpeechError::InvalidResponse);
    }
    Ok(())
}

struct RunDirectory(PathBuf);
impl RunDirectory {
    fn create(root: &Path) -> Result<Self, SpeechError> {
        let runs = root.join("runs");
        std::fs::create_dir_all(&runs).map_err(|_| SpeechError::Unavailable)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SpeechError::Unavailable)?
            .as_nanos();
        let path = runs.join(format!(
            "{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&path)
            .map_err(|_| SpeechError::Unavailable)?;
        Ok(Self(path))
    }
}
impl Drop for RunDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("worker.pid"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

async fn exchange(
    python: &Path,
    worker: &Path,
    root: &Path,
    encoded: Vec<u8>,
    deadline: Duration,
    sender: &mut oneshot::Sender<Result<Vec<u8>, SpeechError>>,
) -> Result<Vec<u8>, SpeechError> {
    let directory = RunDirectory::create(root)?;
    let mut command = tokio::process::Command::new(python);
    command
        .arg("-B")
        .arg(worker)
        .current_dir(&directory.0)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for name in [
        "HOME", "USER", "LOGNAME", "PATH", "TMPDIR", "LANG", "LC_ALL",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    for (key, value) in [
        ("PYTHONDONTWRITEBYTECODE", "1"),
        ("PYTHONNOUSERSITE", "1"),
        ("HF_HUB_OFFLINE", "1"),
        ("TRANSFORMERS_OFFLINE", "1"),
        ("OMP_NUM_THREADS", "2"),
        ("OPENBLAS_NUM_THREADS", "2"),
        ("MKL_NUM_THREADS", "2"),
        ("VECLIB_MAXIMUM_THREADS", "2"),
        ("NUMEXPR_NUM_THREADS", "2"),
    ] {
        command.env(key, value);
    }
    let mut child = command.spawn().map_err(|_| SpeechError::Unavailable)?;
    let pid = child.id().ok_or(SpeechError::Unavailable)?;
    let result = async {
        std::fs::write(directory.0.join("worker.pid"),pid.to_string()).map_err(|_| SpeechError::Unavailable)?;
        let mut stdin = child.stdin.take().ok_or(SpeechError::Unavailable)?;
        let stdout = child.stdout.take().ok_or(SpeechError::Unavailable)?;
        let stderr = child.stderr.take().ok_or(SpeechError::Unavailable)?;
        let pipeline = async {
            let ((),bytes,_,status) = tokio::try_join!(async {
                stdin.write_all(&encoded).await.map_err(|_| SpeechError::Unavailable)?;
                stdin.shutdown().await.map_err(|_| SpeechError::Unavailable)?;
                drop(stdin);
                Ok::<(), SpeechError>(())
            }, read_bounded(stdout,MAX_RESPONSE_BYTES), read_bounded(stderr,65_536), async {
                child.wait().await.map_err(|_| SpeechError::Unavailable)
            })?;
            if !status.success() { return Err(SpeechError::Unavailable); }
            Ok(bytes)
        };
        tokio::select! {
            result = tokio::time::timeout(deadline,pipeline) => result.map_err(|_|SpeechError::Timeout)?,
            _ = sender.closed() => Err(SpeechError::Unavailable),
        }
    }.await;
    // Direct child identity is held by tokio, so no process-name search occurs.
    // kill() also waits: admission remains held throughout cancellation cleanup.
    if child.id().is_some() {
        let _ = child.kill().await;
    }
    result
}

async fn read_bounded(
    mut input: impl AsyncRead + Unpin,
    maximum: usize,
) -> Result<Vec<u8>, SpeechError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = input
            .read(&mut buffer)
            .await
            .map_err(|_| SpeechError::Unavailable)?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len() + count > maximum {
            return Err(SpeechError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SpeechRequest {
        SpeechRequest {
            text: "Это за́мок.".into(),
            request_id: "public-silero-1".into(),
            revision_id: Some("revision-1".into()),
        }
    }
    fn response(request: &SpeechRequest) -> SpeechResponse {
        let wav = crate::wav::encode(&[1, 0, 2, 0]);
        SpeechResponse {
            request_id: request.request_id.clone(),
            revision_id: request.revision_id.clone(),
            source_text: request.text.clone(),
            wav_base64: STANDARD.encode(&wav),
            sha256: format!("{:x}", Sha256::digest(&wav)),
            sample_rate: SAMPLE_RATE,
            channels: 1,
            frames: 2,
            provenance: SpeechProvenance {
                kind: SpeechProvenanceKind::AiGeneratedSpeech,
                provider: "local_silero_tts".into(),
                model: MODEL.into(),
                voice: "xenia".into(),
                prompt_version: "rufren-silero-explicit-v1".into(),
                validation: "Public mock only".into(),
                synthesis_input: Some(synthesis_input(&request.text, true).unwrap()),
            },
            usage: SpeechUsage {
                input_tokens: None,
                output_tokens: None,
                estimated_micro_usd: Some(0),
                reserved_micro_usd: 0,
            },
        }
    }
    #[test]
    fn speech_silero_original_checks_source_actual_input_model_and_wav() {
        let request = request();
        let expected = synthesis_input(&request.text, true).unwrap();
        let good = response(&request);
        validate_response(&request, &expected, true, &good).unwrap();
        for index in 0..7 {
            let mut bad = good.clone();
            match index {
                0 => bad.source_text = "Это замо́к.".into(),
                1 => bad.revision_id = None,
                2 => bad.provenance.model = "unversioned".into(),
                3 => {
                    bad.provenance
                        .synthesis_input
                        .as_mut()
                        .unwrap()
                        .synthesis_text = "Это замок.".into()
                }
                4 => bad.sha256 = "0".repeat(64),
                5 => bad.wav_base64 = STANDARD.encode(b"RIFF-invalid"),
                _ => bad.frames = 3,
            }
            assert!(
                validate_response(&request, &expected, true, &bad).is_err(),
                "{index}"
            );
        }
        assert!(synthesis_input("Это з+амок.", false).is_err());
        assert!(synthesis_input(&request.text, false).is_err());
        assert!(synthesis_input("Это з́амок.", true).is_err());
    }

    struct Fixture {
        root: PathBuf,
        script: PathBuf,
    }
    impl Fixture {
        fn new(source: &str) -> Self {
            let directory = RunDirectory::create(&std::env::temp_dir()).unwrap();
            let root = directory.0.clone();
            std::mem::forget(directory);
            let script = root.join("mock_worker.py");
            std::fs::write(&script, source).unwrap();
            Self { root, script }
        }
        async fn run(&self, deadline: Duration) -> Result<Vec<u8>, SpeechError> {
            let (mut tx, _rx) = oneshot::channel();
            exchange(
                Path::new("/usr/bin/python3"),
                &self.script,
                &self.root,
                b"{}".to_vec(),
                deadline,
                &mut tx,
            )
            .await
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.script);
            let _ = std::fs::remove_dir(self.root.join("runs"));
            let _ = std::fs::remove_dir(&self.root);
        }
    }
    #[tokio::test]
    async fn speech_silero_subprocess_bounds_timeout_and_failure_without_inference() {
        let fixture =
            Fixture::new("import sys\nassert sys.stdin.read() == '{}'\nprint('{}',end='')\n");
        assert_eq!(fixture.run(Duration::from_secs(3)).await.unwrap(), b"{}");
        let fixture = Fixture::new("import time\ntime.sleep(5)\n");
        assert!(matches!(
            fixture.run(Duration::from_millis(20)).await,
            Err(SpeechError::Timeout)
        ));
        assert_eq!(
            std::fs::read_dir(fixture.root.join("runs"))
                .unwrap()
                .count(),
            0
        );
        let fixture = Fixture::new("import sys\nsys.exit(2)\n");
        assert!(matches!(
            fixture.run(Duration::from_secs(3)).await,
            Err(SpeechError::Unavailable)
        ));
        let fixture = Fixture::new("import sys\nsys.stdout.write('x' * (4*1024*1024+1))\n");
        assert!(matches!(
            fixture.run(Duration::from_secs(3)).await,
            Err(SpeechError::ResponseTooLarge)
        ));
        let fixture = Fixture::new("import sys\nsys.stderr.write('x' * 65537)\n");
        assert!(matches!(
            fixture.run(Duration::from_secs(3)).await,
            Err(SpeechError::ResponseTooLarge)
        ));
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn speech_silero_cancellation_reaps_owned_child_before_admission_release() {
        let fixture = Fixture::new("import time\ntime.sleep(5)\n");
        let root = fixture.root.clone();
        let script = fixture.script.clone();
        let gate = Arc::new(Semaphore::new(1));
        let permit = gate.clone().try_acquire_owned().unwrap();
        let (mut sender, receiver) = oneshot::channel();
        let task = tokio::spawn(async move {
            let _permit = permit;
            exchange(
                Path::new("/usr/bin/python3"),
                &script,
                &root,
                b"{}".to_vec(),
                Duration::from_secs(3),
                &mut sender,
            )
            .await
        });
        let pid = tokio::time::timeout(Duration::from_secs(2), async {
            'pid: loop {
                if let Ok(entries) = std::fs::read_dir(fixture.root.join("runs")) {
                    for entry in entries.flatten() {
                        if let Ok(text) = std::fs::read_to_string(entry.path().join("worker.pid")) {
                            if let Ok(pid) = text.parse::<i32>() {
                                break 'pid pid;
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        // See below: polling returns only the exact owned operation's PID file.
        let pid = pid.unwrap();
        assert_eq!(gate.available_permits(), 0);
        drop(receiver);
        assert!(matches!(task.await.unwrap(), Err(SpeechError::Unavailable)));
        assert_eq!(gate.available_permits(), 1);
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}
