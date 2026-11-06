//! Exact recording → independent transcript → learner-adopted words.
use crate::{
    model::AppState,
    repository::{now_ms, Result},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rufren_core::*;
use rufren_speech::{inspect_recognition_wav, RecognitionRequest, RecognitionResponse};
use sha2::{Digest, Sha256};
use uuid::Uuid;

impl AppState {
    pub(crate) fn recognition_request(
        &self,
        recording: Uuid,
        bytes: &[u8],
        request_id: String,
    ) -> Result<RecognitionRequest> {
        let Some(Fact::RecordingAttached(attachment)) =
            self.library.events.get(&recording).map(|e| &e.fact)
        else {
            return Err("That saved recording is unavailable.".into());
        };
        let AnnotationTarget::Attempt(attempt_id) = attachment.target else {
            return Err("Choose your own saved speaking response.".into());
        };
        let attempt = self
            .library
            .attempt(attempt_id)
            .ok_or("Sync the speaking response before transcribing it.")?;
        if attachment.role != AudioRole::Attempt || attempt.skill != Skill::Speaking {
            return Err("Choose your own saved speaking response.".into());
        }
        if bytes.len() > rufren_speech::RECOGNITION_MAX_AUDIO_BYTES
            || format!("{:x}", Sha256::digest(bytes)) != attachment.asset.sha256
        {
            return Err(
                "These bytes do not match the saved recording, or the take is too large.".into(),
            );
        }
        inspect_recognition_wav(bytes).map_err(|e| e.to_string())?;
        let request = RecognitionRequest {
            request_id,
            asset_id: attachment.asset_id.clone(),
            attempt_id: Some(attempt_id.to_string()),
            revision_id: Some(attempt.revision.to_string()),
            language: "ru".into(),
            audio_sha256: attachment.asset.sha256.clone(),
            wav_base64: STANDARD.encode(bytes),
            span: None,
        };
        request.validate().map_err(|e| e.to_string())?;
        Ok(request)
    }

    pub(crate) fn keep_transcription(
        &mut self,
        recording: Uuid,
        expected: &RecognitionRequest,
        response: &RecognitionResponse,
    ) -> Result<Uuid> {
        response.validate_for(expected).map_err(|e| e.to_string())?;
        let bytes = STANDARD
            .decode(&expected.wav_base64)
            .map_err(|_| "The source recording is unreadable.")?;
        self.refresh()?;
        if self.recognition_request(recording, &bytes, expected.request_id.clone())? != *expected {
            return Err("The transcript request no longer matches this exact take.".into());
        }
        let evidence = inspect_recognition_wav(&bytes).map_err(|e| e.to_string())?;
        let original = &response.original_audio;
        if *original != evidence.original_audio
            || response.source_pcm_sha256 != evidence.source_pcm_sha256
        {
            return Err(
                "The transcription source shape differs from your original recording.".into(),
            );
        }
        let attempt = Uuid::parse_str(
            expected
                .attempt_id
                .as_deref()
                .ok_or("Missing speaking attempt")?,
        )
        .map_err(|e| e.to_string())?;
        let revision = Uuid::parse_str(
            expected
                .revision_id
                .as_deref()
                .ok_or("Missing speaking reference")?,
        )
        .map_err(|e| e.to_string())?;
        let p = &response.provenance;
        let fact = Fact::SpeechTranscribed(SpeechTranscription {
            recording,
            attempt,
            revision,
            asset_id: response.asset_id.clone(),
            audio_sha256: response.audio_sha256.clone(),
            span: OriginalFrameSpan {
                start_frame: response.span.start_frame,
                end_frame: response.span.end_frame,
            },
            original_audio: SpeechOriginalAudio {
                sample_rate_hz: original.sample_rate,
                channels: original.channels,
                frames: original.frames,
                encoding: original.encoding.clone(),
                bits: original.bits,
            },
            transcript: response.transcript.clone(),
            language: response.language.clone(),
            provenance: SpeechModelProvenance {
                schema_version: response.schema_version.clone(),
                request_id: response.request_id.clone(),
                provider: p.provider.clone(),
                model: p.model.clone(),
                model_revision: p.revision.clone(),
                runtime: p.runtime.clone(),
                manifest_sha256: p.manifest_sha256.clone(),
                worker_sha256: p.worker_sha256.clone(),
            },
            audit_json: serde_json::to_string(response).map_err(|e| e.to_string())?,
        });
        let id = Uuid::new_v5(&recording, response.request_id.as_bytes());
        self.keep_speech_fact(id, fact)?;
        Ok(id)
    }

    pub(crate) fn confirm_transcription(
        &mut self,
        transcription: Uuid,
        text: String,
    ) -> Result<Uuid> {
        self.refresh()?;
        if text.trim().is_empty() || text.len() > 8192 {
            return Err("Choose a nonempty transcript under 8 KB for this discussion.".into());
        }
        let original = self
            .library
            .speech_transcription(transcription)
            .ok_or("Sync the original transcription first.")?;
        let operation = if text == original.transcript {
            TranscriptOperation::Confirm
        } else {
            TranscriptOperation::Edit
        };
        let active = self.library.transcript_confirmations(transcription);
        if active.len() == 1 && active[0].1.text == text {
            return Ok(active[0].0.id);
        }
        let supersedes = active.iter().map(|(e, _)| e.id).collect();
        let id = Uuid::new_v4();
        self.keep_speech_fact(
            id,
            Fact::TranscriptConfirmed(TranscriptConfirmation {
                transcription,
                text,
                operation,
                supersedes,
            }),
        )?;
        Ok(id)
    }

    pub(crate) fn confirmed_words(&self, confirmation: Uuid, attempt: Uuid) -> Result<String> {
        let c = self
            .library
            .transcript_confirmation(confirmation)
            .ok_or("Sync the confirmed words first.")?;
        let t = self
            .library
            .speech_transcription(c.transcription)
            .ok_or("Sync the original transcription first.")?;
        let a = self
            .library
            .attempt(attempt)
            .ok_or("The speaking response is unavailable.")?;
        let Some(Fact::RecordingAttached(recording)) =
            self.library.events.get(&t.recording).map(|e| &e.fact)
        else {
            return Err("Sync the exact original recording link first.".into());
        };
        if t.attempt != attempt
            || t.revision != a.revision
            || a.skill != Skill::Speaking
            || recording.target != AnnotationTarget::Attempt(attempt)
            || recording.role != AudioRole::Attempt
            || recording.asset_id != t.asset_id
            || recording.asset.sha256 != t.audio_sha256
        {
            return Err("These words belong to a different speaking response.".into());
        }
        Ok(c.text.clone())
    }

    fn keep_speech_fact(&mut self, id: Uuid, fact: Fact) -> Result<()> {
        if let Some(old) = self.library.events.get(&id) {
            if old.fact == fact {
                return Ok(());
            }
            return Err(
                "A repeated transcript changed its contents. The earlier evidence is kept.".into(),
            );
        }
        self.repo.append(&[Event {
            version: EVENT_VERSION,
            id,
            device: self.device,
            recorded_at_ms: now_ms(),
            fact,
        }])?;
        self.refresh()
    }
}
