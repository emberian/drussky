//! Exact oral exercise callbacks and durable bindings; public synthetic metadata/audio only.
#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code)]
#[path = "../src/daily_plan.rs"]
mod daily_plan;
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
#[path = "../src/reader.rs"]
mod reader;
#[path = "../src/recognition.rs"]
mod recognition;
#[path = "../src/repository.rs"]
mod repository;
#[path = "../src/review_timing.rs"]
mod review_timing;
#[path = "../src/sound_focus.rs"]
mod sound_focus;
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

#[test]
fn listening_feedback_repair_clears_old_audio_input_and_remains_supported() {
    let (mut state, teacher, mut first, binding) = prepared(TeacherSkill::Listening);
    state
        .keep_teacher_response(&teacher.request, &teacher)
        .unwrap();
    let (attachment, metadata, _) = attach(&mut state, &binding, "reference", false);
    state
        .teacher_audio_reference_played(&teacher, &mut first, attachment, &metadata)
        .unwrap();
    state
        .teacher_practice_edit_response(&teacher, &mut first, "Hi.".into())
        .unwrap();
    let attempt = state.submit_teacher_practice(&teacher, &mut first).unwrap();
    let request = state.answer_feedback_request(attempt).unwrap();
    let critique = TeacherResponse {
        request,
        provenance: teacher.provenance.clone(),
        usage: teacher.usage.clone(),
        content: TeacherContent {
            lesson_flow: None,
            explanation: "That greeting works.".into(),
            examples: vec![],
            next_step: "Listen again, then say what the greeting means.".into(),
            caveats: vec![],
            practice: vec![],
            feedback: Some(AiAnswerFeedback {
                verdict: AiVerdict::Correct,
                suggested_response: None,
                corrections: vec![],
            }),
        },
    };
    let (source, index, session) = state.start_teacher_feedback_repair(&critique).unwrap();
    let mut repair = state.teacher_practice_draft(&source, index).unwrap();
    assert_ne!(session, first.id);
    assert!(repair.response.is_empty());
    assert!(repair.audio.is_none());
    state
        .prepare_teacher_audio_practice(&source, &mut repair)
        .unwrap();
    let audio = repair.audio.as_ref().unwrap();
    assert_eq!(audio.reference_plays, 0);
    assert!(audio.played_reference.is_none());
    assert!(audio.selected_recording.is_none());
    assert_eq!(audio.cues, vec![Cue::ReferenceMaterial]);
    assert!(state.submit_teacher_practice(&source, &mut repair).is_err());
    assert_eq!(state.library.attempt(attempt).unwrap().response, "Hi.");
}

