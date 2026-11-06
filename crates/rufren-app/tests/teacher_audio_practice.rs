//! Exact oral exercise callbacks and durable bindings; public synthetic metadata/audio only.
#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code)]
#[path = "../src/drafts.rs"]
mod drafts;
#[path = "../src/editor_drafts.rs"]
mod editor_drafts;
#[path = "../src/exposure.rs"]
mod exposure;
#[path = "../src/learning.rs"]
mod learning;
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
#[path = "../src/teacher_notebook.rs"]
mod teacher_notebook;
#[path = "../src/teacher_practice.rs"]
mod teacher_practice;
use model::AppState;
use repository::Repository;
use rufren_core::*;
use rufren_tutor::*;
use sha2::{Digest, Sha256};
use std::rc::Rc;
use teacher_audio_practice::{TEACHER_AUDIO_POLICY, TeacherAudioBinding};
use teacher_practice::TeacherPracticeDraft;
use uuid::Uuid;
fn app() -> AppState {
    let mut s =
        AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap();
    let id = s.content.lessons[0].id.clone();
    s.choose_lesson(&id).unwrap();
    s
}
fn lesson(state: &AppState, skill: TeacherSkill) -> TeacherResponse {
    let listening = skill == TeacherSkill::Listening;
    let mut request = state.lesson_teaching_request().unwrap();
    if let TeacherRequest::TeachLesson(request) = &mut request {
        request.lesson_flow = None;
    }
    TeacherResponse {
        request,
        provenance: TutorProvenance {
            provider: "synthetic".into(),
            model: "teacher-test".into(),
            review_status: ReviewStatus::AiGeneratedTeaching,
            scope: "language content".into(),
            prompt_version: "test-v4".into(),
            response_id: if listening {
                "listening-one"
            } else {
                "speaking-one"
            }
            .into(),
        },
        usage: TutorUsage {
            accounting: UsageAccounting::SubscriptionQuota,
            input_tokens: 0,
            output_tokens: 0,
            estimated_micro_usd: 0,
            reserved_micro_usd: 0,
        },
        content: TeacherContent {
            lesson_flow: None,
            explanation: "A short greeting can carry a complete message.".into(),
            examples: vec![TutorExample {
                russian: "Привет.".into(),
                english: "Hi.".into(),
                note: "Informal greeting".into(),
            }],
            next_step: "Try this small exchange.".into(),
            caveats: vec![],
            feedback: None,
            practice: vec![TeacherPractice {
                prompt: if listening {
                    "Listen and explain the greeting in English."
                } else {
                    "Say hi informally in Russian."
                }
                .into(),
                expected_response: Some(if listening { "Hi." } else { "Привет." }.into()),
                teaching_note: "Accept suitable alternatives.".into(),
                reference_russian: Some("Привет.".into()),
                reference_english: Some("Hi.".into()),
                skill: Some(skill),
            }],
        },
    }
}
fn prepared(
    skill: TeacherSkill,
) -> (
    AppState,
    TeacherResponse,
    TeacherPracticeDraft,
    TeacherAudioBinding,
) {
    let mut s = app();
    let answer = lesson(&s, skill);
    let mut draft = s.teacher_practice_draft(&answer, 0).unwrap();
    let binding = s
        .prepare_teacher_audio_practice(&answer, &mut draft)
        .unwrap();
    (s, answer, draft, binding)
}
fn attach(
    state: &mut AppState,
    binding: &TeacherAudioBinding,
    role: &str,
    interrupted: bool,
) -> (Uuid, AudioRecordMetadata, Vec<u8>) {
    let pcm = rufren_audio::PcmAudio::from_mono(16000, vec![0.125; 16000]).unwrap();
    let bytes = pcm.to_wav();
    let metadata = AudioRecordMetadata {
        asset_id: format!("synthetic-{}", Uuid::new_v4()),
        content_hash: format!("{:x}", Sha256::digest(&bytes)),
        role: role.into(),
        name: "Synthetic take".into(),
        passage_id: binding.passage_id.to_string(),
        revision_id: binding.revision_id.to_string(),
        duration_ms: 1000,
        sample_rate: 16000,
        channels: 1,
        frames: 16000,
        byte_length: bytes.len() as u64,
        created_at_ms: repository::now_ms(),
        mime_type: "audio/wav".into(),
        interrupted,
        device: serde_json::json!({}),
        annotations: vec![],
    };
    metadata.validate().unwrap();
    let event = Event::new(
        state.device,
        metadata.created_at_ms,
        Fact::RecordingAttached(AudioAttachment {
            asset_id: metadata.asset_id.clone(),
            asset: AudioAsset {
                sha256: metadata.content_hash.clone(),
                media_type: metadata.mime_type.clone(),
                sample_rate_hz: metadata.sample_rate,
                channels: metadata.channels,
                frames: metadata.frames,
            },
            target: AnnotationTarget::Revision(binding.revision_id),
            role: if role == "reference" {
                AudioRole::Reference
            } else {
                AudioRole::Attempt
            },
            note: if interrupted {
                "Recording interrupted; captured portion preserved".into()
            } else {
                String::new()
            },
        }),
    )
    .unwrap();
    let id = event.id;
    state.repo.append(&[event]).unwrap();
    state.refresh().unwrap();
    (id, metadata, bytes)
}

