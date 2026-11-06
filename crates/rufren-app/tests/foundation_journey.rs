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
#[test]
fn exposure_is_not_attempt_and_wrong_answer_survives_retry() {
    let mut a = app();
    a.start_foundations().unwrap();
    while !matches!(current(&a).action, FoundationAction::Choose { .. }) {
        for asset in &current(&a).audio_ids {
            a.foundation_replayed(asset).unwrap()
        }
        a.foundation_next(false).unwrap()
    }
    assert_eq!(a.library.attempts().count(), 0);
    let step = current(&a);
    let FoundationAction::Choose {
        options,
        correct_id,
        ..
    } = &step.action
    else {
        unreachable!()
    };
    a.foundation_replayed(&step.audio_ids[0]).unwrap();
    let wrong = options.iter().find(|o| &o.id != correct_id).unwrap();
    a.foundation_edit(vec![wrong.id.clone()]).unwrap();
    a.foundation_submit(&step).unwrap();
    let first = a.foundation_state().unwrap();
    assert!(!first.accepted);
    assert_eq!(first.attempts.len(), 1);
    let attempt = first.attempts[0];
    assert_eq!(
        a.library
            .attempts()
            .find(|(e, _)| e.id == attempt)
            .unwrap()
            .1
            .response,
        wrong.text
    );
    a.foundation_retry().unwrap();
    a.foundation_edit(vec![correct_id.clone()]).unwrap();
    a.foundation_submit(&step).unwrap();
    let second = a.foundation_state().unwrap();
    assert!(second.accepted);
    assert_eq!(second.attempts.len(), 2);
    let saved = a
        .library
        .attempts()
        .find(|(e, _)| e.id == second.attempts[1])
        .unwrap()
        .1;
    assert!(saved.cues.contains(&Cue::Coach));
    assert_eq!(saved.self_rating, SelfRating::NotReported);
    assert_eq!(
        a.library
            .attempts()
            .find(|(e, _)| e.id == attempt)
            .unwrap()
            .1
            .response,
        wrong.text
    );
    a.foundation_submit(&step).unwrap();
    assert_eq!(a.library.attempts().count(), 2);
}
#[test]
fn exact_draft_help_and_place_resume_without_advancing() {
    let mut a = app();
    a.start_foundations().unwrap();
    while !matches!(current(&a).action, FoundationAction::Blend { .. }) {
        a.foundation_next(true).unwrap()
    }
    let step = current(&a);
    let FoundationAction::Blend { tiles, .. } = step.action else {
        unreachable!()
    };
    a.foundation_edit(vec![tiles[0].clone()]).unwrap();
    a.foundation_help().unwrap();
    let before = a.foundation_state().unwrap();
    let mut b = AppState::from_repository(a.repo.clone()).unwrap();
    b.start_foundations().unwrap();
    let after = b.foundation_state().unwrap();
    assert_eq!(before.session, after.session);
    assert_eq!(before.step, after.step);
    assert_eq!(before.draft, after.draft);
    assert!(after.revealed);
    assert_eq!(before.cues, after.cues);
    assert!(after.feedback.is_none());
    assert_eq!(b.library.attempts().count(), 0);
}
#[test]
fn all_episodes_can_be_practiced_without_microphone_or_teacher() {
    let mut a = app();
    a.start_foundations().unwrap();
    let mut turns = 0;
    while !a.foundation_state().unwrap().finished {
        let step = current(&a);
        for asset in &step.audio_ids {
            a.foundation_replayed(asset).unwrap()
        }
        match &step.action {
            FoundationAction::Choose { correct_id, .. } => {
                a.foundation_edit(vec![correct_id.clone()]).unwrap();
                a.foundation_submit(&step).unwrap()
            }
            FoundationAction::Blend { answer, .. } => {
                a.foundation_edit(answer.clone()).unwrap();
                a.foundation_submit(&step).unwrap()
            }
            FoundationAction::Read { .. } => a.foundation_read_try(&step).unwrap(),
            _ => (),
        }
        a.foundation_next(false).unwrap();
        turns += 1;
        assert!(turns < 80);
    }
    assert_eq!(a.foundation_state().unwrap().episode, 2);
    assert!(a.library.attempts().count() > 8);
    for (_, attempt) in a.library.attempts() {
        assert_eq!(attempt.self_rating, SelfRating::NotReported);
        let revision = a
            .library
            .events
            .get(&attempt.revision)
            .expect("reading and response evidence resolves an authored source");
        assert!(is_foundation_reference(revision));
    }
    let now = repository::now_ms();
    assert!(
        SpacedReview
            .queue(
                &a.library,
                now,
                &[
                    Skill::Reading,
                    Skill::Listening,
                    Skill::Writing,
                    Skill::Speaking
                ]
            )
            .is_empty(),
        "foundation targets must not become generic review phrases"
    );
    let plan = a.prepare_daily_plan(now).unwrap();
    assert!(
        plan.items.is_empty(),
        "foundation source anchors must not leak into the daily plan"
    );
}