fn attach(
    state: &mut AppState,
    binding: &TeacherAudioBinding,
    role: &str,
    interrupted: bool,
) -> (Uuid, AudioRecordMetadata, Vec<u8>) {
    attach_level(state, binding, role, interrupted, 0.125)
}
fn attach_level(
    state: &mut AppState,
    binding: &TeacherAudioBinding,
    role: &str,
    interrupted: bool,
    level: f32,
) -> (Uuid, AudioRecordMetadata, Vec<u8>) {
    let pcm = rufren_audio::PcmAudio::from_mono(16000, vec![level; 16000]).unwrap();
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
    let saved_before = s.teacher_practice_draft(&answer, 0).unwrap();
    draft.audio.as_mut().unwrap().cues.clear();
    assert!(s.save_teacher_practice_draft(&draft).is_err());
    assert_eq!(
        s.teacher_practice_draft(&answer, 0).unwrap().audio,
        saved_before.audio
    );
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

fn oral_revisit(
    skill: TeacherSkill,
) -> (
    AppState,
    TeacherResponse,
    TeacherPracticeDraft,
    TeacherPracticeDraft,
    TeacherAudioBinding,
) {
    let (mut state, answer, mut guided, binding) = prepared(skill);
    state
        .keep_teacher_response(&answer.request, &answer)
        .unwrap();
    if binding.skill == Skill::Listening {
        let (id, record, _) = attach(&mut state, &binding, "reference", false);
        state
            .teacher_audio_reference_played(&answer, &mut guided, id, &record)
            .unwrap();
        state
            .teacher_practice_edit_response(&answer, &mut guided, "Hi.".into())
            .unwrap();
    } else {
        let (id, record, _) = attach(&mut state, &binding, "attempt", false);
        state
            .teacher_audio_select_recording(&answer, &mut guided, id, &record, false)
            .unwrap();
    }
    state.submit_teacher_practice(&answer, &mut guided).unwrap();
    // Separate completed input from the new review boundary at millisecond resolution.
    std::thread::sleep(std::time::Duration::from_millis(2));
    let key = TeacherRevisitKey {
        document_id: teacher_document_id(&answer),
        practice_index: 0,
    };
    let mut revisit = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    assert_ne!(revisit.id, guided.id);
    assert!(revisit.response.is_empty());
    assert!(revisit.audio.is_none());
    state
        .prepare_teacher_audio_practice(&answer, &mut revisit)
        .unwrap();
    assert!(revisit.audio.as_ref().unwrap().cues.is_empty());
    (state, answer, guided, revisit, binding)
}

#[test]
fn teacher_audio_revisit_listening_needs_current_session_task_input_without_default_help() {
    let (mut state, answer, guided, mut revisit, binding) = oral_revisit(TeacherSkill::Listening);
    state
        .teacher_practice_edit_response(&answer, &mut revisit, "Hi again.".into())
        .unwrap();
    assert!(
        state
            .submit_teacher_practice(&answer, &mut revisit)
            .is_err()
    );
    assert_eq!(state.library.attempts().count(), 1);
    let (id, record, _) = attach(&mut state, &binding, "reference", false);
    state
        .teacher_audio_reference_played(&answer, &mut revisit, id, &record)
        .unwrap();
    let audio = revisit.audio.as_ref().unwrap().clone();
    let played = audio.played_reference.as_ref().unwrap();
    let Fact::StimulusPresented(presentation) = &state.library.events[&played.presentation].fact
    else {
        panic!()
    };
    let presentation = presentation.clone();
    assert_eq!(presentation.session, Some(revisit.id));
    assert_eq!(presentation.purpose, PresentationPurpose::TaskInput);
    assert!(audio.cues.is_empty());
    assert!(revisit.review.as_ref().unwrap().cues.is_empty());
    let attempt = state
        .submit_teacher_practice(&answer, &mut revisit)
        .unwrap();
    let actual = state.library.attempt(attempt).unwrap();
    assert_eq!(actual.policy, teacher_revisit::TEACHER_REVISIT_POLICY);
    assert!(actual.cues.is_empty());
    assert_eq!(actual.self_rating, SelfRating::NotReported);
    let (context_event, context) = state.library.contexts(attempt)[0];
    assert!(
        teacher_revisit::saved_teacher_revisit_origin(&state.library, attempt, context_event.id)
            .is_some()
    );
    assert!(teacher_audio_practice::valid_submitted_audio(
        &state.library,
        attempt,
        context
    ));
    // Same input text, purpose, family and asset, but another session is not proof
    // that this review actually provided its required listening input.
    let mut forged_presentation = presentation.clone();
    forged_presentation.session = Some(guided.id);
    let forged = Event::new(
        state.device,
        repository::now_ms(),
        Fact::StimulusPresented(forged_presentation),
    )
    .unwrap();
    let forged_id = forged.id;
    let mut library = state.library.clone();
    library.merge(&[forged]).unwrap();
    let mut forged_context = context.clone();
    let mut forged_audio = audio.clone();
    forged_audio.played_reference.as_mut().unwrap().presentation = forged_id;
    let base = teacher_audio_practice::audio_reference_base(context).unwrap();
    forged_context.reference_answer = Some(
        teacher_audio_practice::audio_reference_answer(
            base,
            context.prompt.as_deref().unwrap(),
            &forged_audio,
        )
        .unwrap(),
    );
    assert!(!teacher_audio_practice::valid_submitted_audio(
        &library,
        attempt,
        &forged_context
    ));
    let saved_id = revisit.id;
    assert!(
        state
            .teacher_audio_reference_played(&answer, &mut revisit, id, &record)
            .is_err()
    );
    assert_eq!(revisit.id, saved_id);
    assert!(
        state
            .teacher_practice_edit_response(&answer, &mut revisit, "Changed after saving".into())
            .is_err()
    );
}

#[test]
fn teacher_audio_revisit_replay_reveal_and_optional_report_are_explicit_help() {
    let (mut state, answer, _, mut revisit, binding) = oral_revisit(TeacherSkill::Listening);
    let (id, record, _) = attach(&mut state, &binding, "reference", false);
    state
        .teacher_audio_reference_played(&answer, &mut revisit, id, &record)
        .unwrap();
    state
        .teacher_audio_reference_played(&answer, &mut revisit, id, &record)
        .unwrap();
    state
        .teacher_audio_reveal(&answer, &mut revisit, Cue::OriginalText)
        .unwrap();
    state
        .teacher_audio_reveal(&answer, &mut revisit, Cue::Translation)
        .unwrap();
    state
        .set_teacher_revisit_rating(&mut revisit, SelfRating::WithHelp)
        .unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut revisit, "Hi.".into())
        .unwrap();
    let attempt = state
        .submit_teacher_practice(&answer, &mut revisit)
        .unwrap();
    let actual = state.library.attempt(attempt).unwrap();
    assert_eq!(
        actual.cues,
        [Cue::Replay, Cue::OriginalText, Cue::Translation]
    );
    assert_eq!(actual.self_rating, SelfRating::WithHelp);
    assert!(!actual.cues.contains(&Cue::ReferenceMaterial));
    assert_eq!(
        state
            .teacher_revisit_draft(&answer, 0)
            .unwrap()
            .saved_attempt,
        Some(attempt)
    );
    let context = state.library.contexts(attempt)[0];
    assert!(
        teacher_revisit::saved_teacher_revisit_origin(&state.library, attempt, context.0.id)
            .is_some()
    );
}

