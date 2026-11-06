//! Regression journeys for switching tasks without losing unfinished learning work.
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
#[path = "../src/repository.rs"]
mod repository;
#[path = "../src/recognition.rs"]
mod recognition;
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
use rufren_content::ExerciseKind;
use rufren_core::*;
use std::rc::Rc;
use uuid::Uuid;

#[test]
fn audit_drafts_switch_reopen_resume_and_complete_keep_each_question_and_hint() {
    let path = std::env::temp_dir().join(format!("rufren-audit-drafts-{}.sqlite", Uuid::new_v4()));
    let open = || AppState::from_repository(Rc::new(Repository::open_at(&path).unwrap())).unwrap();
    let mut state = open();
    let exercises: Vec<_> = state
        .content
        .exercises
        .iter()
        .filter(|e| e.kind == ExerciseKind::WrittenRetrieval)
        .take(2)
        .map(|e| e.id.clone())
        .collect();
    state.start_exercise(&exercises[0]).unwrap();
    let original_question = state.exercise().unwrap().clone();
    let original = state.session.as_ref().unwrap().id;
    state.phase(Phase::Retrieve).unwrap();
    state.update_response("Мой первый ответ".into()).unwrap();
    state.hint().unwrap();
    state.start_exercise(&exercises[1]).unwrap();
    let second = state.session.as_ref().unwrap().id;
    state.phase(Phase::Retrieve).unwrap();
    state.update_response("Мой второй ответ".into()).unwrap();
    assert_eq!(state.paused_practices().unwrap().len(), 1);
    drop(state);

    let mut state = open();
    assert_eq!(state.session.as_ref().unwrap().id, second);
    state.resume_practice(original).unwrap();
    assert_eq!(state.session.as_ref().unwrap().draft, "Мой первый ответ");
    assert_eq!(
        state.session.as_ref().unwrap().cues,
        vec![Cue::OriginalText]
    );
    assert_eq!(state.exercise().unwrap(), &original_question);
    assert_eq!(state.paused_practices().unwrap()[0].session.id, second);
    state.phase(Phase::Reflect).unwrap();
    state.record(SelfRating::Recalled).unwrap();
    let (event, attempt) = state
        .library
        .attempts()
        .find(|(_, a)| a.session == original)
        .unwrap();
    assert_eq!(
        SpacedReview.evidence(&state.library, event.id, attempt),
        ReviewEvidence::Assisted
    );
    state.resume_practice(second).unwrap();
    assert_eq!(state.session.as_ref().unwrap().draft, "Мой второй ответ");
    assert!(state.paused_practices().unwrap().is_empty());
    assert!(
        state.resume_practice(original).is_err(),
        "completed archived copies cannot restart evidence"
    );
    drop(state);
    std::fs::remove_file(path).unwrap(); // Only this test's unique database.
}

#[test]
fn audit_drafts_unused_encounters_do_not_clutter_paused_practice_and_archive_is_idempotent() {
    let repo = Rc::new(Repository::open_at(":memory:").unwrap());
    let mut state = AppState::from_repository(repo).unwrap();
    let id = state
        .content
        .exercises
        .iter()
        .find(|e| e.kind == ExerciseKind::WrittenRetrieval)
        .unwrap()
        .id
        .clone();
    state.start_exercise(&id).unwrap();
    state.start_exercise(&id).unwrap();
    assert!(state.paused_practices().unwrap().is_empty());
    state.phase(Phase::Retrieve).unwrap();
    state.update_response("Сохранить".into()).unwrap();
    let original = state.session.as_ref().unwrap().id;
    state.archive_current().unwrap();
    state.archive_current().unwrap();
    assert!(
        state.paused_practices().unwrap().is_empty(),
        "active draft is not also paused"
    );
    state.start_exercise(&id).unwrap();
    let paused = state.paused_practices().unwrap();
    assert_eq!(paused.len(), 1);
    assert_eq!(paused[0].session.id, original);
}

#[test]
fn audit_drafts_interrupted_resume_recovers_active_question_without_rolling_back_response() {
    let repo = Rc::new(Repository::open_at(":memory:").unwrap());
    let mut state = AppState::from_repository(repo.clone()).unwrap();
    let ids: Vec<_> = state
        .content
        .exercises
        .iter()
        .filter(|e| e.kind == ExerciseKind::WrittenRetrieval)
        .take(2)
        .map(|e| e.id.clone())
        .collect();
    state.start_exercise(&ids[0]).unwrap();
    let old_plan = state.plan.clone().unwrap();
    state.phase(Phase::Retrieve).unwrap();
    state.update_response("Первый".into()).unwrap();
    state.start_exercise(&ids[1]).unwrap();
    state.phase(Phase::Retrieve).unwrap();
    state.update_response("Второй".into()).unwrap();
    let active = state.session.as_ref().unwrap().id;
    let question = state.exercise().unwrap().clone();
    // Resume archives the current draft, then persists the selected plan before
    // changing the active session. Stop after that first metadata write.
    state.archive_current().unwrap();
    repo.set("session_plan", &old_plan).unwrap();
    drop(state);
    let mut state = AppState::from_repository(repo).unwrap();
    assert_eq!(state.session.as_ref().unwrap().id, active);
    assert_eq!(state.session.as_ref().unwrap().draft, "Второй");
    assert_eq!(state.exercise().unwrap(), &question);
    // An edit made after recovery must remain newer than the archived copy.
    state.update_response("Второй, продолжение".into()).unwrap();
    let repo = state.repo.clone();
    drop(state);
    let state = AppState::from_repository(repo).unwrap();
    assert_eq!(state.session.as_ref().unwrap().draft, "Второй, продолжение");
    assert_eq!(state.exercise().unwrap(), &question);
}
