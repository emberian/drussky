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
use rufren_content::foundations::{
    FOUNDATION_ORIGINAL_REVISION, FOUNDATION_PREVIOUS_REVISION, FoundationRecallMode,
    foundation_step_pack_revision, foundations_legacy, foundations_previous,
};
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
    let step_count = rufren_content::foundations()
        .episodes
        .iter()
        .map(|e| e.steps.len())
        .sum::<usize>();
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
            FoundationAction::Copy { answer, .. } => {
                a.foundation_edit(vec![answer.clone()]).unwrap();
                a.foundation_submit(&step).unwrap()
            }
            FoundationAction::Recall { answer, mode, .. } => {
                a.foundation_edit(vec![if matches!(mode, FoundationRecallMode::Spoken) {
                    "I tried aloud (self-report).".into()
                } else {
                    answer.clone()
                }])
                .unwrap();
                a.foundation_submit(&step).unwrap()
            }
            _ => (),
        }
        a.foundation_next(false).unwrap();
        turns += 1;
        assert!(turns <= step_count);
    }
    assert_eq!(
        a.foundation_state().unwrap().episode,
        rufren_content::foundations().episodes.len() - 1
    );
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
fn old_pack_reserved_packet_migrates_and_recovers_partial_append_identically() {
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
    reserved["revision"] = foundations_legacy().revision.into();
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
        format!(
            "rufren-foundation:{}:{}",
            foundation_step_pack_revision(&step.id),
            step.id
        )
        .as_bytes(),
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

fn at_step(a: &mut AppState, id: &str) {
    a.start_foundations().unwrap();
    let pack = rufren_content::foundations();
    let mut state = a.foundation_state().unwrap();
    let (ei, si, episode, step) = pack
        .episodes
        .iter()
        .enumerate()
        .find_map(|(ei, e)| {
            e.steps
                .iter()
                .enumerate()
                .find(|(_, s)| s.id == id)
                .map(|(si, s)| (ei, si, e, s))
        })
        .expect("authored step");
    state.episode = ei;
    state.step = si;
    state.episode_id = episode.id.clone();
    state.step_id = step.id.clone();
    state.step_revision = step.revision.clone();
    state.session = uuid::Uuid::new_v4();
    state.finished = false;
    state.draft.clear();
    state.cues.clear();
    state.played.clear();
    state.feedback = None;
    state.accepted = false;
    state.revealed = false;
    state.pending = None;
    state.cannot_yet = false;
    // Prior encounters keep this test about the target retrieval rather than skipped prerequisites.
    state.encounters = pack
        .episodes
        .iter()
        .take(ei + 1)
        .enumerate()
        .flat_map(|(index, e)| {
            e.steps
                .iter()
                .take(if index == ei { si } else { e.steps.len() })
        })
        .map(|s| format!("{}:encountered", s.id))
        .collect();
    a.foundation_save(&state).unwrap();
}
#[test]
fn append_only_upgrade_preserves_exact_old_draft_aids_and_original_source() {
    let mut a = app();
    at_step(&mut a, "f1-build-am");
    let mut old = a.foundation_state().unwrap();
    old.revision = foundations_legacy().revision;
    old.draft = vec!["а".into()];
    old.cues = vec![Cue::Coach];
    old.revealed = true;
    let session = old.session;
    a.foundation_save(&old).unwrap();
    let resumed = a.foundation_state().unwrap();
    assert_eq!(resumed.session, session);
    assert_eq!(resumed.draft, old.draft);
    assert_eq!(resumed.cues, old.cues);
    assert!(resumed.revealed);
    assert_eq!(resumed.step_id, "f1-build-am");
    assert_eq!(resumed.revision, rufren_content::foundations().revision);
    let step = current(&a);
    assert_eq!(
        a.foundation_revision(&resumed, &step).unwrap().id,
        uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_URL,
            format!("rufren-foundation:{}:{}", old.revision, step.id).as_bytes()
        )
    );
    let mut invalid = resumed;
    invalid.revision = "unknown-future-edition".into();
    a.foundation_save(&invalid).unwrap();
    assert!(a.foundation_state().is_err());
    assert_eq!(
        a.repo
            .get::<foundation::FoundationState>("foundation_journey_v1")
            .unwrap()
            .unwrap()
            .revision,
        "unknown-future-edition"
    );
}
#[test]
fn completed_original_course_opens_first_new_episode_without_erasing_history() {
    let mut a = app();
    at_step(&mut a, "f3-end");
    let mut old = a.foundation_state().unwrap();
    old.revision = foundations_legacy().revision;
    old.finished = true;
    old.encounters.push("f3-end:encountered".into());
    let encounters = old.encounters.clone();
    a.foundation_save(&old).unwrap();
    let next = a.foundation_state().unwrap();
    assert_eq!(next.episode, 3);
    assert_eq!(next.step, 0);
    assert!(!next.finished);
    assert_eq!(next.encounters, encounters);
    assert_eq!(
        next.step_id,
        rufren_content::foundations().episodes[3].steps[0].id
    );
}
#[test]
fn hidden_model_meaning_recall_preserves_paraphrase_without_false_rejection() {
    let mut a = app();
    let pack = rufren_content::foundations();
    let step = pack
        .episodes
        .iter()
        .flat_map(|e| &e.steps)
        .find(|s| {
            matches!(
                s.action,
                FoundationAction::Recall {
                    mode: FoundationRecallMode::ListeningMeaning,
                    ..
                }
            )
        })
        .unwrap()
        .clone();
    at_step(&mut a, &step.id);
    assert!(!a.foundation_state().unwrap().revealed);
    a.foundation_replayed(&step.audio_ids[0]).unwrap();
    assert!(
        a.foundation_state().unwrap().cues.is_empty(),
        "first listening input is not replay help"
    );
    let paraphrase = "I understand it as referring to the person being spoken about";
    a.foundation_edit(vec![paraphrase.into()]).unwrap();
    a.foundation_submit(&step).unwrap();
    let state = a.foundation_state().unwrap();
    assert!(state.revealed);
    let attempt = *state.attempts.last().unwrap();
    assert_eq!(
        a.library
            .attempts()
            .find(|(e, _)| e.id == attempt)
            .unwrap()
            .1
            .response,
        paraphrase
    );
    assert!(
        !a.library
            .events
            .values()
            .any(|e| matches!(&e.fact,Fact::CoachJudgmentRecorded(j) if j.attempt==attempt)),
        "unlisted English paraphrase is for self-comparison, never finite-list rejection"
    );
}
#[test]
fn cannot_yet_and_requested_model_do_not_fabricate_correct_recall() {
    let mut a = app();
    let step = rufren_content::foundations()
        .episodes
        .into_iter()
        .flat_map(|e| e.steps)
        .find(|s| {
            matches!(
                s.action,
                FoundationAction::Recall {
                    mode: FoundationRecallMode::Written,
                    ..
                }
            )
        })
        .unwrap();
    at_step(&mut a, &step.id);
    a.foundation_cannot_yet(&step).unwrap();
    let first = a.foundation_state().unwrap();
    assert!(first.revealed);
    assert!(!first.accepted);
    let attempt = *first.attempts.last().unwrap();
    assert!(
        !a.library
            .events
            .values()
            .any(|e| matches!(&e.fact,Fact::CoachJudgmentRecorded(j) if j.attempt==attempt))
    );
    a.foundation_retry().unwrap();
    let FoundationAction::Recall { answer, .. } = &step.action else {
        unreachable!()
    };
    a.foundation_edit(vec![answer.clone()]).unwrap();
    a.foundation_submit(&step).unwrap();
    let second = a.foundation_state().unwrap();
    assert_eq!(second.attempts.len(), 2);
    let (_, response) = a
        .library
        .attempts()
        .find(|(e, _)| e.id == second.attempts[1])
        .unwrap();
    assert!(response.cues.contains(&Cue::Coach));
    assert_eq!(response.self_rating, SelfRating::NotReported);
}

