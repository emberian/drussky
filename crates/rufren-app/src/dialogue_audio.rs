//! Ordinary dialogue voice messages: original → independent hypothesis → adopted words.
use crate::{
    audio_studio::StudioRecording,
    model::AppState,
    repository::{Result, now_ms},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rufren_core::*;
use rufren_speech::{RecognitionRequest, RecognitionResponse, inspect_recognition_wav};
use rufren_tutor::DialogueConfirmedAudioRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DialogueAdoptedSpeech {
    pub text: String,
    pub source: DialogueConfirmedAudioRef,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct DialogueVoiceDraft {
    pub recording: Option<Uuid>,
    pub transcription: Option<Uuid>,
    pub confirmation: Option<Uuid>,
}
fn voice_key(thread: Uuid, input: Uuid) -> String {
    format!("dialogue_audio_draft:{thread}:{input}")
}
impl AppState {
    pub(crate) fn dialogue_voice_draft(
        &self,
        thread: Uuid,
        input: Uuid,
    ) -> Result<DialogueVoiceDraft> {
        self.repo
            .get(&voice_key(thread, input))
            .map(|value| value.unwrap_or_default())
    }
    pub(crate) fn save_dialogue_voice_draft(
        &self,
        thread: Uuid,
        input: Uuid,
        draft: &DialogueVoiceDraft,
    ) -> Result<()> {
        if thread.is_nil() || input.is_nil() {
            return Err("Open a dialogue message before recording.".into());
        }
        if let Some(recording) = draft.recording {
            self.dialogue_input_recording(recording, Some((thread, input)))?;
        }
        if let Some(transcription) = draft.transcription {
            let t = self
                .library
                .dialogue_speech_transcription(transcription)
                .ok_or("The selected transcription is unavailable.")?;
            if t.thread_id != thread || t.input_id != input || Some(t.recording) != draft.recording
            {
                return Err("The selected words belong to a different voice message.".into());
            }
        }
        self.repo.set(&voice_key(thread, input), draft)
    }
    pub(crate) fn register_dialogue_recording(
        &mut self,
        thread: Uuid,
        input: Uuid,
        record: &StudioRecording,
    ) -> Result<Uuid> {
        let metadata: AudioRecordMetadata =
            serde_json::from_value(serde_json::to_value(record).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        metadata.validate().map_err(|e| e.to_string())?;
        let scope = &metadata.device["dialogue_scope"];
        if thread.is_nil()
            || input.is_nil()
            || !metadata.passage_id.is_empty()
            || !metadata.revision_id.is_empty()
            || metadata.role != "attempt"
            || scope["thread_id"].as_str() != Some(thread.to_string().as_str())
            || scope["input_id"].as_str() != Some(input.to_string().as_str())
        {
            return Err("This original does not belong to the current dialogue input.".into());
        }
        self.refresh()?;
        let id = Uuid::new_v5(
            &input,
            format!("dialogue-audio:{}", record.asset_id).as_bytes(),
        );
        if self.library.dialogue_user(input).is_some() && !self.library.events.contains_key(&id) {
            return Err("This message is already sent. Record another message to continue.".into());
        }
        let fact = Fact::DialogueAudioAttached(DialogueAudioAttachment {
            thread_id: thread,
            owner: DialogueAudioOwner::UserInput { input_id: input },
            asset_id: record.asset_id.clone(),
            asset: AudioAsset {
                sha256: record.content_hash.clone(),
                media_type: record.mime_type.clone(),
                sample_rate_hz: record.sample_rate,
                channels: record.channels,
                frames: record.frames,
            },
            provenance_json: None,
        });
        self.keep_dialogue_audio_fact(id, fact, record.created_at_ms)?;
        Ok(id)
    }
    pub(crate) fn dialogue_input_recording(
        &self,
        recording: Uuid,
        scope: Option<(Uuid, Uuid)>,
    ) -> Result<&DialogueAudioAttachment> {
        let Some(Fact::DialogueAudioAttached(record)) =
            self.library.events.get(&recording).map(|e| &e.fact)
        else {
            return Err("Sync the exact dialogue recording before continuing.".into());
        };
        let DialogueAudioOwner::UserInput { input_id } = record.owner else {
            return Err("Choose your own dialogue voice message.".into());
        };
        if scope.is_some_and(|(thread, input)| thread != record.thread_id || input != input_id) {
            return Err("This take belongs to another dialogue input.".into());
        }
        Ok(record)
    }
    pub(crate) fn dialogue_recognition_request(
        &self,
        recording: Uuid,
        bytes: &[u8],
        request_id: String,
    ) -> Result<RecognitionRequest> {
        let record = self.dialogue_input_recording(recording, None)?;
        if bytes.len() > rufren_speech::RECOGNITION_MAX_AUDIO_BYTES
            || format!("{:x}", Sha256::digest(bytes)) != record.asset.sha256
        {
            return Err(
                "These bytes do not match the exact dialogue original, or the take is too large."
                    .into(),
            );
        }
        inspect_recognition_wav(bytes).map_err(|e| e.to_string())?;
        let request = RecognitionRequest {
            request_id,
            asset_id: record.asset_id.clone(),
            attempt_id: None,
            revision_id: None,
            language: "ru".into(),
            audio_sha256: record.asset.sha256.clone(),
            wav_base64: STANDARD.encode(bytes),
            span: None,
        };
        request.validate().map_err(|e| e.to_string())?;
        Ok(request)
    }
    pub(crate) fn keep_dialogue_transcription(
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
        if self.dialogue_recognition_request(recording, &bytes, expected.request_id.clone())?
            != *expected
        {
            return Err("The result belongs to a different dialogue take.".into());
        }
        let inspected = inspect_recognition_wav(&bytes).map_err(|e| e.to_string())?;
        if response.original_audio != inspected.original_audio
            || response.source_pcm_sha256 != inspected.source_pcm_sha256
        {
            return Err(
                "The transcription source shape differs from the exact WAV original.".into(),
            );
        }
        let record = self.dialogue_input_recording(recording, None)?;
        let DialogueAudioOwner::UserInput { input_id } = record.owner else {
            unreachable!()
        };
        let p = &response.provenance;
        let original = &response.original_audio;
        let fact = Fact::DialogueSpeechTranscribed(DialogueSpeechTranscription {
            thread_id: record.thread_id,
            input_id,
            recording,
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
        self.keep_dialogue_audio_fact(id, fact, now_ms())?;
        Ok(id)
    }
    pub(crate) fn confirm_dialogue_transcription(
        &mut self,
        transcription: Uuid,
        text: String,
    ) -> Result<Uuid> {
        self.refresh()?;
        let observation = self
            .library
            .dialogue_speech_transcription(transcription)
            .ok_or("Sync the independent dialogue transcription first.")?;
        if self.library.dialogue_user(observation.input_id).is_some() {
            return Err("These words were already sent. Start another message to continue.".into());
        }
        if text.trim().is_empty() || text.len() > 2048 {
            return Err("Choose a nonempty voice message under 2 KB before sending.".into());
        }
        let heads = self
            .library
            .dialogue_transcript_confirmations(transcription);
        if heads.len() == 1 && heads[0].1.text == text {
            return Ok(heads[0].0.id);
        }
        let fact = Fact::DialogueTranscriptConfirmed(DialogueTranscriptConfirmation {
            thread_id: observation.thread_id,
            input_id: observation.input_id,
            transcription,
            operation: if text == observation.transcript {
                TranscriptOperation::Confirm
            } else {
                TranscriptOperation::Edit
            },
            text,
            supersedes: heads.into_iter().map(|(event, _)| event.id).collect(),
        });
        let id = Uuid::new_v4();
        self.keep_dialogue_audio_fact(id, fact, now_ms())?;
        Ok(id)
    }
    pub(crate) fn dialogue_audio_input_reference(
        &self,
        thread: Uuid,
        input: Uuid,
        confirmation: Uuid,
    ) -> Result<DialogueAdoptedSpeech> {
        let c = self
            .library
            .dialogue_transcript_confirmation(confirmation)
            .ok_or("The adopted dialogue words are unavailable.")?;
        let t = self
            .library
            .dialogue_speech_transcription(c.transcription)
            .ok_or("The original independent transcription is unavailable.")?;
        let source = DialogueConfirmedAudioRef {
            recording: t.recording,
            transcription: c.transcription,
            confirmation,
        };
        let text = self
            .library
            .dialogue_confirmed_input(thread, input, &source)
            .map_err(|e| e.to_string())?;
        Ok(DialogueAdoptedSpeech { text, source })
    }
    pub(crate) fn register_dialogue_generated_audio(
        &mut self,
        reply_id: Uuid,
        chunk_index: usize,
        record: &StudioRecording,
    ) -> Result<Uuid> {
        self.refresh()?;
        let reply = self
            .library
            .dialogue_reply(reply_id)
            .ok_or("Sync the exact dialogue reply before giving it a voice.")?;
        let chunks = teacher_dialogue_reply_chunks(reply);
        let chunk = chunks
            .get(chunk_index)
            .ok_or("That Russian utterance is unavailable.")?;
        let thread = reply.request.thread_id;
        let hash = dialogue_text_sha256(&chunk.russian);
        let scope = &record.device["dialogue_scope"];
        if record.role != "reference"
            || !record.passage_id.is_empty()
            || !record.revision_id.is_empty()
            || scope["thread_id"].as_str() != Some(thread.to_string().as_str())
            || scope["reply_id"].as_str() != Some(reply_id.to_string().as_str())
            || scope["chunk_index"].as_u64() != Some(chunk_index as u64)
        {
            return Err("Generated audio belongs to a different dialogue utterance.".into());
        }
        let fact = Fact::DialogueAudioAttached(DialogueAudioAttachment {
            thread_id: thread,
            owner: DialogueAudioOwner::AssistantChunk {
                reply_id,
                chunk_index,
                source_text_sha256: hash,
            },
            asset_id: record.asset_id.clone(),
            asset: AudioAsset {
                sha256: record.content_hash.clone(),
                media_type: record.mime_type.clone(),
                sample_rate_hz: record.sample_rate,
                channels: record.channels,
                frames: record.frames,
            },
            provenance_json: Some(
                serde_json::to_string(&record.device["provenance"]).map_err(|e| e.to_string())?,
            ),
        });
        let id = Uuid::new_v5(
            &reply_id,
            format!("dialogue-voice:{chunk_index}:{}", record.asset_id).as_bytes(),
        );
        self.keep_dialogue_audio_fact(id, fact, record.created_at_ms)?;
        Ok(id)
    }
    pub(crate) fn heard_dialogue_chunk(
        &mut self,
        reply_id: Uuid,
        chunk_index: usize,
        record: &StudioRecording,
    ) -> Result<()> {
        self.refresh()?;
        let reply = self
            .library
            .dialogue_reply(reply_id)
            .ok_or("This dialogue reply is unavailable.")?;
        let chunks = teacher_dialogue_reply_chunks(reply);
        let chunk = chunks
            .get(chunk_index)
            .ok_or("This Russian utterance is unavailable.")?;
        let owner = DialogueAudioOwner::AssistantChunk {
            reply_id,
            chunk_index,
            source_text_sha256: dialogue_text_sha256(&chunk.russian),
        };
        if !self
            .library
            .dialogue_audio_attachments(&owner)
            .iter()
            .any(|(_, audio)| {
                audio.asset_id == record.asset_id && audio.asset.sha256 == record.content_hash
            })
        {
            return Err("This playback does not match the selected dialogue utterance.".into());
        }
        let event = Event::new(
            self.device,
            now_ms(),
            Fact::StimulusPresented(StimulusPresentation {
                session: None,
                revision: None,
                russian: chunk.russian.clone(),
                purpose: PresentationPurpose::Audio,
                decision: None,
                utterance_family: Some(format!("dialogue:{reply_id}:{chunk_index}")),
            }),
        )
        .map_err(|e| e.to_string())?;
        self.repo.append(&[event])?;
        self.refresh()
    }
    fn keep_dialogue_audio_fact(&mut self, id: Uuid, fact: Fact, time: i64) -> Result<()> {
        if let Some(old) = self.library.events.get(&id) {
            if old.fact == fact {
                return Ok(());
            }
            return Err("A repeated dialogue audio operation changed its contents. The earlier original is kept.".into());
        }
        let event = Event {
            version: EVENT_VERSION,
            id,
            device: self.device,
            recorded_at_ms: time,
            fact,
        };
        self.repo.append(&[event])?;
        self.refresh()
    }
}