#[test]
fn teacher_audio_practice_listening_requires_exact_successful_input_and_persists_replays() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Listening);
    assert!(s.library.attempts().next().is_none());
    assert_eq!(s.library.presentations().count(), 0);
    s.teacher_practice_edit_response(&answer, &mut draft, "Hi.".into())
        .unwrap();
    assert!(s.submit_teacher_practice(&answer, &mut draft).is_err());
    let (id, record, _) = attach(&mut s, &binding, "reference", false);
    let mut wrong = record.clone();
    wrong.content_hash = "f".repeat(64);
    assert!(
        s.teacher_audio_reference_played(&answer, &mut draft, id, &wrong)
            .is_err()
    );
    assert_eq!(draft.audio.as_ref().unwrap().reference_plays, 0);
    s.teacher_audio_reference_played(&answer, &mut draft, id, &record)
        .unwrap();
    assert_eq!(draft.audio.as_ref().unwrap().reference_plays, 1);
    assert!(!draft.audio.as_ref().unwrap().cues.contains(&Cue::Replay));
    let repo = s.repo.clone();
    drop(s);
    let mut s = AppState::from_repository(repo).unwrap();
    draft = s.teacher_practice_draft(&answer, 0).unwrap();
    assert_eq!(
        draft
            .audio
            .as_ref()
            .unwrap()
            .played_reference
            .as_ref()
            .unwrap()
            .attachment,
        id
    );
    s.teacher_audio_reference_played(&answer, &mut draft, id, &record)
        .unwrap();
    assert_eq!(draft.audio.as_ref().unwrap().reference_plays, 2);
    assert!(draft.audio.as_ref().unwrap().cues.contains(&Cue::Replay));
    let attempt = s.submit_teacher_practice(&answer, &mut draft).unwrap();
    let a = s.library.attempt(attempt).unwrap();
    assert_eq!(a.skill, Skill::Listening);
    assert_eq!(a.policy, TEACHER_AUDIO_POLICY);
    assert_eq!(a.self_rating, SelfRating::NotReported);
    assert_eq!(a.response, "Hi.");
    assert!(a.cues.contains(&Cue::Replay));
    let context = s.library.contexts(attempt)[0];
    assert!(teacher_audio_practice::valid_submitted_audio(
        &s.library, attempt, context.1
    ));
    assert!(
        teacher_practice::saved_teacher_practice_origin(&s.library, attempt, context.0.id)
            .is_some()
    );
    // A separately imported context stays unresolved. Retrying this already saved response
    // validates its exact deterministic source context, never the lowest-ID competing head.
    let mut competing = context.1.clone();
    competing.prompt = Some("A separately preserved competing question".into());
    competing.reference_answer =
        Some("A different question description, without this input proof".into());
    let mut competing_event = Event::new(
        s.device,
        repository::now_ms(),
        Fact::AttemptContextRecorded(competing),
    )
    .unwrap();
    competing_event.id = Uuid::from_u128(1);
    s.repo.append(&[competing_event]).unwrap();
    s.refresh().unwrap();
    let before = s.library.clone();
    assert_eq!(
        s.submit_teacher_practice(&answer, &mut draft).unwrap(),
        attempt
    );
    assert_eq!(s.library, before);
    assert!(
        s.answer_feedback_request(attempt).is_err(),
        "saving the original response does not resolve competing task descriptions"
    );
}

