//! Durable notebook migration, sync/export, and exact lesson recovery. No provider calls.
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
#[path = "../src/daily_plan.rs"]
mod daily_plan;
#[path = "../src/sound_focus.rs"]
mod sound_focus;
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
#[path = "../src/teacher_revisit.rs"]
mod teacher_revisit;
use model::AppState;
use repository::Repository;
use rufren_core::*;
use rufren_tutor::*;
use std::rc::Rc;
use uuid::Uuid;

fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn seeded() -> (
    AppState,
    TeacherResponse,
    teacher_practice::TeacherPracticeDraft,
    TeacherRevisitKey,
) {
    let mut source = app();
    let mut answer: TeacherResponse = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/teacher-notebook-response.json"
    ))
    .unwrap();
    let TeacherRequest::TeachLesson(request) = &mut answer.request else {
        unreachable!()
    };
    request.lesson_id = source.content.lessons[0].id.clone();
    source
        .keep_teacher_response(&answer.request, &answer)
        .unwrap();
    assert!(
        source
            .teacher_revisits(repository::now_ms())
            .entries
            .is_empty(),
        "archive/navigation never seed a revisit"
    );
    let mut guided = source.teacher_practice_draft(&answer, 0).unwrap();
    guided.response = "Я читаю книгу.".into();
    source
        .submit_teacher_practice(&answer, &mut guided)
        .unwrap();
    let key = TeacherRevisitKey {
        document_id: teacher_document_id(&answer),
        practice_index: 0,
    };
    // A new isolated replica receives genuine app-produced facts with clearly synthetic
    // prior occurrence times; no existing immutable row is rewritten.
    let mut facts = source.library.clone();
    for event in facts.events.values_mut() {
        if matches!(
            event.fact,
            Fact::AttemptRecorded(_) | Fact::AttemptContextRecorded(_)
        ) {
            event.recorded_at_ms = repository::now_ms() - 2 * DAY_MS;
        }
    }
    let mut state = app();
    state
        .repo
        .append(&facts.events.into_values().collect::<Vec<_>>())
        .unwrap();
    state.refresh().unwrap();
    state.save_teacher_practice_draft(&guided).unwrap();
    (state, answer, guided, key)
}
fn feedback(request: &TeacherRequest) -> TeacherResponse {
    serde_json::from_value(serde_json::json!({"request":request,
      "content":{"explanation":"Synthetic exact-task language feedback.","examples":[],"next_step":"Try this task again later.","caveats":[],"practice":[],"feedback":{"verdict":"correct","suggested_response":null,"corrections":[]}},
      "provenance":{"provider":"synthetic","model":"no-model","response_id":"revisit-feedback","prompt_version":"revisit-test-v1","scope":"Language content only.","review_status":"ai_generated_teaching"},
      "usage":{"input_tokens":0,"output_tokens":0,"estimated_micro_usd":0,"reserved_micro_usd":0}
    })).unwrap()
}

#[test]
fn revisit_fresh_empty_session_neutral_seed_and_unrated_return_keep_guided_slot_immutable() {
    let (mut state, answer, guided, key) = seeded();
    let queue = state.teacher_revisits(repository::now_ms());
    assert_eq!(queue.entries.len(), 1);
    assert!(queue.entries[0].due);
    assert_eq!(
        queue.entries[0].state.last_evidence,
        TeacherRevisitEvidence::GuidedSeed
    );
    assert_eq!(queue.entries[0].state.interval_ms, DAY_MS);
    let mut draft = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    assert_ne!(draft.id, guided.id);
    assert!(draft.response.is_empty() && draft.audio.is_none());
    assert!(draft.review.as_ref().unwrap().cues.is_empty());
    assert_eq!(
        draft.review.as_ref().unwrap().self_rating,
        SelfRating::NotReported
    );
    assert_eq!(
        state
            .start_teacher_revisit(key, repository::now_ms())
            .unwrap()
            .id,
        draft.id
    );
    state
        .teacher_practice_edit_response(&answer, &mut draft, guided.response.clone())
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert_ne!(attempt, guided.saved_attempt.unwrap());
    let saved = state.library.attempt(attempt).unwrap();
    assert_eq!(saved.self_rating, SelfRating::NotReported);
    assert!(saved.cues.is_empty());
    assert_eq!(saved.policy, TEACHER_REVISIT_POLICY_ID);
    let context = state.library.contexts(attempt)[0];
    let (_, proof) = teacher_revisit::teacher_revisit_proof(context.1).unwrap();
    assert_eq!(proof.source_attempt_id, guided.saved_attempt.unwrap());
    assert_eq!(proof.document_id, key.document_id);
    assert_eq!(
        context.1.exercise_id.as_deref(),
        Some(format!("teacher:revisit:v1:{}:0", key.document_id).as_str())
    );
    let before = state.library.clone();
    assert_eq!(
        state.submit_teacher_practice(&answer, &mut draft).unwrap(),
        attempt
    );
    assert_eq!(state.library, before);
    assert_eq!(
        state.teacher_practice_draft(&answer, 0).unwrap().id,
        guided.id
    );
    let queue = state.teacher_revisits(repository::now_ms());
    assert_eq!(queue.entries[0].state.unrated_returns, 1);
    assert_eq!(queue.entries[0].state.rung, 0);
    assert!(
        state
            .teacher_practice_edit_response(&answer, &mut draft, "Changed after save".into())
            .is_err()
    );
    assert!(
        state
            .set_teacher_revisit_rating(&mut draft, SelfRating::Recalled)
            .is_err()
    );
}

