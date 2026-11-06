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
#[path = "../src/handwriting.rs"]
mod handwriting;
use handwriting::*;
use model::{AppState, Page};
use repository::Repository;
use rufren_content::handwriting_tasks::{
    FoundationHandwritingSequence as Sequence, handwriting_task_for_step,
};
use std::rc::Rc;
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn source(a: &mut AppState, id: &str) {
    a.start_foundations().unwrap();
    let pack = rufren_content::foundations();
    let mut state = a.foundation_state().unwrap();
    let (ei, si, e, s) = pack
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
        .unwrap();
    state.episode = ei;
    state.step = si;
    state.episode_id = e.id.clone();
    state.step_id = s.id.clone();
    state.step_revision = s.revision.clone();
    state.session = Uuid::new_v4();
    state.played = s.audio_ids.clone();
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
fn begin(a: &mut AppState, step: &str, sequence: Sequence) -> HandwritingActivity {
    source(a, step);
    let task = handwriting_task_for_step(step).unwrap();
    a.open_writing_task(&task.id, sequence).unwrap();
    a.active_handwriting().unwrap()
}
fn drawing(run: &HandwritingActivity, sealed: bool) -> HandwritingSubmission {
    let task = run.task();
    HandwritingSubmission {
        version: 1,
        model_version: "hand-printed-six-v2".into(),
        view_box: [task.paper.width, task.paper.height],
        scope_id: run.scope(),
        task_id: task.id,
        task_revision: task.revision,
        target: task.target,
        audio_id: task.reference_audio_id,
        audio_role: task.audio_role,
        attempt_id: run.attempt_id,
        created_at_ms: 123456,
        mode: if run.sequence == Sequence::SeeCopyHearSay {
            InkMode::Copy
        } else {
            InkMode::Hidden
        },
        input_mode: "pen".into(),
        meaning_visible: false,
        sound_visible: true,
        sealed,
        strokes: vec![InkStroke {
            pointer_id: 1,
            pointer_type: "pen".into(),
            capture_geometry: None,
            active: false,
            interrupted: false,
            cancel_reason: None,
            points: vec![
                InkPoint {
                    x: 127.8,
                    y: 277.2,
                    time_ms: 111.5,
                    pressure: 0.7,
                    tilt_x: 12.0,
                    tilt_y: -7.0,
                    twist: 30.0,
                    width: 2.0,
                    height: 2.0,
                },
                InkPoint {
                    x: 184.8,
                    y: 132.8,
                    time_ms: 117.7,
                    pressure: 0.6,
                    tilt_x: 15.0,
                    tilt_y: -8.0,
                    twist: 30.0,
                    width: 2.0,
                    height: 2.0,
                },
            ],
        }],
        aids: vec![],
        saved_at_ms: sealed.then_some(123999),
        prior_attempt_id: None,
        pending_snapshot: None,
    }
}
fn save(a: &mut AppState, run: &HandwritingActivity, ink: &HandwritingSubmission) {
    a.handwriting_store_original(&run.scope(), &run.task(), ink)
        .unwrap();
    a.handwriting_store_draft(&run.scope(), &run.task(), ink)
        .unwrap();
    a.writing_accept_web(run.id, run.round_id, ink).unwrap();
}
#[test]
fn writing_requires_taught_source_and_preserves_exact_course() {
    let mut a = app();
    a.start_foundations().unwrap();
    assert!(a.open_handwriting().is_err());
    a.foundation_replayed("foundation-a").unwrap();
    let before = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    a.open_handwriting().unwrap();
    assert!(a.page == Page::Handwriting);
    let run = a.active_handwriting().unwrap();
    assert_eq!(run.definition().unwrap().target, "А");
    assert_eq!(
        serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
        before
    );
    assert_eq!(a.library.attempts().count(), 0);
}
#[test]
fn original_pressure_is_immutable_and_separate_from_repair_draft() {
    let mut a = app();
    let run = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let original = drawing(&run, true);
    save(&mut a, &run, &original);
    let mut altered = original.clone();
    altered.strokes[0].points[0].pressure = 0.2;
    assert!(
        a.handwriting_store_original(&run.scope(), &run.task(), &altered)
            .is_err()
    );
    assert_eq!(
        a.handwriting_originals(&run.scope(), &run.task()).unwrap(),
        vec![original]
    );
    assert_eq!(a.active_handwriting().unwrap().stage, WritingStage::Compare);
    assert_eq!(a.library.attempts().count(), 0);
}
#[test]
fn word_and_phrase_complete_connected_audio_ink_compare_say_without_grade() {
    for step in ["f2-cat", "f3-message"] {
        let mut a = app();
        let run = begin(&mut a, step, Sequence::HearWriteCompareSay);
        let before = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
        let original = drawing(&run, true);
        a.handwriting_store_original(&run.scope(), &run.task(), &original)
            .unwrap();
        assert!(
            a.writing_accept_web(run.id, run.round_id, &original)
                .is_err(),
            "unhearddictationcannotcomplete"
        );
        a.writing_audio_completed(run.id, run.round_id, &run.definition().unwrap().audio_id)
            .unwrap();
        a.writing_accept_web(run.id, run.round_id, &original)
            .unwrap();
        a.writing_go_say(run.id, run.round_id).unwrap();
        a.writing_say_done(run.id, run.round_id, SayDisposition::SelfReportedSaid)
            .unwrap();
        let done = a.active_handwriting().unwrap();
        assert_eq!(done.stage, WritingStage::Complete);
        assert_eq!(done.say, SayDisposition::SelfReportedSaid);
        assert_eq!(
            serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
            before
        );
        let export = a.handwriting_export().unwrap();
        assert_eq!(
            export["originals"][0]["payload"]["strokes"][0]["points"][0]["pressure"],
            0.7
        );
        assert_eq!(a.library.attempts().count(), 0);
        assert!(
            !run.task()
                .instruction
                .contains(&run.definition().unwrap().target)
        );
    }
}
#[test]
fn requested_routes_do_not_overwrite_or_relabel_unfinished_drafts() {
    let mut a = app();
    let copy = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let draft = drawing(&copy, false);
    a.handwriting_store_draft(&copy.scope(), &copy.task(), &draft)
        .unwrap();
    let task = copy.definition().unwrap();
    a.open_writing_task(&task.id, Sequence::HearWriteCompareSay)
        .unwrap();
    let hear = a.active_handwriting().unwrap();
    assert_ne!(hear.id, copy.id);
    assert_eq!(hear.sequence, Sequence::HearWriteCompareSay);
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap().id, copy.id);
    assert_eq!(
        a.handwriting_draft(&copy.scope(), &copy.task()).unwrap(),
        Some(draft)
    );
}
#[test]
fn fresh_round_and_skipped_speaking_preserve_prior_original_and_honest_disposition() {
    let mut a = app();
    let run = begin(&mut a, "f3-message", Sequence::SeeCopyHearSay);
    let original = drawing(&run, true);
    save(&mut a, &run, &original);
    a.writing_go_say(run.id, run.round_id).unwrap();
    a.writing_say_done(run.id, run.round_id, SayDisposition::Skipped)
        .unwrap();
    a.writing_new_round(run.id, run.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let fresh = a.active_handwriting().unwrap();
    assert_ne!(fresh.scope(), run.scope());
    assert!(fresh.original.is_none());
    assert!(!fresh.audio_completed);
    assert_eq!(fresh.history[0].say, SayDisposition::Skipped);
    assert_eq!(
        a.handwriting_originals(&run.scope(), &run.task()).unwrap(),
        vec![original]
    );
}

#[test]
fn fresh_catalog_navigation_starts_course_and_first_task_without_an_active_run() {
    let mut a = app();
    a.open_writing_catalog().unwrap();
    assert!(a.active_handwriting().is_err());
    assert!(!a.writing_navigation_needs_flush());
    a.start_foundations().unwrap();
    a.foundation_replayed("foundation-a").unwrap();
    a.open_writing_catalog().unwrap();
    assert!(!a.writing_navigation_needs_flush());
    let task = handwriting_task_for_step("f1-a").unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    assert!(a.writing_navigation_needs_flush());
    a.open_writing_catalog().unwrap();
    assert!(
        !a.writing_navigation_needs_flush(),
        "catalogwithastaleactivepointerhasnolivemountedpaper"
    );
    a.navigate(Page::Today).unwrap();
    assert!(a.page == Page::Today);
}

#[test]
fn completed_changed_mode_reopens_requested_course_route_without_losing_originals() {
    let mut a = app();
    let copy = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let first = drawing(&copy, true);
    save(&mut a, &copy, &first);
    a.writing_go_say(copy.id, copy.round_id).unwrap();
    a.writing_say_done(copy.id, copy.round_id, SayDisposition::Skipped)
        .unwrap();
    a.writing_new_round(copy.id, copy.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let hear = a.active_handwriting().unwrap();
    a.writing_audio_completed(hear.id, hear.round_id, &hear.definition().unwrap().audio_id)
        .unwrap();
    let second = drawing(&hear, true);
    save(&mut a, &hear, &second);
    a.writing_go_say(hear.id, hear.round_id).unwrap();
    a.writing_say_done(hear.id, hear.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    a.navigate(Page::Foundations).unwrap();
    a.open_handwriting().unwrap();
    let reopened = a.active_handwriting().unwrap();
    assert_eq!(reopened.sequence, Sequence::SeeCopyHearSay);
    assert_eq!(reopened.stage, WritingStage::Writing);
    assert_ne!(reopened.round_id, hear.round_id);
    assert_eq!(reopened.history.len(), 2);
    assert_eq!(
        a.handwriting_originals(&copy.scope(), &copy.task())
            .unwrap(),
        vec![first]
    );
    assert_eq!(
        a.handwriting_originals(&hear.scope(), &hear.task())
            .unwrap(),
        vec![second]
    );
}

#[test]
fn unfinished_changed_route_reentry_preserves_both_exact_drafts() {
    let mut a = app();
    let copy = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let original = drawing(&copy, true);
    save(&mut a, &copy, &original);
    a.writing_new_round(copy.id, copy.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let hear = a.active_handwriting().unwrap();
    let hear_draft = drawing(&hear, false);
    a.handwriting_store_draft(&hear.scope(), &hear.task(), &hear_draft)
        .unwrap();
    a.open_writing_catalog().unwrap();
    a.open_writing_task(&copy.task_id, Sequence::SeeCopyHearSay)
        .unwrap();
    let other_copy = a.active_handwriting().unwrap();
    assert_ne!(other_copy.id, hear.id);
    assert_eq!(other_copy.sequence, Sequence::SeeCopyHearSay);
    let copy_draft = drawing(&other_copy, false);
    a.handwriting_store_draft(&other_copy.scope(), &other_copy.task(), &copy_draft)
        .unwrap();
    a.open_writing_task(&copy.task_id, Sequence::HearWriteCompareSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap(), hear);
    assert_eq!(
        a.handwriting_draft(&hear.scope(), &hear.task()).unwrap(),
        Some(hear_draft)
    );
    a.open_writing_task(&copy.task_id, Sequence::SeeCopyHearSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap(), other_copy);
    assert_eq!(
        a.handwriting_draft(&other_copy.scope(), &other_copy.task())
            .unwrap(),
        Some(copy_draft)
    );
    let review = a.handwriting_round_review(copy.id, copy.round_id).unwrap();
    assert_eq!(review.sequence, Sequence::SeeCopyHearSay);
    assert_eq!(review.stage, Some(WritingStage::Compare));
    assert_eq!(review.task.audio_role, InkAudioRole::Model);
    assert_eq!(
        review.originals[0].payload,
        serde_json::to_value(original).unwrap()
    );
    assert_eq!(a.library.attempts().count(), 0);
}

#[test]
fn history_reopens_old_session_exactly_without_fresh_attempt_or_course_movement() {
    let mut a = app();
    let first = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    save(&mut a, &first, &drawing(&first, true));
    a.writing_go_say(first.id, first.round_id).unwrap();
    a.writing_say_done(first.id, first.round_id, SayDisposition::Skipped)
        .unwrap();
    let completed = a.active_handwriting().unwrap();
    let current = begin(&mut a, "f3-message", Sequence::HearWriteCompareSay);
    let draft = drawing(&current, false);
    a.handwriting_store_draft(&current.scope(), &current.task(), &draft)
        .unwrap();
    let cursor = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    let mut reloaded = AppState::from_repository(a.repo.clone()).unwrap();
    assert_eq!(reloaded.handwriting_history().unwrap().len(), 2);
    reloaded.open_handwriting_history(first.id).unwrap();
    assert_eq!(reloaded.active_handwriting().unwrap(), completed);
    assert_eq!(
        serde_json::to_value(reloaded.foundation_state().unwrap()).unwrap(),
        cursor
    );
    let review = reloaded
        .handwriting_round_review(first.id, first.round_id)
        .unwrap();
    assert_eq!(review.original, completed.original);
    assert_eq!(review.stage, Some(WritingStage::Complete));
    assert_eq!(
        reloaded.active_handwriting().unwrap(),
        completed,
        "review must be read-only"
    );
    reloaded.open_handwriting_history(current.id).unwrap();
    assert_eq!(reloaded.active_handwriting().unwrap(), current);
    assert_eq!(
        reloaded
            .handwriting_draft(&current.scope(), &current.task())
            .unwrap(),
        Some(draft)
    );
    assert_eq!(reloaded.library.attempts().count(), 0);
}

#[test]
fn canonical_history_recovers_interrupted_run_and_original_index_writes() {
    let mut a = app();
    let run = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    // Simulate an activity persisted immediately before its course/active pointers.
    let mut orphan = run.clone();
    orphan.id = Uuid::new_v4();
    orphan.round_id = Uuid::new_v4();
    orphan.attempt_id = Uuid::new_v4();
    orphan.scope_id = format!("{}:{}", orphan.id, orphan.round_id);
    a.repo
        .set(&format!("handwriting.activity:{}", orphan.id), &orphan)
        .unwrap();
    let original = drawing(&orphan, true);
    // Simulate raw original persisted immediately before the originals index and
    // generic artifact/accepted-reference writes.
    a.repo
        .set(
            &format!("handwriting.original:{}", original.attempt_id),
            &original,
        )
        .unwrap();
    let mut reloaded = AppState::from_repository(a.repo.clone()).unwrap();
    assert!(
        reloaded
            .handwriting_history()
            .unwrap()
            .iter()
            .any(|r| r.id == orphan.id)
    );
    reloaded.open_handwriting_history(orphan.id).unwrap();
    assert_eq!(reloaded.active_handwriting().unwrap(), orphan);
    let review = reloaded
        .handwriting_round_review(orphan.id, orphan.round_id)
        .unwrap();
    assert!(
        review.original.is_none(),
        "durable bytes do not invent accepted comparison"
    );
    assert_eq!(review.stage, Some(WritingStage::Writing));
    assert_eq!(review.originals.len(), 1);
    assert_eq!(
        review.originals[0].payload,
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(
        reloaded
            .handwriting_originals(&orphan.scope(), &orphan.task())
            .unwrap(),
        vec![original]
    );
}

#[test]
fn round_review_recovers_competing_unindexed_artifacts_without_selecting_them() {
    let mut a = app();
    let run = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    save(&mut a, &run, &drawing(&run, true));
    let accepted = a.active_handwriting().unwrap().original.unwrap();
    let competing = a.writing_store_artifact(Uuid::new_v4(), &run.scope(), &run.task().id,
        &run.task_revision, "apple-pencilkit-control-points-v1",
        serde_json::json!({"checkpoint_id":Uuid::new_v4(), "archive":"preserved-original-bytes"})).unwrap();
    let review = a.handwriting_round_review(run.id, run.round_id).unwrap();
    assert_eq!(review.original, Some(accepted.clone()));
    assert_eq!(review.originals.len(), 2);
    assert!(review.originals.iter().any(|o| o.reference == competing));
    assert_eq!(a.active_handwriting().unwrap().original, Some(accepted));
    assert_eq!(a.library.attempts().count(), 0);
}

#[test]
fn legacy_archived_round_keeps_unknown_stage_and_exact_original_identity() {
    let mut a = app();
    let run = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    save(&mut a, &run, &drawing(&run, true));
    a.writing_new_round(run.id, run.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let current = a.active_handwriting().unwrap();
    let mut legacy = serde_json::to_value(&current).unwrap();
    for field in ["stage", "attempt_id", "audio_completed"] {
        legacy["history"][0].as_object_mut().unwrap().remove(field);
    }
    a.repo
        .set(&format!("handwriting.activity:{}", current.id), &legacy)
        .unwrap();
    let review = a.handwriting_round_review(run.id, run.round_id).unwrap();
    assert_eq!(review.stage, None);
    assert_eq!(review.sequence, Sequence::SeeCopyHearSay);
    assert_eq!(review.original, current.history[0].original);
    assert_eq!(a.active_handwriting().unwrap().round_id, current.round_id);
}

#[test]
fn mismatched_legacy_route_pointer_is_rejected_without_replacing_active_run() {
    let mut a = app();
    let first = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let second = begin(&mut a, "f3-message", Sequence::SeeCopyHearSay);
    a.repo
        .set(
            &format!(
                "handwriting.course:{}:{}:copy",
                second.course_session, second.task_id
            ),
            &first.id,
        )
        .unwrap();
    assert!(
        a.open_writing_task(&second.task_id, Sequence::SeeCopyHearSay)
            .is_err()
    );
    assert_eq!(a.active_handwriting().unwrap(), second);
    assert_eq!(a.handwriting_history().unwrap().len(), 2);
}

#[test]
fn review_rejects_changed_artifact_bytes_without_changing_the_live_round() {
    let mut a = app();
    let run = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    save(&mut a, &run, &drawing(&run, true));
    let current = a.active_handwriting().unwrap();
    let reference = current.original.as_ref().unwrap();
    let mut artifact: serde_json::Value = a.repo.get(&reference.local_key).unwrap().unwrap();
    artifact["payload"]["strokes"][0]["points"][0]["pressure"] = serde_json::json!(0.25);
    a.repo.set(&reference.local_key, &artifact).unwrap();
    assert!(a.handwriting_round_review(run.id, run.round_id).is_err());
    assert_eq!(a.active_handwriting().unwrap(), current);
    assert_eq!(a.library.attempts().count(), 0);
}

#[test]
fn exporting_nonactive_run_preserves_current_work_and_unindexed_competing_originals() {
    let mut a = app();
    let first = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let ink = drawing(&first, true);
    save(&mut a, &first, &ink);
    let competitor_payload =
        serde_json::json!({"checkpoint_id":Uuid::new_v4(), "archive":"exact-original-archive"});
    let competing = a
        .writing_store_artifact(
            Uuid::new_v4(),
            &first.scope(),
            &first.task().id,
            &first.task_revision,
            "apple-pencilkit-control-points-v1",
            competitor_payload.clone(),
        )
        .unwrap();
    let latest = serde_json::json!({"scope_id":first.scope(), "checkpoint_id":competing.id, "archive":"exact-latest-draft"});
    a.repo
        .set(
            &format!("handwriting.native.latest:{}", first.scope()),
            &latest,
        )
        .unwrap();
    a.writing_new_round(first.id, first.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let second = a.active_handwriting().unwrap();
    // The new round's raw web original reached disk before either index write.
    let unindexed = drawing(&second, true);
    a.repo
        .set(
            &format!("handwriting.original:{}", unindexed.attempt_id),
            &unindexed,
        )
        .unwrap();
    let current = begin(&mut a, "f3-message", Sequence::HearWriteCompareSay);
    let current_draft = drawing(&current, false);
    a.handwriting_store_draft(&current.scope(), &current.task(), &current_draft)
        .unwrap();
    let course_before = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    let export = a.handwriting_export_run(first.id).unwrap();
    assert_eq!(export["run"], serde_json::to_value(&second).unwrap());
    let originals = export["originals"].as_array().unwrap();
    assert_eq!(originals.len(), 3);
    let competitor = originals
        .iter()
        .find(|v| v["id"] == serde_json::json!(competing.id))
        .unwrap();
    assert_eq!(competitor["scope_id"], competing.scope_id);
    assert_eq!(competitor["task_id"], competing.task_id);
    assert_eq!(competitor["task_revision"], competing.task_revision);
    assert_eq!(competitor["format"], competing.format);
    assert_eq!(competitor["sha256"], competing.sha256);
    assert_eq!(competitor["payload"], competitor_payload);
    assert!(
        originals
            .iter()
            .any(|v| v["payload"] == serde_json::to_value(&ink).unwrap())
    );
    assert!(
        originals
            .iter()
            .any(|v| v["payload"] == serde_json::to_value(&unindexed).unwrap())
    );
    assert_eq!(export["latest_native_drafts"], serde_json::json!([latest]));
    assert_eq!(a.active_handwriting().unwrap(), current);
    assert_eq!(
        serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
        course_before
    );
    assert_eq!(
        a.handwriting_draft(&current.scope(), &current.task())
            .unwrap(),
        Some(current_draft)
    );
    assert_eq!(a.library.attempts().count(), 0);
}

#[test]
fn duplicate_unfinished_routes_follow_active_then_explicit_preference() {
    let mut a = app();
    let copy = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    a.open_writing_task(&copy.task_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let hear = a.active_handwriting().unwrap();
    a.writing_audio_completed(hear.id, hear.round_id, &hear.definition().unwrap().audio_id)
        .unwrap();
    let hear_original = drawing(&hear, true);
    save(&mut a, &hear, &hear_original);
    a.writing_go_say(hear.id, hear.round_id).unwrap();
    a.writing_say_done(hear.id, hear.round_id, SayDisposition::Skipped)
        .unwrap();
    a.writing_new_round(hear.id, hear.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let hear_fresh = a.active_handwriting().unwrap();
    let hear_draft = drawing(&hear_fresh, false);
    a.handwriting_store_draft(&hear_fresh.scope(), &hear_fresh.task(), &hear_draft)
        .unwrap();

    a.open_handwriting_history(copy.id).unwrap();
    let copy_original = drawing(&copy, true);
    save(&mut a, &copy, &copy_original);
    a.writing_go_say(copy.id, copy.round_id).unwrap();
    a.writing_say_done(copy.id, copy.round_id, SayDisposition::Skipped)
        .unwrap();
    a.writing_new_round(copy.id, copy.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let changed = a.active_handwriting().unwrap();
    let changed_draft = drawing(&changed, false);
    a.handwriting_store_draft(&changed.scope(), &changed.task(), &changed_draft)
        .unwrap();
    let route_key = format!(
        "handwriting.course:{}:{}:sound",
        copy.course_session, copy.task_id
    );
    assert_eq!(a.repo.get::<Uuid>(&route_key).unwrap(), Some(copy.id));

    // Even a stale preference must not displace a matching active draft.
    a.repo.set(&route_key, &hear.id).unwrap();
    a.open_writing_task(&copy.task_id, Sequence::HearWriteCompareSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap(), changed);

    // Switch to another route so route reentry must consult the saved preference.
    a.open_writing_task(&copy.task_id, Sequence::SeeCopyHearSay)
        .unwrap();
    let third = a.active_handwriting().unwrap();
    assert_ne!(third.id, changed.id);
    a.open_writing_task(&copy.task_id, Sequence::HearWriteCompareSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap(), changed);

    // Explicit history resume changes that preference; UUID ordering cannot
    // choose the same run on both of these otherwise identical route selections.
    a.open_handwriting_history(hear.id).unwrap();
    assert_eq!(a.repo.get::<Uuid>(&route_key).unwrap(), Some(hear.id));
    a.open_writing_task(&copy.task_id, Sequence::SeeCopyHearSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap(), third);
    a.open_writing_task(&copy.task_id, Sequence::HearWriteCompareSay)
        .unwrap();
    assert_eq!(a.active_handwriting().unwrap(), hear_fresh);
    assert_eq!(
        a.handwriting_draft(&changed.scope(), &changed.task())
            .unwrap(),
        Some(changed_draft)
    );
    assert_eq!(
        a.handwriting_draft(&hear_fresh.scope(), &hear_fresh.task())
            .unwrap(),
        Some(hear_draft)
    );
    assert_eq!(
        a.handwriting_originals(&copy.scope(), &copy.task())
            .unwrap(),
        vec![copy_original]
    );
    assert_eq!(
        a.handwriting_originals(&hear.scope(), &hear.task())
            .unwrap(),
        vec![hear_original]
    );
    assert_eq!(a.library.attempts().count(), 0);
}