#[test]
fn teacher_audio_revisit_speaking_rejects_old_bytes_and_resumes_exact_new_take() {
    let (mut state, answer, guided, mut revisit, binding) = oral_revisit(TeacherSkill::Speaking);
    assert!(revisit.selected_asset_id().is_none());
    let (old, record, _) = attach(&mut state, &binding, "attempt", false);
    assert!(
        state
            .teacher_audio_select_recording(&answer, &mut revisit, old, &record, false)
            .is_err()
    );
    assert!(revisit.selected_asset_id().is_none());
    assert!(revisit.response.is_empty());
    let (fresh, record, bytes) = attach_level(&mut state, &binding, "attempt", false, 0.25);
    state
        .teacher_audio_select_recording(&answer, &mut revisit, fresh, &record, false)
        .unwrap();
    let session = revisit.id;
    let repository = state.repo.clone();
    drop(state);
    let mut state = AppState::from_repository(repository).unwrap();
    let mut revisit = state.teacher_revisit_draft(&answer, 0).unwrap();
    assert_eq!(revisit.id, session);
    assert_eq!(revisit.selected_asset_id(), Some(record.asset_id.as_str()));
    state
        .teacher_audio_select_recording(&answer, &mut revisit, fresh, &record, true)
        .unwrap();
    let attempt = state
        .submit_teacher_practice(&answer, &mut revisit)
        .unwrap();
    assert_ne!(Some(attempt), guided.saved_attempt);
    let current = state.library.attempt(attempt).unwrap();
    assert_eq!(current.cues, [Cue::ReferenceMaterial]);
    assert_eq!(current.self_rating, SelfRating::NotReported);
    let link = state.library.recordings(AnnotationTarget::Attempt(attempt))[0];
    assert_eq!(link.1.asset_id, record.asset_id);
    assert_eq!(link.1.asset.sha256, record.content_hash);
    let asr = state
        .recognition_request(link.0.id, &bytes, "review-transcription".into())
        .unwrap();
    assert_eq!(asr.attempt_id, Some(attempt.to_string()));
    assert!(
        state.answer_feedback_request(attempt).is_err(),
        "exact reviewed words still require ASR and learner confirmation"
    );
    let context = state.library.contexts(attempt)[0];
    assert!(
        teacher_revisit::saved_teacher_revisit_origin(&state.library, attempt, context.0.id)
            .is_some()
    );
}