#[test]
fn revisit_self_report_persists_before_critique_syncs_due_and_rejects_stale_replaced_session() {
    let (mut state, answer, _, key) = seeded();
    let mut draft = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    state
        .set_teacher_revisit_rating(&mut draft, SelfRating::Recalled)
        .unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut draft, "Я читаю книгу.".into())
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert_eq!(
        state.library.attempt(attempt).unwrap().self_rating,
        SelfRating::Recalled
    );
    assert_eq!(
        state.teacher_revisits(repository::now_ms()).entries[0]
            .state
            .rung,
        1
    );
    let request = state.answer_feedback_request(attempt).unwrap();
    let response = feedback(&request);
    state.keep_teacher_response(&request, &response).unwrap();
    assert_eq!(state.teacher_revisit_for_feedback(&response), Some(key));
    assert_eq!(
        state.teacher_revisit_feedback(attempt),
        Some(response.clone())
    );
    assert_eq!(
        state.teacher_lesson_for_feedback(&response),
        Some((key.document_id, 0))
    );
    assert!(
        state
            .lesson_practice_summary()
            .unwrap()
            .observations
            .iter()
            .any(|o| o.attempt_id == attempt)
    );
    let now = repository::now_ms();
    let expected = state.teacher_revisits(now).entries[0].state.clone();
    let text = repository::export_library(state.library.clone()).unwrap();
    let mut remote = app();
    remote
        .repo
        .append(
            &repository::import_library(&text)
                .unwrap()
                .events
                .into_values()
                .collect::<Vec<_>>(),
        )
        .unwrap();
    remote.refresh().unwrap();
    assert_eq!(remote.teacher_revisits(now).entries[0].state, expected);
    assert!(remote.active_teacher_revisit().unwrap().is_none());
    assert_eq!(remote.teacher_revisit_feedback(attempt), Some(response));
    let fresh = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    assert_ne!(fresh.id, draft.id);
    assert!(fresh.response.is_empty() && fresh.audio.is_none());
    assert_eq!(
        fresh.review.as_ref().unwrap().self_rating,
        SelfRating::NotReported
    );
    assert!(fresh.review.as_ref().unwrap().cues.is_empty());
    assert!(
        state.save_teacher_practice_draft(&draft).is_err(),
        "old UI callback cannot overwrite replacement"
    );
}

#[test]
fn revisit_out_of_band_support_survives_stale_ui_save_and_cross_window_exposure() {
    let (mut state, answer, _, key) = seeded();
    let mut stale = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    state
        .teacher_revisit_mark_active_support(Cue::Translation)
        .unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut stale, "Я читаю книгу.".into())
        .unwrap();
    assert!(
        stale
            .review
            .as_ref()
            .unwrap()
            .cues
            .contains(&Cue::Translation)
    );
    let foreign = Event::new(
        Uuid::new_v4(),
        repository::now_ms(),
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(Uuid::new_v4()),
            revision: None,
            russian: "Я читаю книгу.".into(),
            purpose: PresentationPurpose::Reference,
            decision: None,
            utterance_family: None,
        }),
    )
    .unwrap();
    state.repo.append(&[foreign]).unwrap();
    state
        .set_teacher_revisit_rating(&mut stale, SelfRating::Recalled)
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut stale).unwrap();
    let cues = &state.library.attempt(attempt).unwrap().cues;
    assert!(cues.contains(&Cue::Translation) && cues.contains(&Cue::ReferenceMaterial));
    let projected = &state.teacher_revisits(repository::now_ms()).entries[0].state;
    assert_eq!(projected.rung, 0);
    assert_eq!(projected.answer_help_returns, 1);
}

