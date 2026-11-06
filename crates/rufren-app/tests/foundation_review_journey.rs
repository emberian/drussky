//! Native journeys through the real app model, learning policy, and SQLite repository.
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

#[path = "../src/foundation.rs"]
mod foundation;
#[path = "../src/foundation_review.rs"]
mod foundation_review;
use model::AppState;
use repository::Repository;
use rufren_content::FoundationAction;
use rufren_core::*;
use std::rc::Rc;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn current(a: &AppState) -> rufren_content::FoundationStep {
    let s = a.foundation_state().unwrap();
    rufren_content::foundations().episodes[s.episode].steps[s.step].clone()
}

fn reach_recall(a: &mut AppState) -> rufren_content::FoundationStep {
    while !matches!(current(a).action, FoundationAction::Recall { .. }) {
        a.foundation_next(true).unwrap();
    }
    current(a)
}
fn answer(a: &mut AppState, step: &rufren_content::FoundationStep) {
    let FoundationAction::Recall { answer, .. } = &step.action else {
        unreachable!()
    };
    for asset in &step.audio_ids {
        a.foundation_replayed(asset).unwrap();
    }
    a.foundation_edit(vec![answer.clone()]).unwrap();
    a.foundation_submit(step).unwrap();
}
#[test]
fn delayed_return_preserves_course_draft_across_reload_and_skip_never_reports_success() {
    let mut a = app();
    assert!(a.foundation_review_queue(i64::MAX).is_empty());
    let task = reach_recall(&mut a);
    answer(&mut a, &task);
    let response_session = a.foundation_state().unwrap().session;
    let first = a
        .library
        .attempts()
        .find(|(_, attempt)| attempt.session == response_session)
        .unwrap()
        .0
        .clone();
    let bytes = serde_json::to_vec(&first).unwrap();
    let initial = a.foundation_review_queue(first.recorded_at_ms);
    assert!(initial.is_empty());
    a.foundation_next(false).unwrap();
    a.foundation_edit(vec!["unfinished course draft".into()])
        .unwrap();
    let attempts_before_help = a.library.attempts().count();
    a.foundation_help().unwrap();
    let course = a.foundation_state().unwrap();
    assert_eq!(a.library.attempts().count(), attempts_before_help);
    assert_eq!(course.draft, vec!["unfinished course draft".to_string()]);
    assert!(course.feedback.is_none());
    assert!(course.revealed && !course.cues.is_empty());
    let course_bytes = serde_json::to_vec(&course).unwrap();
    let due = first.recorded_at_ms + 2 * DAY_MS;
    let queue = a.foundation_review_queue(due);
    assert!(queue.iter().any(|e| e.step_id == task.id));
    a.foundation_start_review_at(&task.id, due).unwrap();
    let active = a.foundation_state().unwrap();
    assert_ne!(active.session, course.session);
    assert!(
        active.draft.is_empty()
            && active.cues.is_empty()
            && active.feedback.is_none()
            && !active.revealed
    );
    assert_eq!(active.step_id, task.id);
    assert!(active.review_resume.is_some());
    let mut resumed = AppState::from_repository(a.repo.clone()).unwrap();
    resumed.start_foundations().unwrap();
    assert_eq!(resumed.foundation_state().unwrap().session, active.session);
    // Skip is navigation only: no additional response or correctness judgment.
    let attempts_before_skip = resumed.library.attempts().count();
    let judgments_before_skip = resumed
        .library
        .events
        .values()
        .filter(|e| matches!(e.fact, Fact::CoachJudgmentRecorded(_)))
        .count();
    resumed.foundation_next(true).unwrap();
    assert_eq!(resumed.library.attempts().count(), attempts_before_skip);
    assert_eq!(
        resumed
            .library
            .events
            .values()
            .filter(|e| matches!(e.fact, Fact::CoachJudgmentRecorded(_)))
            .count(),
        judgments_before_skip
    );
    assert_eq!(
        serde_json::to_vec(&resumed.foundation_state().unwrap()).unwrap(),
        course_bytes
    );
    assert_eq!(
        serde_json::to_vec(&resumed.library.events[&first.id]).unwrap(),
        bytes
    );
}

#[test]
fn due_queue_is_bounded_and_assistance_does_not_become_a_clean_self_report() {
    let mut a = app();
    let first = reach_recall(&mut a);
    a.foundation_help().unwrap();
    answer(&mut a, &first);
    let session = a.foundation_state().unwrap().session;
    let attempt = a
        .library
        .attempts()
        .find(|(_, attempt)| attempt.session == session)
        .unwrap()
        .1;
    assert_eq!(attempt.self_rating, SelfRating::NotReported);
    assert!(!attempt.cues.is_empty());
    assert!(
        a.foundation_review_queue(repository::now_ms() + DAY_MS)
            .len()
            <= 3
    );
    assert!(
        a.foundation_review_queue(repository::now_ms() + DAY_MS)
            .iter()
            .any(|e| e.step_id == first.id)
    );
}

#[test]
fn completed_actual_due_return_restores_course_and_keeps_exact_delayed_context() {
    let mut original = app();
    let task = reach_recall(&mut original);
    answer(&mut original, &task);
    original.foundation_next(false).unwrap();
    original
        .foundation_edit(vec!["course answer still being written".into()])
        .unwrap();
    let course = original.foundation_state().unwrap();
    let course_bytes = serde_json::to_vec(&course).unwrap();
    // Replay an existing completed task three days in the past. The new response below
    // uses the real clock, so an open future fixture cannot fabricate a delayed start.
    let historical: Vec<_> = original
        .library
        .events
        .values()
        .cloned()
        .map(|mut event| {
            if event.recorded_at_ms > 0 {
                event.recorded_at_ms -= 3 * DAY_MS;
            }
            event
        })
        .collect();
    let repo = Rc::new(Repository::open_at(":memory:").unwrap());
    repo.append(&historical).unwrap();
    let mut returning = AppState::from_repository(repo).unwrap();
    returning.foundation_save(&course).unwrap();
    assert!(
        returning
            .foundation_review_queue(repository::now_ms())
            .iter()
            .any(|e| e.step_id == task.id)
    );
    returning.foundation_start_review(&task.id).unwrap();
    let session = returning.foundation_state().unwrap().session;
    answer(&mut returning, &task);
    let (completed, response) = returning
        .library
        .attempts()
        .find(|(_, a)| a.session == session)
        .unwrap();
    assert_eq!(response.self_rating, SelfRating::NotReported);
    assert_eq!(returning.library.contexts(completed.id).len(), 1);
    assert!(returning.library.contexts(completed.id)[0].1.delayed_probe);
    let decision = returning
        .library
        .review_start_for_session(
            session,
            ReviewTarget::Passage {
                revision: response.revision,
                skill: response.skill,
            },
        )
        .unwrap();
    assert!(decision.started_at_ms >= decision.baseline_due_at_ms);
    assert!(decision.started_at_ms <= completed.recorded_at_ms);
    assert!(
        !returning
            .foundation_review_queue(completed.recorded_at_ms)
            .iter()
            .any(|e| e.step_id == task.id)
    );
    returning.foundation_next(false).unwrap();
    assert_eq!(
        serde_json::to_vec(&returning.foundation_state().unwrap()).unwrap(),
        course_bytes
    );
    for old in historical {
        assert_eq!(
            serde_json::to_vec(&returning.library.events[&old.id]).unwrap(),
            serde_json::to_vec(&old).unwrap()
        );
    }
}