#[test]
fn seven_episode_upgrade_preserves_every_unfinished_state_field() {
    let mut a = app();
    at_step(&mut a, "c5-window-write");
    let mut old = a.foundation_state().unwrap();
    old.revision = FOUNDATION_PREVIOUS_REVISION.into();
    old.draft = vec!["ок".into()];
    old.cues = vec![Cue::Coach, Cue::OriginalText];
    old.revealed = true;
    old.played = vec!["foundation-okno".into()];
    old.cannot_yet = true;
    old.attempts.push(uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        b"saved-old-attempt-reference",
    ));
    let mut expected = serde_json::to_value(&old).unwrap();
    expected["revision"] = rufren_content::foundations().revision.into();
    a.foundation_save(&old).unwrap();
    let mut reopened = AppState::from_repository(a.repo.clone()).unwrap();
    reopened.start_foundations().unwrap();
    assert_eq!(
        serde_json::to_value(reopened.foundation_state().unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        foundation_step_pack_revision("c5-window-write"),
        FOUNDATION_PREVIOUS_REVISION
    );
}

#[test]
fn completed_seven_episode_course_opens_alphabet_without_rewriting_completions() {
    let mut a = app();
    at_step(&mut a, "c6-end");
    let mut old = a.foundation_state().unwrap();
    old.revision = FOUNDATION_PREVIOUS_REVISION.into();
    old.finished = true;
    old.cannot_yet = true;
    old.encounters.push("c6-end:encountered".into());
    let encounters = old.encounters.clone();
    let attempts = old.attempts.clone();
    a.foundation_save(&old).unwrap();
    let next = a.foundation_state().unwrap();
    assert_eq!(next.step_id, "a8-ya");
    assert_eq!(next.episode, 7);
    assert_eq!(next.step, 0);
    assert!(!next.finished && !next.cannot_yet);
    assert_eq!(next.encounters, encounters);
    assert_eq!(next.attempts, attempts);
    assert_ne!(next.session, old.session);
}