#[test]
fn teacher_audio_practice_reveals_and_answer_edits_fork_without_faking_a_new_play() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Listening);
    let (id, record, _) = attach(&mut s, &binding, "reference", false);
    s.teacher_audio_reference_played(&answer, &mut draft, id, &record)
        .unwrap();
    s.teacher_practice_edit_response(&answer, &mut draft, "Hi.".into())
        .unwrap();
    let original = s.submit_teacher_practice(&answer, &mut draft).unwrap();
    let old_session = draft.id;
    let play = draft
        .audio
        .as_ref()
        .unwrap()
        .played_reference
        .as_ref()
        .unwrap()
        .presentation;
    let presentations = s.library.presentations().count();
    s.teacher_audio_reveal(&answer, &mut draft, Cue::Translation)
        .unwrap();
    assert_ne!(draft.id, old_session);
    assert!(draft.saved_attempt.is_none());
    assert_eq!(
        draft
            .audio
            .as_ref()
            .unwrap()
            .played_reference
            .as_ref()
            .unwrap()
            .presentation,
        play
    );
    assert_eq!(s.library.presentations().count(), presentations);
    s.teacher_practice_edit_response(&answer, &mut draft, "An informal hello.".into())
        .unwrap();
    let edited = s.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert_ne!(edited, original);
    assert_eq!(s.library.attempt(original).unwrap().response, "Hi.");
    assert!(
        !s.library
            .attempt(original)
            .unwrap()
            .cues
            .contains(&Cue::Translation)
    );
    assert!(
        s.library
            .attempt(edited)
            .unwrap()
            .cues
            .contains(&Cue::Translation)
    );
    assert!(teacher_audio_practice::valid_submitted_audio(
        &s.library,
        edited,
        s.library.contexts(edited)[0].1
    ));
}

#[test]
fn teacher_audio_practice_speaking_copies_exact_selected_take_and_opens_existing_asr_path() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Speaking);
    assert!(s.submit_teacher_practice(&answer, &mut draft).is_err());
    assert!(
        s.teacher_practice_edit_response(&answer, &mut draft, "Привет.".into())
            .is_err()
    );
    let (id, record, bytes) = attach(&mut s, &binding, "attempt", false);
    s.teacher_audio_select_recording(&answer, &mut draft, id, &record, false)
        .unwrap();
    assert_eq!(draft.selected_asset_id(), Some(record.asset_id.as_str()));
    let attempt = s.submit_teacher_practice(&answer, &mut draft).unwrap();
    let a = s.library.attempt(attempt).unwrap();
    assert_eq!(a.policy, TEACHER_AUDIO_POLICY);
    assert_eq!(a.self_rating, SelfRating::NotReported);
    assert!(a.response.contains(&record.asset_id));
    assert!(!a.response.contains("Привет"));
    let links = s.library.recordings(AnnotationTarget::Attempt(attempt));
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].1.asset_id, record.asset_id);
    assert_eq!(links[0].1.asset.sha256, record.content_hash);
    let asr = s
        .recognition_request(links[0].0.id, &bytes, "oral-transcription".into())
        .unwrap();
    assert_eq!(asr.attempt_id, Some(attempt.to_string()));
    assert_eq!(asr.revision_id, Some(binding.revision_id.to_string()));
    assert_eq!(asr.audio_sha256, record.content_hash);
    assert!(
        s.answer_feedback_request(attempt).is_err(),
        "a recording label cannot bypass ASR and learner adoption"
    );
    let review = SpacedReview.project(
        &s.library,
        ReviewKey {
            revision: binding.revision_id,
            skill: Skill::Speaking,
        },
        repository::now_ms(),
    );
    assert_eq!(review.interval_ms, 0);
    assert_eq!(review.coach_confirmed_unassisted, 0);
    assert!(
        teacher_practice::saved_teacher_practice_origin(
            &s.library,
            attempt,
            s.library.contexts(attempt)[0].0.id
        )
        .is_some()
    );
}

