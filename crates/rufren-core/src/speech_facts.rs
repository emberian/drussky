//! Independent transcription hypotheses and explicit learner adoption. No acoustic grades.
use crate::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OriginalFrameSpan {
    pub start_frame: u64,
    pub end_frame: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechOriginalAudio {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub frames: u64,
    pub encoding: String,
    pub bits: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechModelProvenance {
    pub schema_version: String,
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub model_revision: String,
    pub runtime: String,
    pub manifest_sha256: String,
    pub worker_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechTranscription {
    /// Exact immutable attempt attachment, not merely another link with the same asset ID.
    pub recording: Uuid,
    pub attempt: Uuid,
    pub revision: Uuid,
    pub asset_id: String,
    pub audio_sha256: String,
    pub span: OriginalFrameSpan,
    pub original_audio: SpeechOriginalAudio,
    /// Verbatim ASR hypothesis. Empty recognition output remains a valid observation.
    pub transcript: String,
    pub language: String,
    pub provenance: SpeechModelProvenance,
    /// Full bounded worker response, retaining derivation/decoding/timing/usage details.
    pub audit_json: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptOperation {
    Confirm,
    Edit,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TranscriptConfirmation {
    pub transcription: Uuid,
    pub text: String,
    pub operation: TranscriptOperation,
    pub supersedes: BTreeSet<Uuid>,
}
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn bounded(value: &str, n: usize) -> bool {
    !value.trim().is_empty() && value.len() <= n
}
impl SpeechTranscription {
    pub fn validate(&self) -> Result<(), Error> {
        let p = &self.provenance;
        let a = &self.original_audio;
        if self.recording.is_nil()
            || self.attempt.is_nil()
            || self.revision.is_nil()
            || !bounded(&self.asset_id, 256)
            || !sha(&self.audio_sha256)
            || self.language != "ru"
            || self.transcript.len() > 16384
            || !(8000..=48000).contains(&a.sample_rate_hz)
            || !(1..=2).contains(&a.channels)
            || a.frames == 0
            || a.frames > u64::from(a.sample_rate_hz) * 45
            || !matches!(
                (a.encoding.as_str(), a.bits),
                ("pcm_s16le", 16) | ("ieee_f32le", 32)
            )
            || self.span.start_frame >= self.span.end_frame
            || self.span.end_frame > a.frames
            || p.schema_version != "rufren-speech-transcript-v1"
            || !bounded(&p.request_id, 128)
            || !bounded(&p.provider, 128)
            || !bounded(&p.model, 512)
            || !bounded(&p.model_revision, 256)
            || !bounded(&p.runtime, 256)
            || !sha(&p.manifest_sha256)
            || !sha(&p.worker_sha256)
            || self.audit_json.len() > 65536
        {
            return Err(Error::Invalid(
                "invalid speech transcription identity, source shape, or provenance".into(),
            ));
        }
        let audit: serde_json::Value = serde_json::from_str(&self.audit_json)
            .map_err(|_| Error::Invalid("invalid transcription audit JSON".into()))?;
        let string = |key: &str, value: &str| {
            audit.get(key).and_then(serde_json::Value::as_str) == Some(value)
        };
        let number = |object: &serde_json::Value, key: &str, value: u64| {
            object.get(key).and_then(serde_json::Value::as_u64) == Some(value)
        };
        let audit_p = &audit["provenance"];
        let original = &audit["original_audio"];
        let span = &audit["span"];
        let derivative = &audit["derivative"];
        let derivative_frames = ((self.span.end_frame - self.span.start_frame) * 16000)
            .div_ceil(u64::from(a.sample_rate_hz));
        let matches = string("schema_version", &p.schema_version)
            && string("request_id", &p.request_id)
            && string("attempt_id", &self.attempt.to_string())
            && string("revision_id", &self.revision.to_string())
            && string("asset_id", &self.asset_id)
            && string("audio_sha256", &self.audio_sha256)
            && string("transcript", &self.transcript)
            && string("language", &self.language)
            && string("span_coordinate_system", "original_frames_half_open")
            && string("evidence_kind", "transcript_hypothesis")
            && string("language_selection", "explicit_not_detected")
            && audit
                .get("target_conditioned")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && ["word_timestamps", "confidence", "acoustic_assessment"]
                .iter()
                .all(|key| audit.get(*key).is_some_and(serde_json::Value::is_null))
            && string("word_alignment_status", "not_run")
            && number(original, "sample_rate", u64::from(a.sample_rate_hz))
            && number(original, "channels", u64::from(a.channels))
            && number(original, "frames", a.frames)
            && number(original, "bits", u64::from(a.bits))
            && original["encoding"].as_str() == Some(a.encoding.as_str())
            && number(span, "start_frame", self.span.start_frame)
            && number(span, "end_frame", self.span.end_frame)
            && [
                ("provider", p.provider.as_str()),
                ("model", p.model.as_str()),
                ("revision", p.model_revision.as_str()),
                ("runtime", p.runtime.as_str()),
                ("manifest_sha256", p.manifest_sha256.as_str()),
                ("worker_sha256", p.worker_sha256.as_str()),
            ]
            .iter()
            .all(|(key, value)| audit_p[*key].as_str() == Some(*value))
            && audit_p["mlx_version"]
                .as_str()
                .is_some_and(|s| bounded(s, 64))
            && audit_p["decoding"].is_object()
            && audit["timing"].is_object()
            && audit["usage"].is_object()
            && audit["source_pcm_sha256"].as_str().is_some_and(sha)
            && string(
                "source_pcm_hash_format",
                "interleaved-little-endian-f32-no-header-v1",
            )
            && number(derivative, "sample_rate", 16000)
            && number(derivative, "channels", 1)
            && number(derivative, "frames", derivative_frames)
            && derivative["pcm_sha256"].as_str().is_some_and(sha)
            && derivative["pcm_hash_format"].as_str() == Some("little-endian-f32-no-header-v1")
            && derivative["method"]
                .as_str()
                .is_some_and(|s| bounded(s, 512))
            && derivative["scipy_version"]
                .as_str()
                .is_some_and(|s| bounded(s, 64))
            && derivative["amplitude_normalization"].as_bool() == Some(false);
        if !matches {
            return Err(Error::Invalid(
                "typed transcription differs from its full worker audit".into(),
            ));
        }
        Ok(())
    }
}
impl TranscriptConfirmation {
    pub fn validate(&self) -> Result<(), Error> {
        if self.transcription.is_nil()
            || !bounded(&self.text, 16384)
            || self.supersedes.len() > 64
            || self.supersedes.iter().any(Uuid::is_nil)
        {
            return Err(Error::Invalid("invalid transcript confirmation".into()));
        }
        Ok(())
    }
}
pub(crate) fn validate_speech_references(
    fact: &Fact,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    let invalid = || {
        Error::Invalid("transcription/confirmation does not match its exact recording, attempt, source or span".into())
    };
    match fact {
        Fact::SpeechTranscribed(t) => {
            if let Some(recording) = events.get(&t.recording) {
                match &recording.fact {
                    Fact::RecordingAttached(a)
                        if a.target == AnnotationTarget::Attempt(t.attempt)
                            && a.role == AudioRole::Attempt
                            && a.asset_id == t.asset_id
                            && a.asset.sha256 == t.audio_sha256 => {}
                    _ => return Err(invalid()),
                }
            }
            if let Some(attempt) = events.get(&t.attempt) {
                match &attempt.fact {
                    Fact::AttemptRecorded(a)
                        if a.revision == t.revision && a.skill == Skill::Speaking => {}
                    _ => return Err(invalid()),
                }
            }
            if events
                .get(&t.revision)
                .is_some_and(|e| !matches!(e.fact, Fact::PassageRevised(_)))
            {
                return Err(invalid());
            }
        }
        Fact::TranscriptConfirmed(c) => {
            if let Some(transcription) = events.get(&c.transcription) {
                match &transcription.fact {
                    Fact::SpeechTranscribed(t)
                        if match c.operation {
                            TranscriptOperation::Confirm => c.text == t.transcript,
                            TranscriptOperation::Edit => c.text != t.transcript,
                        } => {}
                    _ => return Err(invalid()),
                }
            }
            for parent in &c.supersedes {
                if events.get(parent).is_some_and(|e|!matches!(&e.fact,Fact::TranscriptConfirmed(old) if old.transcription==c.transcription)) {return Err(invalid());}
            }
        }
        _ => {}
    }
    Ok(())
}
impl Library {
    pub fn speech_transcription(&self, id: Uuid) -> Option<&SpeechTranscription> {
        match &self.events.get(&id)?.fact {
            Fact::SpeechTranscribed(t) => Some(t),
            _ => None,
        }
    }
    pub fn speech_transcriptions(&self, attempt: Uuid) -> Vec<(&Event, &SpeechTranscription)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::SpeechTranscribed(t) if t.attempt == attempt => Some((e, t)),
                _ => None,
            })
            .collect()
    }
    pub fn transcript_confirmation(&self, id: Uuid) -> Option<&TranscriptConfirmation> {
        match &self.events.get(&id)?.fact {
            Fact::TranscriptConfirmed(c) => Some(c),
            _ => None,
        }
    }
    pub fn transcript_confirmations(
        &self,
        transcription: Uuid,
    ) -> Vec<(&Event, &TranscriptConfirmation)> {
        let superseded: BTreeSet<_> = self
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::TranscriptConfirmed(c) => Some(c.supersedes.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::TranscriptConfirmed(c)
                    if c.transcription == transcription && !superseded.contains(&e.id) =>
                {
                    Some((e, c))
                }
                _ => None,
            })
            .collect()
    }
}