#[test]
fn revisit_recent_prior_restudy_holds_cadence_without_inventing_in_session_help() {
    let (mut state, answer, _, key) = seeded();
    let prior = Event::new(
        Uuid::new_v4(),
        repository::now_ms() - DAY_MS / 24,
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(Uuid::new_v4()),
            revision: None,
            russian: "Я читаю книгу.".into(),
            purpose: PresentationPurpose::Reference,
            decision: None,
            utterance_family: None,
        }),
    )
    .unwrap();
    state.repo.append(&[prior]).unwrap();
    state.refresh().unwrap();
    let mut draft = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    state
        .set_teacher_revisit_rating(&mut draft, SelfRating::Recalled)
        .unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut draft, "Я читаю книгу.".into())
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert!(state.library.attempt(attempt).unwrap().cues.is_empty());
    let projected = &state.teacher_revisits(repository::now_ms()).entries[0].state;
    assert_eq!(projected.rung, 0);
    assert_eq!(projected.recent_restudy_returns, 1);
}

#[test]
fn revisit_missing_archive_mismatched_context_ambiguous_and_future_history_stay_outside_due() {
    let (state, _, guided, _) = seeded();
    let facts = state.library.clone();
    for scenario in ["archive", "context", "ambiguous", "future"] {
        let mut altered = facts.clone();
        if scenario == "archive" {
            altered
                .events
                .retain(|_, e| !matches!(e.fact, Fact::TeacherDocumentRecorded(_)));
        }
        if scenario == "context" {
            for e in altered.events.values_mut() {
                if let Fact::AttemptContextRecorded(c) = &mut e.fact {
                    c.reference_answer = Some("Unrelated reference".into());
                }
            }
        }
        if scenario == "ambiguous" {
            let context = altered.contexts(guided.saved_attempt.unwrap())[0].1.clone();
            let event = Event::new(
                Uuid::new_v4(),
                repository::now_ms(),
                Fact::AttemptContextRecorded(context),
            )
            .unwrap();
            altered.events.insert(event.id, event);
        }
        if scenario == "future" {
            for e in altered.events.values_mut() {
                if matches!(
                    e.fact,
                    Fact::AttemptRecorded(_) | Fact::AttemptContextRecorded(_)
                ) {
                    e.recorded_at_ms = repository::now_ms() + DAY_MS;
                }
            }
        }
        let mut replica = app();
        replica
            .repo
            .append(&altered.events.into_values().collect::<Vec<_>>())
            .unwrap();
        replica.refresh().unwrap();
        let queue = replica.teacher_revisits(repository::now_ms());
        assert!(queue.entries.is_empty(), "{scenario}");
        assert!(!queue.diagnostics.is_empty(), "{scenario}");
    }
}

#[test]
fn revisit_early_started_draft_finished_after_due_never_advances_interval() {
    let (mut state, answer, _, key) = seeded();
    let before = state.teacher_revisits(repository::now_ms()).entries[0]
        .state
        .clone();
    let early_start = before.seed.occurred_at_ms + DAY_MS / 2;
    assert!(early_start < before.due_at_ms);
    let mut draft = state.start_teacher_revisit(key, early_start).unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut draft, "Я читаю книгу.".into())
        .unwrap();
    state
        .set_teacher_revisit_rating(&mut draft, SelfRating::Recalled)
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert!(
        state.library.events[&attempt].recorded_at_ms > before.due_at_ms,
        "the synthetic early draft is submitted after the due boundary"
    );
    let context = state.library.contexts(attempt)[0].1;
    assert!(!context.delayed_probe);
    assert_eq!(
        teacher_revisit::teacher_revisit_proof(context)
            .unwrap()
            .1
            .started_at_ms,
        early_start
    );
    let after = &state.teacher_revisits(repository::now_ms()).entries[0].state;
    assert_eq!(after.rung, 0);
    assert_eq!(after.self_report_advances, 0);
    assert_eq!(after.due_at_ms, before.due_at_ms);
    assert_eq!(after.ignored_early_returns, 1);
}