#[test]
fn every_old_authored_event_retains_frozen_bytes_and_source_edition() {
    let a = app();
    let state = a.foundation_state().unwrap();
    let original = foundations_legacy();
    let original_ids: std::collections::BTreeSet<_> = original
        .episodes
        .iter()
        .flat_map(|e| &e.steps)
        .map(|s| s.id.as_str())
        .collect();
    let frozen = foundations_previous();
    let current = rufren_content::foundations();
    for old in frozen.episodes.iter().flat_map(|e| &e.steps) {
        let source_edition = if original_ids.contains(old.id.as_str()) {
            FOUNDATION_ORIGINAL_REVISION
        } else {
            FOUNDATION_PREVIOUS_REVISION
        };
        let Some(russian) = old
            .audio_ids
            .first()
            .and_then(|id| frozen.assets.iter().find(|a| &a.id == id))
            .map(|a| a.spoken.clone())
            .or_else(|| {
                old.display
                    .clone()
                    .filter(|text| text.chars().any(|c| ('А'..='я').contains(&c)))
            })
        else {
            continue;
        };
        let current_step = current
            .episodes
            .iter()
            .flat_map(|e| &e.steps)
            .find(|s| s.id == old.id)
            .unwrap();
        assert_eq!(
            serde_json::to_vec(current_step).unwrap(),
            serde_json::to_vec(old).unwrap()
        );
        let id = uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_URL,
            format!("rufren-foundation:{source_edition}:{}", old.id).as_bytes(),
        );
        // This is the historical v8 envelope, independently resolved from the
        // frozen .2 graph and original .1 membership, never the current resolver.
        let expected = Event {
            version: 8,
            id,
            device: uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, b"rufren-foundation-author-v1"),
            recorded_at_ms: 0,
            fact: Fact::PassageRevised(Revision {
                passage: Passage {
                    id,
                    title: old.instruction.clone(),
                    russian,
                    translation: old.instruction.clone(),
                    source: format!(
                        "rufren-foundation-v1:{source_edition}:{}@{}",
                        old.id, old.revision
                    ),
                    coach_reviewed: false,
                },
                parents: std::collections::BTreeSet::new(),
            }),
        };
        assert_eq!(
            serde_json::to_vec(&a.foundation_revision(&state, current_step).unwrap()).unwrap(),
            serde_json::to_vec(&expected).unwrap(),
            "historical source {} changed",
            old.id
        );
    }
}

#[test]
fn previous_edition_reserved_recall_recovers_same_partial_packet_once() {
    let mut a = app();
    at_step(&mut a, "c6-they-read");
    let step = current(&a);
    let FoundationAction::Recall { answer, .. } = &step.action else {
        unreachable!()
    };
    a.foundation_edit(vec![answer.clone()]).unwrap();
    a.foundation_submit(&step).unwrap();
    let state = a.foundation_state().unwrap();
    let attempt = *state.attempts.last().unwrap();
    let events: Vec<_> = a
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
    reserved["revision"] = FOUNDATION_PREVIOUS_REVISION.into();
    reserved["feedback"] = serde_json::Value::Null;
    reserved["accepted"] = false.into();
    reserved["attempts"] = serde_json::json!([]);
    reserved["pending"] = serde_json::json!({"events": events, "feedback": state.feedback.unwrap(), "accepted": true});
    let mut b = app();
    let pending = serde_json::from_value(reserved).unwrap();
    b.foundation_save(&pending).unwrap();
    let original_attempt = a.library.events[&attempt].clone();
    let original_bytes = serde_json::to_vec(&original_attempt).unwrap();
    b.repo.append(&[original_attempt]).unwrap();
    b.foundation_submit(&step).unwrap();
    b.foundation_submit(&step).unwrap();
    assert_eq!(b.library.attempts().count(), 1);
    assert!(b.foundation_state().unwrap().pending.is_none());
    assert_eq!(
        serde_json::to_vec(&b.library.events[&attempt]).unwrap(),
        original_bytes
    );
    let context = b.library.contexts(attempt);
    assert_eq!(context.len(), 1);
    assert_eq!(
        context[0].1.content_revision.as_deref(),
        Some(FOUNDATION_PREVIOUS_REVISION)
    );
    assert_eq!(
        context[0].1.prompt.as_deref(),
        Some(serde_json::to_string(&step).unwrap().as_str())
    );
}