#[test]
fn reserved_packet_recovers_a_partial_append_with_identical_events() {
    let mut a = app();
    a.start_foundations().unwrap();
    while !matches!(current(&a).action, FoundationAction::Choose { .. }) {
        a.foundation_next(true).unwrap()
    }
    let step = current(&a);
    let FoundationAction::Choose { correct_id, .. } = &step.action else {
        unreachable!()
    };
    a.foundation_replayed(&step.audio_ids[0]).unwrap();
    a.foundation_edit(vec![correct_id.clone()]).unwrap();
    a.foundation_submit(&step).unwrap();
    let state = a.foundation_state().unwrap();
    let attempt = *state.attempts.last().unwrap();
    let events: Vec<Event> = a
        .library
        .events
        .values()
        .filter(|e| match &e.fact {
            Fact::AttemptRecorded(_) => e.id == attempt,
            Fact::AttemptContextRecorded(c) => c.attempt == attempt,
            Fact::CoachJudgmentRecorded(j) => j.attempt == attempt,
            Fact::PassageRevised(_) => true,
            _ => false,
        })
        .cloned()
        .collect();
    let mut reserved = serde_json::to_value(&state).unwrap();
    reserved["feedback"] = serde_json::Value::Null;
    reserved["accepted"] = false.into();
    reserved["pending"] =
        serde_json::json!({"events":events,"feedback":state.feedback.unwrap(),"accepted":true});
    reserved["attempts"] = serde_json::json!([]);
    let mut b = app();
    let pending: foundation::FoundationState = serde_json::from_value(reserved).unwrap();
    b.foundation_save(&pending).unwrap();
    let original = a.library.events.get(&attempt).unwrap().clone();
    b.repo.append(&[original.clone()]).unwrap();
    assert!(b.foundation_edit(vec!["different content".into()]).is_err());
    b.foundation_submit(&step).unwrap();
    assert_eq!(b.library.events.get(&attempt).unwrap(), &original);
    assert_eq!(b.library.contexts(attempt).len(), 1);
    assert!(b.foundation_state().unwrap().pending.is_none());
    b.foundation_submit(&step).unwrap();
    assert_eq!(b.library.attempts().count(), 1);
}

#[test]
fn optional_recording_resolves_its_authored_russian_reference() {
    let mut a = app();
    a.start_foundations().unwrap();
    while !matches!(current(&a).action, FoundationAction::Speak { .. }) {
        a.foundation_next(true).unwrap()
    }
    let state = a.foundation_state().unwrap();
    let step = current(&a);
    let revision = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        format!("rufren-foundation:{}:{}", state.revision, step.id).as_bytes(),
    );
    let metadata:AudioRecordMetadata=serde_json::from_value(serde_json::json!({"asset_id":"foundation-test-take","content_hash":"a".repeat(64),"role":"attempt","name":"Test original","passage_id":revision.to_string(),"revision_id":revision.to_string(),"duration_ms":1000,"sample_rate":16000,"channels":1,"frames":16000,"byte_length":32044,"created_at_ms":repository::now_ms(),"mime_type":"audio/wav"})).unwrap();
    a.foundation_save_recording(metadata.clone()).unwrap();
    assert_eq!(a.foundation_state().unwrap().recording, Some(metadata));
    assert!(is_foundation_reference(
        a.library.events.get(&revision).unwrap()
    ));
    assert_eq!(a.library.revision(revision).unwrap().passage.russian, "кот");
    assert_eq!(
        a.library
            .recordings(AnnotationTarget::Revision(revision))
            .len(),
        1
    );
}
