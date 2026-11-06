//! Exact original audio and independently adopted words for ordinary dialogue.
//! These facts never create an attempt, a passage, a grade, or a review seed.
use crate::*;
use rufren_tutor_contract::{DialogueConfirmedAudioRef, TeacherDialogueRequest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DialogueAudioOwner {
    UserInput {
        input_id: Uuid,
    },
    AssistantChunk {
        reply_id: Uuid,
        chunk_index: usize,
        source_text_sha256: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DialogueAudioAttachment {
    pub thread_id: Uuid,
    pub owner: DialogueAudioOwner,
    pub asset_id: String,
    pub asset: AudioAsset,
    /// Complete bounded generated-speech provenance, including exact source text.
    /// User microphone/import originals have no machine-synthesis provenance.
    pub provenance_json: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DialogueSpeechTranscription {
    pub thread_id: Uuid,
    pub input_id: Uuid,
    pub recording: Uuid,
    pub asset_id: String,
    pub audio_sha256: String,
    pub span: OriginalFrameSpan,
    pub original_audio: SpeechOriginalAudio,
    pub transcript: String,
    pub language: String,
    pub provenance: SpeechModelProvenance,
    /// Unmodified independent worker result. Attempt/revision IDs must both be null.
    pub audit_json: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DialogueTranscriptConfirmation {
    pub thread_id: Uuid,
    pub input_id: Uuid,
    pub transcription: Uuid,
    pub text: String,
    pub operation: TranscriptOperation,
    pub supersedes: BTreeSet<Uuid>,
}
fn invalid() -> Error {
    Error::Invalid("dialogue audio differs from its exact thread, input, original, observation or adopted words".into())
}
fn bounded(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max
}
pub fn dialogue_text_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn generated_provenance(a: &DialogueAudioAttachment, hash: &str) -> Result<(), Error> {
    let json = a
        .provenance_json
        .as_ref()
        .filter(|s| s.len() <= 8192)
        .ok_or_else(invalid)?;
    let p: serde_json::Value = serde_json::from_str(json).map_err(|_| invalid())?;
    let source = p["source_text"].as_str().ok_or_else(invalid)?;
    if !matches!(
        p["kind"].as_str(),
        Some("ai_generated_speech" | "system_generated_speech")
    ) || source.chars().count() > 256
        || !bounded(source, 1024)
        || dialogue_text_sha256(source) != hash
        || p["source_text_sha256"].as_str() != Some(hash)
        || ["provider", "model", "voice", "request_id"]
            .iter()
            .any(|k| p[*k].as_str().is_none_or(|v| !bounded(v, 256)))
        || a.asset.sample_rate_hz != 24_000
        || a.asset.channels != 1
        || a.asset.frames > 24_000 * 45
    {
        return Err(invalid());
    }
    Ok(())
}
impl DialogueAudioAttachment {
    pub fn validate(&self) -> Result<(), Error> {
        self.asset.validate()?;
        if self.thread_id.is_nil()
            || !bounded(&self.asset_id, 256)
            || self.asset_id.chars().any(char::is_control)
        {
            return Err(invalid());
        }
        match &self.owner {
            DialogueAudioOwner::UserInput { input_id }
                if !input_id.is_nil() && self.provenance_json.is_none() =>
            {
                Ok(())
            }
            DialogueAudioOwner::AssistantChunk {
                reply_id,
                chunk_index,
                source_text_sha256,
            } if !reply_id.is_nil() && *chunk_index < 2 && valid_audio_hash(source_text_sha256) => {
                generated_provenance(self, source_text_sha256)
            }
            _ => Err(invalid()),
        }
    }
}
impl DialogueSpeechTranscription {
    pub fn validate(&self) -> Result<(), Error> {
        let a = &self.original_audio;
        let p = &self.provenance;
        if self.thread_id.is_nil()
            || self.input_id.is_nil()
            || self.recording.is_nil()
            || !bounded(&self.asset_id, 256)
            || !valid_audio_hash(&self.audio_sha256)
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
            || !valid_audio_hash(&p.manifest_sha256)
            || !valid_audio_hash(&p.worker_sha256)
            || self.audit_json.len() > 65536
        {
            return Err(invalid());
        }
        let v: serde_json::Value = serde_json::from_str(&self.audit_json).map_err(|_| invalid())?;
        let string = |key: &str, value: &str| v[key].as_str() == Some(value);
        let number =
            |object: &serde_json::Value, key: &str, value: u64| object[key].as_u64() == Some(value);
        let original = &v["original_audio"];
        let span = &v["span"];
        let derivative = &v["derivative"];
        let provenance = &v["provenance"];
        let expected_frames = ((self.span.end_frame - self.span.start_frame) * 16000)
            .div_ceil(u64::from(a.sample_rate_hz));
        let matches = string("schema_version", &p.schema_version)
            && string("request_id", &p.request_id)
            && v.get("attempt_id").is_some_and(serde_json::Value::is_null)
            && v.get("revision_id").is_some_and(serde_json::Value::is_null)
            && string("asset_id", &self.asset_id)
            && string("audio_sha256", &self.audio_sha256)
            && string("transcript", &self.transcript)
            && string("language", &self.language)
            && string("span_coordinate_system", "original_frames_half_open")
            && string("evidence_kind", "transcript_hypothesis")
            && string("language_selection", "explicit_not_detected")
            && v["target_conditioned"].as_bool() == Some(false)
            && ["word_timestamps", "confidence", "acoustic_assessment"]
                .iter()
                .all(|key| v.get(*key).is_some_and(serde_json::Value::is_null))
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
            .all(|(key, value)| provenance[*key].as_str() == Some(*value))
            && provenance["mlx_version"]
                .as_str()
                .is_some_and(|s| bounded(s, 64))
            && provenance["decoding"].is_object()
            && v["timing"].is_object()
            && v["usage"].is_object()
            && v["source_pcm_sha256"]
                .as_str()
                .is_some_and(valid_audio_hash)
            && string(
                "source_pcm_hash_format",
                "interleaved-little-endian-f32-no-header-v1",
            )
            && number(derivative, "sample_rate", 16000)
            && number(derivative, "channels", 1)
            && number(derivative, "frames", expected_frames)
            && derivative["pcm_sha256"]
                .as_str()
                .is_some_and(valid_audio_hash)
            && derivative["pcm_hash_format"].as_str() == Some("little-endian-f32-no-header-v1")
            && derivative["method"]
                .as_str()
                .is_some_and(|s| bounded(s, 512))
            && derivative["scipy_version"]
                .as_str()
                .is_some_and(|s| bounded(s, 64))
            && derivative["amplitude_normalization"].as_bool() == Some(false);
        if !matches {
            return Err(invalid());
        }
        Ok(())
    }
}
pub fn validate_dialogue_audio_fact(event: &Event) -> Result<(), Error> {
    match &event.fact {
        Fact::DialogueAudioAttached(a) => a.validate(),
        Fact::DialogueSpeechTranscribed(t) => {
            t.validate()?;
            if event.id != Uuid::new_v5(&t.recording, t.provenance.request_id.as_bytes()) {
                return Err(invalid());
            }
            Ok(())
        }
        Fact::DialogueTranscriptConfirmed(c) => {
            if c.thread_id.is_nil()
                || c.input_id.is_nil()
                || c.transcription.is_nil()
                || !bounded(&c.text, 8192)
                || c.supersedes.len() > 64
                || c.supersedes.contains(&event.id)
                || c.supersedes.iter().any(Uuid::is_nil)
            {
                Err(invalid())
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}
pub(crate) fn validate_dialogue_audio_identities(
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    let mut assets = BTreeMap::<&str, &AudioAsset>::new();
    for event in events.values() {
        let pair = match &event.fact {
            Fact::DialogueAudioAttached(a) => Some((a.asset_id.as_str(), &a.asset)),
            Fact::RecordingAttached(a) => Some((a.asset_id.as_str(), &a.asset)),
            _ => None,
        };
        if let Some((id, asset)) = pair {
            if assets.insert(id, asset).is_some_and(|old| old != asset) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
pub fn validate_dialogue_audio_references(
    event: &Event,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    match &event.fact {
        Fact::DialogueAudioAttached(a) => match &a.owner {
            DialogueAudioOwner::UserInput { input_id } => {
                if let Some(request) = crate::teacher_dialogue::dialogue_user_in(events, *input_id)
                {
                    if request.thread_id.to_string() != a.thread_id.to_string() {
                        return Err(invalid());
                    }
                }
            }
            DialogueAudioOwner::AssistantChunk {
                reply_id,
                chunk_index,
                source_text_sha256,
            } => {
                if let Some(reply) = crate::teacher_dialogue::dialogue_reply_in(events, *reply_id) {
                    let chunks = teacher_dialogue_reply_chunks(reply);
                    let chunk = chunks.get(*chunk_index).ok_or_else(invalid)?;
                    if reply.request.thread_id.to_string() != a.thread_id.to_string()
                        || dialogue_text_sha256(&chunk.russian) != *source_text_sha256
                    {
                        return Err(invalid());
                    }
                }
            }
        },
        Fact::DialogueSpeechTranscribed(t) => {
            if let Some(record) = events.get(&t.recording) {
                match &record.fact {
                    Fact::DialogueAudioAttached(a)
                        if a.thread_id == t.thread_id
                            && a.owner
                                == (DialogueAudioOwner::UserInput {
                                    input_id: t.input_id,
                                })
                            && a.asset_id == t.asset_id
                            && a.asset.sha256 == t.audio_sha256 => {}
                    _ => return Err(invalid()),
                }
            }
        }
        Fact::DialogueTranscriptConfirmed(c) => {
            if let Some(transcription) = events.get(&c.transcription) {
                match &transcription.fact {
                    Fact::DialogueSpeechTranscribed(t)
                        if t.thread_id == c.thread_id
                            && t.input_id == c.input_id
                            && (match c.operation {
                                TranscriptOperation::Confirm => c.text == t.transcript,
                                TranscriptOperation::Edit => c.text != t.transcript,
                            }) => {}
                    _ => return Err(invalid()),
                }
            }
            let mut pending: Vec<_> = c.supersedes.iter().copied().collect();
            let mut visited = BTreeSet::new();
            while let Some(parent) = pending.pop() {
                if parent == event.id {
                    return Err(invalid());
                }
                if !visited.insert(parent) {
                    continue;
                }
                if let Some(parent) = events.get(&parent) {
                    match &parent.fact {
                        Fact::DialogueTranscriptConfirmed(old)
                            if old.thread_id == c.thread_id
                                && old.input_id == c.input_id
                                && old.transcription == c.transcription =>
                        {
                            pending.extend(old.supersedes.iter().copied())
                        }
                        _ => return Err(invalid()),
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
/// Merge permits absent parents; every available reference must still have the right type.
pub fn validate_dialogue_input_source(
    request: &TeacherDialogueRequest,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    let Some(source) = &request.confirmed_audio else {
        return Ok(());
    };
    let thread = Uuid::parse_str(&request.thread_id.to_string()).map_err(|_| invalid())?;
    let input = Uuid::parse_str(&request.request_id.to_string()).map_err(|_| invalid())?;
    if let Some(record) = events.get(&source.recording) {
        match &record.fact {
            Fact::DialogueAudioAttached(a)
                if a.thread_id == thread
                    && a.owner == (DialogueAudioOwner::UserInput { input_id: input }) => {}
            _ => return Err(invalid()),
        }
    }
    if let Some(t) = events.get(&source.transcription) {
        match &t.fact {
            Fact::DialogueSpeechTranscribed(t)
                if t.thread_id == thread
                    && t.input_id == input
                    && t.recording == source.recording => {}
            _ => return Err(invalid()),
        }
    }
    if let Some(c) = events.get(&source.confirmation) {
        match &c.fact {
            Fact::DialogueTranscriptConfirmed(c)
                if c.thread_id == thread
                    && c.input_id == input
                    && c.transcription == source.transcription
                    && c.text == request.text => {}
            _ => return Err(invalid()),
        }
    }
    Ok(())
}
impl Library {
    pub fn dialogue_audio_attachments(
        &self,
        owner: &DialogueAudioOwner,
    ) -> Vec<(&Event, &DialogueAudioAttachment)> {
        self.events
            .values()
            .filter_map(|event| match &event.fact {
                Fact::DialogueAudioAttached(a) if &a.owner == owner => Some((event, a)),
                _ => None,
            })
            .collect()
    }
    pub fn dialogue_speech_transcription(&self, id: Uuid) -> Option<&DialogueSpeechTranscription> {
        match &self.events.get(&id)?.fact {
            Fact::DialogueSpeechTranscribed(t) => Some(t),
            _ => None,
        }
    }
    pub fn dialogue_speech_transcriptions(
        &self,
        input: Uuid,
    ) -> Vec<(&Event, &DialogueSpeechTranscription)> {
        self.events
            .values()
            .filter_map(|event| match &event.fact {
                Fact::DialogueSpeechTranscribed(t) if t.input_id == input => Some((event, t)),
                _ => None,
            })
            .collect()
    }
    pub fn dialogue_transcript_confirmation(
        &self,
        id: Uuid,
    ) -> Option<&DialogueTranscriptConfirmation> {
        match &self.events.get(&id)?.fact {
            Fact::DialogueTranscriptConfirmed(c) => Some(c),
            _ => None,
        }
    }
    pub fn dialogue_transcript_confirmations(
        &self,
        transcription: Uuid,
    ) -> Vec<(&Event, &DialogueTranscriptConfirmation)> {
        let all: Vec<_> = self
            .events
            .values()
            .filter_map(|event| match &event.fact {
                Fact::DialogueTranscriptConfirmed(c) if c.transcription == transcription => {
                    Some((event, c))
                }
                _ => None,
            })
            .collect();
        let superseded: BTreeSet<_> = all
            .iter()
            .flat_map(|(_, c)| c.supersedes.iter().copied())
            .collect();
        all.into_iter()
            .filter(|(event, _)| !superseded.contains(&event.id))
            .collect()
    }
    /// Send requires the complete selected source chain, not just a missing-parent merge.
    pub fn dialogue_confirmed_input(
        &self,
        thread: Uuid,
        input: Uuid,
        source: &DialogueConfirmedAudioRef,
    ) -> Result<String, Error> {
        let c = self
            .dialogue_transcript_confirmation(source.confirmation)
            .ok_or_else(invalid)?;
        let t = self
            .dialogue_speech_transcription(source.transcription)
            .ok_or_else(invalid)?;
        let Some(Fact::DialogueAudioAttached(a)) =
            self.events.get(&source.recording).map(|event| &event.fact)
        else {
            return Err(invalid());
        };
        let heads = self.dialogue_transcript_confirmations(source.transcription);
        if heads.len() != 1
            || heads[0].0.id != source.confirmation
            || c.thread_id != thread
            || c.input_id != input
            || c.transcription != source.transcription
            || t.thread_id != thread
            || t.input_id != input
            || t.recording != source.recording
            || a.thread_id != thread
            || a.owner != (DialogueAudioOwner::UserInput { input_id: input })
            || a.asset_id != t.asset_id
            || a.asset.sha256 != t.audio_sha256
        {
            return Err(invalid());
        }
        Ok(c.text.clone())
    }
}