#[test]
fn teacher_audio_practice_clear_new_capture_and_selected_partial_never_reuse_old_attempt() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Speaking);
    let (first, record, _) = attach(&mut s, &binding, "attempt", false);
    s.teacher_audio_select_recording(&answer, &mut draft, first, &record, false)
        .unwrap();
    let old = s.submit_teacher_practice(&answer, &mut draft).unwrap();
    let session = draft.id;
    s.teacher_audio_clear_recording(&answer, &mut draft)
        .unwrap();
    assert_ne!(draft.id, session);
    assert!(draft.selected_attachment().is_none());
    assert!(draft.response.is_empty());
    assert!(s.submit_teacher_practice(&answer, &mut draft).is_err());
    let (partial, record, _) = attach(&mut s, &binding, "attempt", true);
    assert!(
        s.teacher_audio_select_recording(&answer, &mut draft, partial, &record, false)
            .is_err()
    );
    assert!(draft.selected_attachment().is_none());
    s.teacher_audio_select_recording(&answer, &mut draft, partial, &record, true)
        .unwrap();
    let new = s.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert_ne!(new, old);
    assert!(
        s.library
            .attempt(new)
            .unwrap()
            .response
            .contains("interrupted")
    );
    assert_eq!(
        s.library.recordings(AnnotationTarget::Attempt(old))[0]
            .1
            .asset_id,
        s.library
            .events
            .get(&first)
            .and_then(|e| if let Fact::RecordingAttached(a) = &e.fact {
                Some(a.asset_id.clone())
            } else {
                None
            })
            .unwrap()
    );
}

#[test]
fn teacher_audio_practice_rejects_wrong_question_roles_and_changed_source_binding() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Speaking);
    let (reference, metadata, _) = attach(&mut s, &binding, "reference", false);
    assert!(
        s.teacher_audio_select_recording(&answer, &mut draft, reference, &metadata, false)
            .is_err()
    );
    let (attempt, metadata, _) = attach(&mut s, &binding, "attempt", false);
    assert!(
        s.teacher_audio_reference_played(&answer, &mut draft, attempt, &metadata)
            .is_err()
    );
    let mut changed = answer.clone();
    changed.content.practice[0].prompt =
        "A changed question under the same response identity".into();
    assert!(
        s.prepare_teacher_audio_practice(&changed, &mut draft)
            .is_err()
    );
    assert_eq!(draft.audio.as_ref().unwrap().binding, binding);
    let mut foreign = answer;
    foreign.provenance.response_id = "another-response".into();
    assert!(
        s.prepare_teacher_audio_practice(&foreign, &mut draft)
            .is_err()
    );
}

#[test]
fn teacher_audio_practice_speaking_model_replay_is_support_and_legacy_text_draft_stays_decodable() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Speaking);
    let (id, record, _) = attach(&mut s, &binding, "reference", false);
    s.teacher_audio_reveal(&answer, &mut draft, Cue::OriginalText)
        .unwrap();
    s.teacher_audio_reference_played(&answer, &mut draft, id, &record)
        .unwrap();
    assert!(draft.audio.as_ref().unwrap().cues.contains(&Cue::Replay));
    assert!(
        draft
            .audio
            .as_ref()
            .unwrap()
            .cues
            .contains(&Cue::OriginalText)
    );
    let mut json = serde_json::to_value(&draft).unwrap();
    json.as_object_mut().unwrap().remove("audio");
    let restored: TeacherPracticeDraft = serde_json::from_value(json).unwrap();
    assert!(restored.audio.is_none());
}

#[test]
fn teacher_audio_practice_corrupt_missing_support_cue_fails_before_any_attempt_fact() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Listening);
    let (id, record, _) = attach(&mut s, &binding, "reference", false);
    s.teacher_audio_reference_played(&answer, &mut draft, id, &record)
        .unwrap();
    s.teacher_practice_edit_response(&answer, &mut draft, "Hi.".into())
        .unwrap();
    let before = s.library.clone();
    draft.audio.as_mut().unwrap().cues.clear();
    assert!(s.submit_teacher_practice(&answer, &mut draft).is_err());
    assert_eq!(s.library, before);
    assert!(s.library.attempts().next().is_none());
}

#[test]
fn teacher_audio_practice_corrupt_listening_take_selection_fails_before_any_attempt_fact() {
    let (mut s, answer, mut draft, binding) = prepared(TeacherSkill::Listening);
    let (id, record, _) = attach(&mut s, &binding, "reference", false);
    s.teacher_audio_reference_played(&answer, &mut draft, id, &record)
        .unwrap();
    s.teacher_practice_edit_response(&answer, &mut draft, "Hi.".into())
        .unwrap();
    let (id, record, _) = attach(&mut s, &binding, "attempt", false);
    let before = s.library.clone();
    draft.audio.as_mut().unwrap().selected_recording =
        Some(teacher_audio_practice::TeacherSelectedRecording {
            attachment: id,
            asset_id: record.asset_id,
            sha256: record.content_hash,
            interrupted: false,
        });
    assert!(s.submit_teacher_practice(&answer, &mut draft).is_err());
    assert_eq!(s.library, before);
    assert!(s.library.attempts().next().is_none());
}