#[test]
fn teacher_audio_revisit_fresh_speaking_take_has_no_inherited_reference_help() {
    let (mut state, answer, _, mut revisit, binding) = oral_revisit(TeacherSkill::Speaking);
    let (id, record, _) = attach_level(&mut state, &binding, "attempt", false, 0.375);
    state
        .teacher_audio_select_recording(&answer, &mut revisit, id, &record, false)
        .unwrap();
    let attempt = state
        .submit_teacher_practice(&answer, &mut revisit)
        .unwrap();
    assert!(state.library.attempt(attempt).unwrap().cues.is_empty());
    assert_eq!(
        state.library.attempt(attempt).unwrap().policy,
        teacher_revisit::TEACHER_REVISIT_POLICY
    );
}

#[test]
fn teacher_audio_revisit_durable_replay_survives_actual_metadata_write_failure() {
    let (source, answer, _, mut revisit, binding) = oral_revisit(TeacherSkill::Listening);
    let folder =
        std::env::temp_dir().join(format!("rufren-replay-write-failure-{}", Uuid::new_v4()));
    std::fs::create_dir(&folder).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let database = folder.join("library.sqlite");
    let repo = Rc::new(Repository::open_at(&database).unwrap());
    repo.append(&source.library.events.into_values().collect::<Vec<_>>())
        .unwrap();
    let key = teacher_revisit::teacher_revisit_storage_key(&revisit).unwrap();
    repo.set(&key, &revisit).unwrap();
    let mut state = AppState::from_repository(repo).unwrap();
    state.select_teacher_revisit(&revisit).unwrap();
    let (attachment, record, _) = attach(&mut state, &binding, "reference", false);
    state
        .teacher_audio_reference_played(&answer, &mut revisit, attachment, &record)
        .unwrap();
    assert_eq!(revisit.audio.as_ref().unwrap().reference_plays, 1);
    assert!(revisit.audio.as_ref().unwrap().cues.is_empty());
    // The prepare write (plays=1) succeeds. The subsequent Audio event commits, then
    // the callback's metadata write (plays=2) fails inside the actual SQLite store.
    let sql = rusqlite::Connection::open(&database).unwrap();
    sql.execute_batch("CREATE TRIGGER fail_second_reference_play BEFORE INSERT ON metadata
        WHEN NEW.key LIKE 'teacher_revisit:%' AND json_extract(NEW.json,'$.audio.reference_plays') >= 2
        BEGIN SELECT RAISE(ABORT,'synthetic interrupted replay metadata write'); END;").unwrap();
    assert!(
        state
            .teacher_audio_reference_played(&answer, &mut revisit, attachment, &record)
            .is_err()
    );
    let saved = state.teacher_revisit_draft(&answer, 0).unwrap();
    assert_eq!(saved.audio.as_ref().unwrap().reference_plays, 1);
    assert!(saved.audio.as_ref().unwrap().cues.is_empty());
    state.refresh().unwrap();
    let replay_sources: Vec<_> = state
        .library
        .presentations()
        .filter(|(_, p)| p.session == Some(revisit.id) && p.purpose == PresentationPurpose::Audio)
        .map(|(e, _)| e.id)
        .collect();
    assert_eq!(
        replay_sources.len(),
        1,
        "Audio acknowledgment survives failed metadata save"
    );
    // Resume from the durable old draft, not the callback's uncommitted local fields.
    let mut revisit = saved;
    state
        .teacher_practice_edit_response(&answer, &mut revisit, "Hi.".into())
        .unwrap();
    let attempt = state
        .submit_teacher_practice(&answer, &mut revisit)
        .unwrap();
    let actual = state.library.attempt(attempt).unwrap();
    assert_eq!(actual.cues, [Cue::Replay]);
    assert_eq!(actual.self_rating, SelfRating::NotReported);
    assert!(!actual.cues.contains(&Cue::ReferenceMaterial));
    assert!(
        state
            .teacher_revisit_draft(&answer, 0)
            .unwrap()
            .review
            .unwrap()
            .cues
            .contains(&Cue::Replay)
    );
    let context = state.library.contexts(attempt)[0];
    assert!(teacher_audio_practice::valid_submitted_audio(
        &state.library,
        attempt,
        context.1
    ));
    assert!(
        teacher_revisit::saved_teacher_revisit_origin(&state.library, attempt, context.0.id)
            .is_some()
    );
    sql.execute_batch("DROP TRIGGER fail_second_reference_play;")
        .unwrap();
    drop(sql);
    drop(state);
    std::fs::remove_dir_all(folder).unwrap();
}
