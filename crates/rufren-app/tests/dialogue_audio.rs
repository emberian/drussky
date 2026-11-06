//! Exact oral exercise callbacks and durable bindings; public synthetic metadata/audio only.
#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code)]
#[path = "../src/dialogue_audio.rs"]
mod dialogue_audio;
#[path = "../src/drafts.rs"]
mod drafts;
#[path = "../src/editor_drafts.rs"]
mod editor_drafts;
#[path = "../src/exposure.rs"]
mod exposure;
#[path = "../src/learning.rs"]
mod learning;
#[path = "../src/daily_plan.rs"]
mod daily_plan;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/native_connection.rs"]
mod native_connection;
#[path = "../src/native_http.rs"]
mod native_http;
#[path = "../src/recognition.rs"]
mod recognition;
#[path = "../src/repository.rs"]
mod repository;
#[path = "../src/teacher.rs"]
mod teacher;
#[path = "../src/teacher_audio_practice.rs"]
mod teacher_audio_practice;
#[path = "../src/teacher_context.rs"]
mod teacher_context;
#[path = "../src/teacher_dialogue.rs"]
mod teacher_dialogue;
#[path = "../src/teacher_notebook.rs"]
mod teacher_notebook;
#[path = "../src/teacher_practice.rs"]
mod teacher_practice;
#[path = "../src/teacher_revisit.rs"]
mod teacher_revisit;
mod audio_studio {
    pub type StudioRecording = rufren_core::AudioRecordMetadata;
}
use model::AppState;
use repository::Repository;
use rufren_core::*;
use rufren_speech::*;
use rufren_tutor::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::rc::Rc;
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn new_message(s: &AppState) -> teacher_dialogue::DialogueDraft {
    s.new_dialogue(
        DialogueConfig {
            intent: DialogueIntent::Clarification,
            topic: "A public greeting".into(),
            teacher_role: None,
            learner_role: None,
        },
        DialogueAction::Ask,
        None,
    )
    .unwrap()
}
fn original(thread: Uuid, input: Uuid) -> (AudioRecordMetadata, Vec<u8>) {
    let pcm = rufren_audio::PcmAudio::from_mono(16000, vec![0.125; 1600]).unwrap();
    let bytes = pcm.to_wav();
    (
        AudioRecordMetadata {
            asset_id: format!("dialogue-{}", Uuid::new_v4()),
            content_hash: format!("{:x}", Sha256::digest(&bytes)),
            role: "attempt".into(),
            name: "Synthetic WAV".into(),
            passage_id: String::new(),
            revision_id: String::new(),
            duration_ms: 100,
            sample_rate: 16000,
            channels: 1,
            frames: 1600,
            byte_length: bytes.len() as u64,
            created_at_ms: repository::now_ms(),
            mime_type: "audio/wav".into(),
            interrupted: false,
            device: json!({"dialogue_scope":{"thread_id":thread,"input_id":input}}),
            annotations: vec![],
        },
        bytes,
    )
}
fn hypothesis(request: &RecognitionRequest, bytes: &[u8]) -> RecognitionResponse {
    let inspected = inspect_recognition_wav(bytes).unwrap();
    let frames = inspected.original_audio.frames;
    serde_json::from_value(json!({"schema_version":RECOGNITION_SCHEMA,"request_id":request.request_id,"attempt_id":null,"revision_id":null,"asset_id":request.asset_id,"audio_sha256":request.audio_sha256,"original_audio":inspected.original_audio,"source_pcm_sha256":inspected.source_pcm_sha256,"source_pcm_hash_format":"interleaved-little-endian-f32-no-header-v1","span":{"start_frame":0,"end_frame":frames},"span_coordinate_system":"original_frames_half_open","derivative":{"sample_rate":16000,"channels":1,"frames":frames,"pcm_sha256":"c".repeat(64),"pcm_hash_format":"little-endian-f32-no-header-v1","method":"arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5","scipy_version":"1.17.0","amplitude_normalization":false},"evidence_kind":"transcript_hypothesis","transcript":"Привет!","language":"ru","language_selection":"explicit_not_detected","target_conditioned":false,"word_timestamps":null,"word_alignment_status":"not_run","confidence":null,"acoustic_assessment":null,"provenance":{"provider":"local_qwen3_asr","model":RECOGNITION_MODEL,"revision":RECOGNITION_REVISION,"runtime":RECOGNITION_RUNTIME,"mlx_version":"0.30.0","manifest_sha256":"d".repeat(64),"worker_sha256":"e".repeat(64),"decoding":{"temperature":0.0,"max_tokens":1024,"seed":42,"language":"Russian","hotwords":[]}},"timing":{"started_at":"2026-11-05T00:00:00Z","completed_at":"2026-11-05T00:00:01Z","elapsed_seconds":1.0,"process_peak_rss_bytes":1000000,"metal_peak_bytes":1000000},"usage":{"prompt_tokens":100,"generation_tokens":8}})).unwrap()
}
#[test]
fn dialogue_audio_original_asr_adoption_and_send_do_not_make_graded_practice() {
    let mut s = app();
    let mut draft = new_message(&s);
    let counts = (s.library.attempts().count(), s.library.heads().len());
    let (record, bytes) = original(draft.thread_id, draft.id);
    let attachment = s
        .register_dialogue_recording(draft.thread_id, draft.id, &record)
        .unwrap();
    let request = s
        .dialogue_recognition_request(attachment, &bytes, "asr-one".into())
        .unwrap();
    assert_eq!(request.attempt_id, None);
    assert_eq!(request.revision_id, None);
    let observation = s
        .keep_dialogue_transcription(attachment, &request, &hypothesis(&request, &bytes))
        .unwrap();
    let confirmation = s
        .confirm_dialogue_transcription(observation, "Здравствуйте!".into())
        .unwrap();
    let adopted = s
        .dialogue_audio_input_reference(draft.thread_id, draft.id, confirmation)
        .unwrap();
    assert_eq!(adopted.text, "Здравствуйте!");
    s.adopt_dialogue_audio(&mut draft, adopted.text, adopted.source.clone())
        .unwrap();
    let id = s.prepare_dialogue_turn(&mut draft).unwrap();
    let saved = s.library.dialogue_user(id).unwrap();
    assert_eq!(saved.text, "Здравствуйте!");
    assert_eq!(saved.confirmed_audio, Some(adopted.source));
    assert_eq!(
        (s.library.attempts().count(), s.library.heads().len()),
        counts
    );
    assert!(
        s.library
            .learner_snapshot(repository::now_ms())
            .features
            .is_empty()
    );
}
#[test]
fn dialogue_audio_rejects_foreign_inputs_wrong_bytes_and_target_conditioned_results() {
    let mut s = app();
    let a = new_message(&s);
    let b = new_message(&s);
    let (record, bytes) = original(a.thread_id, a.id);
    assert!(
        s.register_dialogue_recording(b.thread_id, b.id, &record)
            .is_err()
    );
    let attachment = s
        .register_dialogue_recording(a.thread_id, a.id, &record)
        .unwrap();
    let mut wrong = bytes.clone();
    wrong[0] ^= 1;
    assert!(
        s.dialogue_recognition_request(attachment, &wrong, "wrong".into())
            .is_err()
    );
    let request = s
        .dialogue_recognition_request(attachment, &bytes, "asr-one".into())
        .unwrap();
    let mut response = hypothesis(&request, &bytes);
    response.target_conditioned = true;
    assert!(
        s.keep_dialogue_transcription(attachment, &request, &response)
            .is_err()
    );
    let observation = s
        .keep_dialogue_transcription(attachment, &request, &hypothesis(&request, &bytes))
        .unwrap();
    let confirmation = s
        .confirm_dialogue_transcription(observation, "Привет!".into())
        .unwrap();
    assert!(
        s.dialogue_audio_input_reference(b.thread_id, b.id, confirmation)
            .is_err()
    );
}
#[test]
fn dialogue_audio_confirmed_words_are_immutable_after_send_and_originals_resume_exactly() {
    let mut s = app();
    let mut draft = new_message(&s);
    let (record, bytes) = original(draft.thread_id, draft.id);
    let attachment = s
        .register_dialogue_recording(draft.thread_id, draft.id, &record)
        .unwrap();
    s.save_dialogue_voice_draft(
        draft.thread_id,
        draft.id,
        &dialogue_audio::DialogueVoiceDraft {
            recording: Some(attachment),
            transcription: None,
            confirmation: None,
        },
    )
    .unwrap();
    let repo = s.repo.clone();
    drop(s);
    let mut s = AppState::from_repository(repo).unwrap();
    assert_eq!(
        s.dialogue_voice_draft(draft.thread_id, draft.id)
            .unwrap()
            .recording,
        Some(attachment)
    );
    let request = s
        .dialogue_recognition_request(attachment, &bytes, "asr-one".into())
        .unwrap();
    let observation = s
        .keep_dialogue_transcription(attachment, &request, &hypothesis(&request, &bytes))
        .unwrap();
    let confirmation = s
        .confirm_dialogue_transcription(observation, "Привет!".into())
        .unwrap();
    let adopted = s
        .dialogue_audio_input_reference(draft.thread_id, draft.id, confirmation)
        .unwrap();
    s.adopt_dialogue_audio(&mut draft, adopted.text, adopted.source)
        .unwrap();
    let id = s.prepare_dialogue_turn(&mut draft).unwrap();
    assert!(
        s.confirm_dialogue_transcription(observation, "Другие слова".into())
            .is_err()
    );
    assert_eq!(s.library.dialogue_user(id).unwrap().text, "Привет!");
    assert_eq!(s.library.dialogue_speech_transcriptions(draft.id).len(), 1);
    assert!(s.library.attempts().next().is_none());
}
#[test]
fn dialogue_audio_exact_ai_chunk_registration_and_playback_are_not_download_exposure() {
    let mut s = app();
    let mut draft = new_message(&s);
    s.save_dialogue_draft(&mut draft, "Say hi".into()).unwrap();
    let input = s.prepare_dialogue_turn(&mut draft).unwrap();
    let request = s.library.dialogue_user(input).unwrap().clone();
    let response:TeacherDialogueResponse=serde_json::from_value(json!({"request":request,"content":{"explanation_english":"An informal greeting.","utterance":{"russian":"Привет!","english":"Hi!","note":"To a friend."},"next_question":null,"correction":null,"scene_state":"open"},"provenance":{"provider":"fixture","model":"fixture","response_id":"spoken-reply","review_status":"ai_generated_teaching","prompt_version":"v1","scope":"Synthetic text"},"usage":{"input_tokens":0,"output_tokens":0,"estimated_micro_usd":0,"reserved_micro_usd":0}})).unwrap();
    let reply = teacher_dialogue_reply_id(&response);
    s.repo
        .append(&[Event::new(
            s.device,
            repository::now_ms(),
            Fact::TeacherDialogueReplyRecorded(response),
        )
        .unwrap()])
        .unwrap();
    s.refresh().unwrap();
    let hash = dialogue_text_sha256("Привет!");
    let mut record = original(draft.thread_id, draft.id).0;
    record.role = "reference".into();
    record.sample_rate = 24000;
    record.frames = 2400;
    record.device = json!({"dialogue_scope":{"thread_id":draft.thread_id,"reply_id":reply,"chunk_index":0},"provenance":{"kind":"system_generated_speech","provider":"fixture","model":"fixture","voice":"fixture","request_id":"voice-one","source_text":"Привет!","source_text_sha256":hash}});
    let before = s.library.presentations().count();
    s.register_dialogue_generated_audio(reply, 0, &record)
        .unwrap();
    assert_eq!(s.library.presentations().count(), before);
    s.heard_dialogue_chunk(reply, 0, &record).unwrap();
    assert_eq!(s.library.presentations().count(), before + 1);
    assert!(s.heard_dialogue_chunk(reply, 1, &record).is_err());
    let (_, presentation) = s.library.presentations().last().unwrap();
    assert_eq!(presentation.session, None);
    assert_eq!(presentation.revision, None);
    assert_eq!(presentation.purpose, PresentationPurpose::Audio);
    assert!(s.library.attempts().next().is_none());
}
