//! Explicit installed-system voice adapter. No shell, network service or audible playback.
use crate::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Files {
    root: PathBuf,
}
impl Files {
    fn create() -> Result<Self, SpeechError> {
        use std::os::unix::fs::DirBuilderExt;
        for _ in 0..8 {
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| SpeechError::Unavailable)?
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "rufren-speech-{}-{time}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::DirBuilder::new().mode(0o700).create(&root) {
                Ok(()) => return Ok(Self { root }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(SpeechError::Unavailable),
            }
        }
        Err(SpeechError::Unavailable)
    }
    /// Remove only this operation's named outputs, after its child has stopped.
    fn clean(&self) {
        let _ = std::fs::remove_file(self.root.join("reference.wav"));
        let _ = std::fs::remove_file(self.root.join("say.pid"));
        let _ = std::fs::remove_dir(&self.root);
    }
}

pub(crate) async fn generate(request: &SpeechRequest) -> Result<SpeechResponse, SpeechError> {
    let files = Files::create()?;
    let output = files.root.join("reference.wav");
    let mut child = match tokio::process::Command::new("/usr/bin/say")
        .args([
            "-v",
            "Milena",
            "-f",
            "-",
            "--file-format=WAVE",
            "--data-format=LEI16@24000",
            "--channels=1",
            "-o",
        ])
        .arg(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            files.clean();
            return Err(SpeechError::Unavailable);
        }
    };
    // Track this exact child rather than searching or killing by process name.
    if std::fs::write(
        files.root.join("say.pid"),
        child.id().ok_or(SpeechError::Unavailable)?.to_string(),
    )
    .is_err()
    {
        let _ = child.kill().await;
        files.clean();
        return Err(SpeechError::Unavailable);
    }
    let result = async {
        let mut stdin = child.stdin.take().ok_or(SpeechError::Unavailable)?;
        stdin
            .write_all(request.text.as_bytes())
            .await
            .map_err(|_| SpeechError::Unavailable)?;
        stdin
            .shutdown()
            .await
            .map_err(|_| SpeechError::Unavailable)?;
        drop(stdin);
        let status = child.wait().await.map_err(|_| SpeechError::Unavailable)?;
        if !status.success() {
            return Err(SpeechError::Unavailable);
        }
        let metadata = std::fs::symlink_metadata(&output).map_err(|_| SpeechError::Unavailable)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(SpeechError::InvalidResponse);
        }
        if metadata.len() > MAX_PCM_BYTES as u64 + 4096 {
            return Err(SpeechError::ResponseTooLarge);
        }
        let bytes = std::fs::read(&output).map_err(|_| SpeechError::Unavailable)?;
        let pcm = crate::wav::decode(&bytes)?;
        let wav = crate::wav::encode(pcm);
        Ok(SpeechResponse{request_id:request.request_id.clone(),revision_id:request.revision_id.clone(),source_text:request.text.clone(),wav_base64:STANDARD.encode(&wav),sha256:format!("{:x}",Sha256::digest(&wav)),sample_rate:SAMPLE_RATE,channels:1,frames:(pcm.len()/2)as u64,
            provenance:SpeechProvenance{kind:SpeechProvenanceKind::SystemGeneratedSpeech,provider:"macos_say".into(),model:"Milena".into(),voice:"Milena".into(),prompt_version:"rufren-macos-russian-v1".into(),validation:"Installed Russian system voice; not Gemini, a human recording or a verified learner pronunciation assessment".into(),synthesis_input:None},
            usage:SpeechUsage{input_tokens:None,output_tokens:None,estimated_micro_usd:Some(0),reserved_micro_usd:0}})
    };
    let outcome = match tokio::time::timeout(Duration::from_secs(30), result).await {
        Ok(value) => value,
        Err(_) => {
            let _ = child.kill().await;
            Err(SpeechError::Timeout)
        }
    };
    // If input or waiting failed before normal exit, terminate/reap before file cleanup.
    if child.id().is_some() {
        let _ = child.kill().await;
    }
    files.clean();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_local_wav_parser_rejects_invalid_formats_and_lengths() {
        let wav = crate::wav::encode(&[1, 0, 2, 0]);
        assert_eq!(crate::wav::decode(&wav).unwrap(), [1, 0, 2, 0]);
        for offset in [0, 4, 8, 20, 22, 24, 28, 32, 34, 40] {
            let mut bad = wav.clone();
            bad[offset] ^= 1;
            assert!(crate::wav::decode(&bad).is_err(), "{offset}");
        }
        assert!(crate::wav::decode(&wav[..wav.len() - 1]).is_err());
    }
    #[tokio::test]
    async fn speech_local_installed_russian_voice_produces_bound_original() {
        let service = crate::SpeechService::macos().unwrap();
        let request = SpeechRequest {
            text: "Здравствуйте! Я учу русский язык.".into(),
            request_id: "local-public-fixture".into(),
            revision_id: None,
        };
        let result = service.generate(&request).await.unwrap();
        assert_eq!(
            result.provenance.kind,
            SpeechProvenanceKind::SystemGeneratedSpeech
        );
        assert_eq!(result.source_text, request.text);
        assert_eq!(result.usage.reserved_micro_usd, 0);
        assert!(result.frames > 2400);
        let wav = STANDARD.decode(result.wav_base64).unwrap();
        assert_eq!(result.sha256, format!("{:x}", Sha256::digest(&wav)));
        assert!(crate::wav::decode(&wav).is_ok());
    }
}