#[test]
fn revisit_late_synced_in_session_reference_requalifies_cadence_without_rewriting_attempt() {
    let (mut state, answer, _, key) = seeded();
    let mut draft = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut draft, "Я читаю книгу.".into())
        .unwrap();
    state
        .set_teacher_revisit_rating(&mut draft, SelfRating::Recalled)
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    let original_bytes = serde_json::to_vec(&state.library.events[&attempt]).unwrap();
    let occurred = state.library.events[&attempt].recorded_at_ms;
    let before = state.teacher_revisits(repository::now_ms()).entries[0]
        .state
        .clone();
    assert_eq!(before.rung, 1);
    let late = Event::new(
        Uuid::new_v4(),
        draft.started_at_ms + (occurred - draft.started_at_ms) / 2,
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(Uuid::new_v4()),
            revision: None,
            russian: "Я читаю книгу.".into(),
            purpose: PresentationPurpose::Reference,
            decision: None,
            utterance_family: None,
        }),
    )
    .unwrap();
    // This independently observed reference becomes available only after submission.
    let source_id = late.id;
    state.repo.append(&[late]).unwrap();
    state.refresh().unwrap();
    let queue = state.teacher_revisits(repository::now_ms());
    let entry = &queue.entries[0];
    assert_eq!(entry.state.rung, 0);
    assert_eq!(entry.state.answer_help_returns, 1);
    assert_eq!(entry.state.due_at_ms, occurred + DAY_MS);
    assert!(entry.state.due_at_ms < before.due_at_ms);
    assert!(
        entry
            .derived_support
            .iter()
            .any(|derived| derived.attempt_id == attempt
                && derived.presentation_fact_ids == [source_id]
                && derived.omitted_source_count == 0)
    );
    assert!(
        queue
            .diagnostics
            .iter()
            .any(|text| text.contains("self-report are unchanged"))
    );
    assert_eq!(
        serde_json::to_vec(&state.library.events[&attempt]).unwrap(),
        original_bytes
    );
    assert!(state.library.attempt(attempt).unwrap().cues.is_empty());
    assert_eq!(
        state.library.attempt(attempt).unwrap().self_rating,
        SelfRating::Recalled
    );
}

#[test]
fn revisit_late_own_audio_is_derived_replay_without_rewriting_reported_cues() {
    let (mut state, answer, _, key) = seeded();
    let mut draft = state
        .start_teacher_revisit(key, repository::now_ms())
        .unwrap();
    state
        .teacher_practice_edit_response(&answer, &mut draft, "Я читаю книгу.".into())
        .unwrap();
    state
        .set_teacher_revisit_rating(&mut draft, SelfRating::Recalled)
        .unwrap();
    let attempt = state.submit_teacher_practice(&answer, &mut draft).unwrap();
    let bytes = serde_json::to_vec(&state.library.events[&attempt]).unwrap();
    assert_eq!(
        state.teacher_revisits(repository::now_ms()).entries[0]
            .state
            .rung,
        1
    );
    let audio = Event::new(
        Uuid::new_v4(),
        state.library.events[&attempt].recorded_at_ms,
        Fact::StimulusPresented(StimulusPresentation {
            session: Some(draft.id),
            revision: Some(state.library.attempt(attempt).unwrap().revision),
            russian: "Я читаю книгу.".into(),
            purpose: PresentationPurpose::Audio,
            decision: None,
            utterance_family: None,
        }),
    )
    .unwrap();
    let source = audio.id;
    state.repo.append(&[audio]).unwrap();
    state.refresh().unwrap();
    let queue = state.teacher_revisits(repository::now_ms());
    let entry = &queue.entries[0];
    assert_eq!(entry.state.rung, 0);
    assert_eq!(entry.state.replay_only_returns, 1);
    assert_eq!(entry.state.answer_help_returns, 0);
    assert!(entry.derived_support.iter().any(|s| s.attempt_id == attempt
        && s.cue == Cue::Replay
        && s.presentation_fact_ids == [source]));
    assert_eq!(
        serde_json::to_vec(&state.library.events[&attempt]).unwrap(),
        bytes
    );
    assert!(state.library.attempt(attempt).unwrap().cues.is_empty());
}
