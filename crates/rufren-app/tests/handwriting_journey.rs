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
#[path = "../src/foundation_writing.rs"]
mod foundation_writing;
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
    let task = handwriting_task_for_step(&a.foundation_state().unwrap().step_id).unwrap();
    assert!(
        a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
            .is_err()
    );
    let unheard = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    a.open_handwriting().unwrap();
    assert!(a.page == Page::Foundations);
    assert!(!a.active_handwriting().unwrap().audio_completed);
    assert_eq!(
        serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
        unheard
    );
    a.foundation_replayed("foundation-a").unwrap();
    let before = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    a.open_handwriting().unwrap();
    assert!(a.page == Page::Foundations);
    let run = a.active_handwriting().unwrap();
    assert_eq!(run.definition().unwrap().target, "А");
    assert_eq!(
        serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
        before
    );
    assert_eq!(a.library.attempts().count(), 0);
}
#[test]
fn copy_continues_completed_course_model_without_inventing_exposure() {
    let mut a = app();
    source(&mut a, "f2-cat");
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.foundation_replayed(&task.audio_id).unwrap();
    let events = a.library.events.len();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let run = a.active_handwriting().unwrap();
    assert!(run.audio_completed);
    assert_eq!(a.library.events.len(), events);
    save(&mut a, &run, &drawing(&run, true));
    a.writing_go_say(run.id, run.round_id).unwrap();
    a.writing_say_done(run.id, run.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    a.writing_new_round(run.id, run.round_id, Sequence::SeeCopyHearSay)
        .unwrap();
    let fresh = a.active_handwriting().unwrap();
    assert!(!fresh.audio_completed);
    assert_eq!(fresh.history[0].audio_completed, Some(true));
}
#[test]
fn copy_does_not_reuse_unheard_foreign_or_catalog_listening() {
    for case in ["unheard", "asset", "step", "catalog"] {
        let mut a = app();
        source(&mut a, "f2-cat");
        let task = handwriting_task_for_step("f2-cat").unwrap();
        let mut state = a.foundation_state().unwrap();
        state.played.clear();
        a.foundation_save(&state).unwrap();
        if case != "unheard" {
            a.foundation_replayed(if case == "asset" {
                "foundation-a"
            } else {
                &task.audio_id
            })
            .unwrap();
        }
        if case == "step" {
            let mut state = a.foundation_state().unwrap();
            state
                .encounters
                .push(format!("{}:encountered", task.foundation_step_id));
            let pack = rufren_content::foundations();
            let step = &pack.episodes[0].steps[0];
            state.episode = 0;
            state.step = 0;
            state.episode_id = pack.episodes[0].id.clone();
            state.step_id = step.id.clone();
            state.step_revision = step.revision.clone();
            a.foundation_save(&state).unwrap();
        }
        // Encounter eligibility alone is not listening completion.
        let mut state = a.foundation_state().unwrap();
        state
            .encounters
            .push(format!("{}:encountered", task.foundation_step_id));
        a.foundation_save(&state).unwrap();
        if case == "catalog" {
            a.open_writing_catalog().unwrap();
        }
        a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
            .unwrap();
        assert!(!a.active_handwriting().unwrap().audio_completed, "{case}");
    }
}
#[test]
fn invalid_course_revision_cannot_supply_copy_listening() {
    let mut a = app();
    source(&mut a, "f2-cat");
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.foundation_replayed(&task.audio_id).unwrap();
    let mut state = a.foundation_state().unwrap();
    state.step_revision = "obsolete-step-revision".into();
    a.foundation_save(&state).unwrap();
    assert!(
        a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
            .is_err()
    );
    assert!(a.active_handwriting().is_err());
}
#[test]
fn course_entry_ignores_stale_catalog_surface() {
    let mut a = app();
    a.open_writing_catalog().unwrap();
    source(&mut a, "f2-cat");
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.foundation_replayed(&task.audio_id).unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let run = a.active_handwriting().unwrap();
    assert!(!run.entry_previewed);
    assert!(run.audio_completed);
    save(&mut a, &run, &drawing(&run, true));
    a.writing_finish_and_continue(run.id, run.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    assert!(a.page == Page::Foundations);
    assert_ne!(a.foundation_state().unwrap().session, run.course_session);
}
#[test]
fn a_new_course_session_does_not_inherit_the_previous_runs_listening() {
    let mut a = app();
    source(&mut a, "f2-cat");
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.foundation_replayed(&task.audio_id).unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let first = a.active_handwriting().unwrap();
    assert!(first.audio_completed);
    source(&mut a, "f2-cat");
    let mut state = a.foundation_state().unwrap();
    state.played.clear();
    state.encounters.push("f2-cat:encountered".into());
    a.foundation_save(&state).unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let next = a.active_handwriting().unwrap();
    assert_ne!(next.course_session, first.course_session);
    assert!(!next.audio_completed);
    assert!(
        a.handwriting_history()
            .unwrap()
            .iter()
            .find(|r| r.id == first.id)
            .unwrap()
            .audio_completed
    );
}
#[test]
fn dictation_requires_fresh_input_even_after_matching_course_completion() {
    let mut a = app();
    source(&mut a, "f2-cat");
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.foundation_replayed(&task.audio_id).unwrap();
    a.open_writing_task(&task.id, Sequence::HearWriteCompareSay)
        .unwrap();
    let run = a.active_handwriting().unwrap();
    assert!(!run.audio_completed);
    assert!(
        a.writing_accept_web(run.id, run.round_id, &drawing(&run, true))
            .is_err()
    );
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
        let mut expected = a.foundation_state().unwrap();
        // The real completed playback records replay support, while the
        // course cursor and all learning evidence remain unchanged.
        if !expected.cues.contains(&rufren_core::Cue::Replay) {
            expected.cues.push(rufren_core::Cue::Replay);
        }
        let before = serde_json::to_value(expected).unwrap();
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
    let mut hear = a.active_handwriting().unwrap();
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
    hear.return_intent = Some(WritingReturnIntent::Catalog);
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
    let mut completed = a.active_handwriting().unwrap();
    let mut current = begin(&mut a, "f3-message", Sequence::HearWriteCompareSay);
    let draft = drawing(&current, false);
    a.handwriting_store_draft(&current.scope(), &current.task(), &draft)
        .unwrap();
    let cursor = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    let mut reloaded = AppState::from_repository(a.repo.clone()).unwrap();
    assert_eq!(reloaded.handwriting_history().unwrap().len(), 2);
    reloaded.open_handwriting_history(first.id).unwrap();
    completed.return_intent = Some(WritingReturnIntent::Catalog);
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
    current.return_intent = Some(WritingReturnIntent::Catalog);
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
    let mut expected = orphan.clone();
    expected.return_intent = Some(WritingReturnIntent::Catalog);
    assert_eq!(reloaded.active_handwriting().unwrap(), expected);
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

#[test]
fn finish_inline_writing_advances_its_course_once_and_keeps_original() {
    let mut a = app();
    let run = begin(&mut a, "f2-cat", Sequence::SeeCopyHearSay);
    let ink = drawing(&run, true);
    a.writing_accept_web(run.id, run.round_id, &ink).unwrap();
    a.writing_audio_completed(run.id, run.round_id, &run.definition().unwrap().audio_id)
        .unwrap();
    let original = a.active_handwriting().unwrap().original;
    a.writing_finish_and_continue(run.id, run.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    let next = a.foundation_state().unwrap();
    assert_ne!(next.session, run.course_session);
    assert_ne!(next.step_id, run.course_step);
    assert!(a.page == Page::Foundations);
    assert_eq!(a.active_handwriting().unwrap().original, original);
    assert_eq!(
        a.active_handwriting().unwrap().say,
        SayDisposition::SelfReportedSaid
    );
    a.writing_finish_and_continue(run.id, run.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    assert_eq!(a.foundation_state().unwrap().session, next.session);
    assert_eq!(a.library.attempts().count(), 0);
}

#[test]
fn finish_catalog_writing_returns_to_catalog_without_advancing_course() {
    let mut a = app();
    source(&mut a, "f2-cat");
    a.open_writing_catalog().unwrap();
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let run = a.active_handwriting().unwrap();
    let cursor = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    a.writing_accept_web(run.id, run.round_id, &drawing(&run, true))
        .unwrap();
    a.writing_finish_and_continue(run.id, run.round_id, SayDisposition::Skipped)
        .unwrap();
    assert_eq!(
        a.repo
            .get::<String>("handwriting.surface")
            .unwrap()
            .as_deref(),
        Some("catalog")
    );
    assert_eq!(
        serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
        cursor
    );
    assert_eq!(a.active_handwriting().unwrap().say, SayDisposition::Skipped);
}

#[test]
fn continuous_lesson_prepares_paper_before_audio_and_restores_exact_round() {
    let mut a = app();
    source(&mut a, "f1-a");
    a.navigate(Page::Foundations).unwrap();
    let mut course = a.foundation_state().unwrap();
    course.played.clear();
    a.foundation_save(&course).unwrap();
    let run = a.prepare_foundation_writing().unwrap().unwrap();
    assert!(a.page == Page::Foundations);
    assert!(!run.audio_completed);
    assert!(a.foundation_state().unwrap().played.is_empty());
    assert!(
        !a.handwriting_task_available(&run.definition().unwrap())
            .unwrap()
    );
    let restored = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(restored.id, run.id);
    assert_eq!(restored.round_id, run.round_id);
    assert!(a.writing_navigation_needs_flush());
    a.writing_audio_completed(run.id, run.round_id, &run.definition().unwrap().audio_id)
        .unwrap();
    assert!(
        a.foundation_state()
            .unwrap()
            .played
            .contains(&run.definition().unwrap().audio_id)
    );
    a.resume_handwriting().unwrap();
    assert!(a.page == Page::Foundations);
}

#[test]
fn continuous_lesson_rebinds_catalog_intent_without_replacing_run() {
    let mut a = app();
    source(&mut a, "f1-a");
    a.open_writing_catalog().unwrap();
    let task = handwriting_task_for_step("f1-a").unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let catalog = a.active_handwriting().unwrap();
    assert!(matches!(
        catalog.return_intent,
        Some(WritingReturnIntent::Catalog)
    ));
    a.navigate(Page::Foundations).unwrap();
    let bound = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(bound.id, catalog.id);
    assert_eq!(bound.round_id, catalog.round_id);
    assert_eq!(bound.course_step, catalog.course_step);
    assert_eq!(bound.entry_previewed, catalog.entry_previewed);
    assert!(bound.audio_completed);
    a.open_handwriting_history(bound.id).unwrap();
    assert!(a.page == Page::Handwriting);
    assert!(matches!(
        a.active_handwriting().unwrap().return_intent,
        Some(WritingReturnIntent::Catalog)
    ));
}

#[test]
fn continuous_lesson_preparation_does_not_offer_paper_on_unrelated_actions() {
    let mut a = app();
    source(&mut a, "c5-keyboard");
    assert!(a.prepare_foundation_writing().unwrap().is_none());
}

#[test]
fn continuous_lesson_complete_is_restored_and_fresh_round_keeps_return_intent() {
    let mut a = app();
    source(&mut a, "f2-cat");
    a.navigate(Page::Foundations).unwrap();
    let run = a.prepare_foundation_writing().unwrap().unwrap();
    a.writing_accept_web(run.id, run.round_id, &drawing(&run, true))
        .unwrap();
    a.writing_go_say(run.id, run.round_id).unwrap();
    a.writing_say_done(run.id, run.round_id, SayDisposition::Skipped)
        .unwrap();
    let completed = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(completed.id, run.id);
    assert_eq!(completed.round_id, run.round_id);
    assert_eq!(completed.stage, WritingStage::Complete);
    let original = completed.original.clone();
    a.writing_new_round(run.id, run.round_id, Sequence::SeeCopyHearSay)
        .unwrap();
    let fresh = a.active_handwriting().unwrap();
    assert!(fresh.entry_previewed);
    assert_eq!(fresh.return_intent, completed.return_intent);
    assert_eq!(fresh.history[0].original, original);
    a.writing_accept_web(fresh.id, fresh.round_id, &drawing(&fresh, true))
        .unwrap();
    a.writing_finish_and_continue(fresh.id, fresh.round_id, SayDisposition::Skipped)
        .unwrap();
    assert!(a.page == Page::Foundations);
    let after = a.foundation_state().unwrap();
    a.writing_finish_and_continue(fresh.id, fresh.round_id, SayDisposition::Skipped)
        .unwrap();
    assert!(a.page == Page::Foundations);
    assert_eq!(a.foundation_state().unwrap().session, after.session);
}

#[test]
fn completed_catalog_round_keeps_its_original_unheard_evidence_when_embedded() {
    let mut a = app();
    source(&mut a, "f2-cat");
    a.open_writing_catalog().unwrap();
    let task = handwriting_task_for_step("f2-cat").unwrap();
    a.open_writing_task(&task.id, Sequence::SeeCopyHearSay)
        .unwrap();
    let run = a.active_handwriting().unwrap();
    assert!(!run.audio_completed);
    save(&mut a, &run, &drawing(&run, true));
    a.writing_go_say(run.id, run.round_id).unwrap();
    a.writing_say_done(run.id, run.round_id, SayDisposition::Skipped)
        .unwrap();
    let completed = a.active_handwriting().unwrap();
    let original_bytes = a.handwriting_export_run(run.id).unwrap()["originals"].clone();
    a.navigate(Page::Foundations).unwrap();
    let mut restored = a.prepare_foundation_writing().unwrap().unwrap();
    assert!(matches!(
        restored.return_intent,
        Some(WritingReturnIntent::Foundations { .. })
    ));
    restored.return_intent = completed.return_intent.clone();
    assert_eq!(restored, completed);
    assert_eq!(
        a.handwriting_export_run(run.id).unwrap()["originals"],
        original_bytes
    );
}

fn start_checkpoint(
    a: &mut AppState,
    anchor: &str,
) -> foundation_writing::FoundationWritingCheckpointView {
    source(a, anchor);
    let state = a.foundation_state().unwrap();
    let step = rufren_content::foundations().episodes[state.episode].steps[state.step].clone();
    match &step.action {
        rufren_content::FoundationAction::Choose { correct_id, .. } => {
            a.foundation_edit(vec![correct_id.clone()]).unwrap();
            a.foundation_submit(&step).unwrap();
        }
        rufren_content::FoundationAction::Read { .. } => a.foundation_read_try(&step).unwrap(),
        _ => panic!("fixture needs an actual response at this anchor"),
    }
    a.foundation_next(false).unwrap();
    a.foundation_writing_checkpoint().unwrap().unwrap()
}
fn finish_checkpoint_at(a: &mut AppState, now: i64) -> HandwritingActivity {
    let checkpoint = a.foundation_writing_checkpoint().unwrap().unwrap();
    let run = a.prepare_foundation_writing().unwrap().unwrap();
    a.writing_audio_completed(run.id, run.round_id, &run.definition().unwrap().audio_id)
        .unwrap();
    let ink = drawing(&run, true);
    save(a, &run, &ink);
    a.writing_go_say(run.id, run.round_id).unwrap();
    a.writing_say_done(run.id, run.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    a.foundation_finish_writing_checkpoint_at(checkpoint.item_session, false, now)
        .unwrap();
    a.writing_load(run.id).unwrap()
}

#[test]
fn continuous_recall_preserves_anchor_ink_and_advances_each_item_once() {
    let mut a = app();
    let checkpoint = start_checkpoint(&mut a, "f1-radio-no");
    assert_eq!((checkpoint.index, checkpoint.count), (0, 2));
    assert_eq!(checkpoint.task_id, "foundation-hand-print-ma");
    assert_eq!(a.foundation_state().unwrap().step_id, "f1-radio-no");
    let before_course = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    let run = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(run.sequence, Sequence::HearWriteCompareSay);
    assert!(!run.audio_completed && !run.model_revealed);
    assert_eq!(run.pre_submission_cues, Some(vec![]));
    let draft = drawing(&run, false);
    a.handwriting_store_draft(&run.scope(), &run.task(), &draft)
        .unwrap();
    a.navigate(Page::Today).unwrap();
    let mut resumed = AppState::from_repository(a.repo.clone()).unwrap();
    resumed.start_foundations().unwrap();
    let same = resumed.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(same.id, run.id);
    assert_eq!(same.round_id, run.round_id);
    assert_eq!(
        serde_json::to_value(resumed.foundation_state().unwrap()).unwrap(),
        before_course
    );
    assert_eq!(
        resumed
            .handwriting_draft(&run.scope(), &run.task())
            .unwrap(),
        Some(draft)
    );
    resumed
        .writing_audio_completed(run.id, run.round_id, "foundation-ma")
        .unwrap();
    assert_eq!(
        serde_json::to_value(resumed.foundation_state().unwrap()).unwrap(),
        before_course
    );
    save(&mut resumed, &run, &drawing(&run, true));
    resumed
        .writing_finish_and_continue(run.id, run.round_id, SayDisposition::SelfReportedSaid)
        .unwrap();
    let second = resumed.foundation_writing_checkpoint().unwrap().unwrap();
    assert_eq!(second.task_id, "foundation-hand-print-am");
    assert_eq!(second.index, 1);
    assert_ne!(second.item_session, checkpoint.item_session);
    // Duplicate old continuation must not skip the next listening turn.
    resumed
        .writing_finish_and_continue(run.id, run.round_id, SayDisposition::Skipped)
        .unwrap();
    assert_eq!(
        resumed.foundation_writing_checkpoint().unwrap().unwrap(),
        second
    );
    assert!(
        resumed
            .foundation_skip_writing_checkpoint(checkpoint.item_session)
            .is_err()
    );
    resumed
        .foundation_skip_writing_checkpoint(second.item_session)
        .unwrap();
    assert!(resumed.foundation_writing_checkpoint().unwrap().is_none());
    assert_eq!(resumed.foundation_state().unwrap().step_id, "f1-end");
    assert_eq!(
        resumed
            .handwriting_originals(&run.scope(), &run.task())
            .unwrap()
            .len(),
        1
    );
    let visits = resumed
        .repo
        .metadata_with_prefix("foundation.writing.visit:")
        .unwrap();
    assert_eq!(visits.len(), 2);
    assert_eq!(
        visits.iter().filter(|(_, v)| v["skipped"] == true).count(),
        1
    );
    assert_eq!(
        visits
            .iter()
            .map(|(_, v)| v["originals"].as_array().unwrap().len())
            .sum::<usize>(),
        1
    );
}

#[test]
fn continuous_recall_skipped_source_is_not_silently_taught_or_queued() {
    let mut a = app();
    source(&mut a, "f1-radio-no");
    let mut state = a.foundation_state().unwrap();
    state.encounters.retain(|e| e != "f1-model-ma:encountered");
    state.encounters.push("f1-model-ma:skipped".into());
    a.foundation_save(&state).unwrap();
    let step = rufren_content::foundations().episodes[state.episode].steps[state.step].clone();
    let rufren_content::FoundationAction::Choose { correct_id, .. } = &step.action else {
        panic!()
    };
    a.foundation_edit(vec![correct_id.clone()]).unwrap();
    a.foundation_submit(&step).unwrap();
    a.foundation_next(false).unwrap();
    let checkpoint = a.foundation_writing_checkpoint().unwrap().unwrap();
    assert_eq!(checkpoint.count, 1);
    assert_eq!(checkpoint.task_id, "foundation-hand-print-am");
    a.foundation_skip_writing_checkpoint(checkpoint.item_session)
        .unwrap();
    assert!(
        a.foundation_writing_returns(repository::now_ms() + 2 * rufren_core::DAY_MS)
            .is_empty()
    );
}

#[test]
fn continuous_recall_repair_keeps_original_and_assistance_chronology() {
    let mut a = app();
    start_checkpoint(&mut a, "f2-radio-no");
    let first = a.prepare_foundation_writing().unwrap().unwrap();
    a.writing_audio_completed(first.id, first.round_id, "foundation-kot")
        .unwrap();
    let mut original = drawing(&first, true);
    original.aids.push(InkAid {
        kind: "visual_model".into(),
        value: "copy".into(),
        at_ms: 123500,
    });
    save(&mut a, &first, &original);
    assert_eq!(
        a.active_handwriting().unwrap().pre_submission_cues,
        Some(vec!["visual_model".into()])
    );
    a.writing_new_round(first.id, first.round_id, Sequence::SeeCopyHearSay)
        .unwrap();
    let supported = a.active_handwriting().unwrap();
    assert_eq!(
        supported.history[0].pre_submission_cues,
        Some(vec!["visual_model".into()])
    );
    assert_eq!(
        supported.history[0].original.as_ref().unwrap().id,
        first.attempt_id
    );
    assert!(
        supported
            .pre_submission_cues
            .as_ref()
            .unwrap()
            .contains(&"after_comparison".into())
    );
    assert_eq!(supported.return_intent, first.return_intent);
    a.writing_audio_completed(supported.id, supported.round_id, "foundation-kot")
        .unwrap();
    save(&mut a, &supported, &drawing(&supported, true));
    a.writing_new_round(
        supported.id,
        supported.round_id,
        Sequence::HearWriteCompareSay,
    )
    .unwrap();
    let retry = a.active_handwriting().unwrap();
    assert_eq!(retry.sequence, Sequence::HearWriteCompareSay);
    assert!(!retry.model_revealed && !retry.audio_completed);
    assert_eq!(
        retry.pre_submission_cues,
        Some(vec!["after_comparison".into()])
    );
    assert_eq!(retry.history.len(), 2);
    assert_eq!(
        a.prepare_foundation_writing().unwrap().unwrap().round_id,
        retry.round_id
    );
    assert_eq!(
        a.handwriting_originals(&first.scope(), &first.task())
            .unwrap(),
        vec![original]
    );
}

#[test]
fn continuous_recall_catalog_detour_cannot_replace_checkpoint_or_clear_help() {
    let mut a = app();
    let checkpoint = start_checkpoint(&mut a, "f1-radio-no");
    let first = a.prepare_foundation_writing().unwrap().unwrap();
    let draft = drawing(&first, false);
    a.handwriting_store_draft(&first.scope(), &first.task(), &draft)
        .unwrap();
    a.open_writing_catalog().unwrap();
    a.open_writing_task("foundation-hand-print-a", Sequence::SeeCopyHearSay)
        .unwrap();
    let catalog = a.active_handwriting().unwrap();
    assert_ne!(first.id, catalog.id);
    a.start_foundations().unwrap();
    let restored = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(restored.id, first.id);
    assert_eq!(restored.round_id, first.round_id);
    assert_eq!(
        restored.pre_submission_cues,
        Some(vec!["reference_material".into()])
    );
    assert_eq!(
        a.foundation_writing_checkpoint().unwrap().unwrap(),
        checkpoint
    );
    assert_eq!(
        a.handwriting_draft(&first.scope(), &first.task()).unwrap(),
        Some(draft)
    );
    assert!(a.foundation_edit(vec!["late old answer".into()]).is_err());
    assert!(a.foundation_replayed("foundation-net").is_err());
    assert!(a.foundation_next(true).is_err());
}

#[test]
fn continuous_recall_later_returns_preserve_course_and_are_bounded_without_grades() {
    let mut a = app();
    let now = repository::now_ms();
    start_checkpoint(&mut a, "f1-radio-no");
    finish_checkpoint_at(&mut a, now);
    finish_checkpoint_at(&mut a, now + 1);
    start_checkpoint(&mut a, "f2-radio-no");
    finish_checkpoint_at(&mut a, now + 2);
    assert!(
        a.foundation_writing_returns(now + rufren_core::DAY_MS - 1)
            .is_empty()
    );
    let later = now + rufren_core::DAY_MS + 10;
    let queue = a.foundation_writing_returns(later);
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].task_id, "foundation-hand-print-ma");
    assert_eq!(queue[1].task_id, "foundation-hand-print-am");
    let mut course = a.foundation_state().unwrap();
    course.draft = vec!["unfinished response kept exactly".into()];
    course.cues.push(rufren_core::Cue::Translation);
    a.foundation_save(&course).unwrap();
    let before = serde_json::to_value(&course).unwrap();
    let event_count = a.repo.library().unwrap().events.len();
    a.foundation_start_writing_return_at(&queue[0].task_id, later)
        .unwrap();
    let view = a.foundation_writing_checkpoint().unwrap().unwrap();
    assert_eq!(
        view.kind,
        foundation_writing::FoundationWritingCheckpointKind::Return
    );
    let run = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(run.sequence, Sequence::HearWriteCompareSay);
    assert!(!run.model_revealed && !run.audio_completed);
    let mut restored = AppState::from_repository(a.repo.clone()).unwrap();
    restored.start_foundations().unwrap();
    assert_eq!(
        restored.foundation_writing_checkpoint().unwrap().unwrap(),
        view
    );
    restored
        .foundation_finish_writing_checkpoint_at(view.item_session, true, later + 1)
        .unwrap();
    assert_eq!(
        serde_json::to_value(restored.foundation_state().unwrap()).unwrap(),
        before
    );
    assert_eq!(restored.repo.library().unwrap().events.len(), event_count);
    let queue = restored.foundation_writing_returns(later + 2);
    assert_eq!(queue.len(), 1); // One of two daily return opportunities remains.
    assert_eq!(queue[0].task_id, "foundation-hand-print-am");
    restored
        .foundation_start_writing_return_at(&queue[0].task_id, later + 2)
        .unwrap();
    let second = restored.foundation_writing_checkpoint().unwrap().unwrap();
    restored
        .foundation_finish_writing_checkpoint_at(second.item_session, true, later + 3)
        .unwrap();
    assert!(restored.foundation_writing_returns(later + 4).is_empty());
    assert_eq!(
        serde_json::to_value(restored.foundation_state().unwrap()).unwrap(),
        before
    );
    assert_eq!(restored.repo.library().unwrap().events.len(), event_count);
}

#[test]
fn continuous_recall_recovers_committed_visit_before_course_write_without_new_round() {
    let mut a = app();
    start_checkpoint(&mut a, "f1-radio-no");
    let before_first = a.foundation_state().unwrap();
    let first_view = a.foundation_writing_checkpoint().unwrap().unwrap();
    let completed = finish_checkpoint_at(&mut a, repository::now_ms());
    let receipt_key = format!("foundation.writing.visit:{}", first_view.item_session);
    let receipt = a
        .repo
        .get::<serde_json::Value>(&receipt_key)
        .unwrap()
        .unwrap();
    // Fault boundary: visit reached durable storage, the later course write did not.
    a.foundation_save(&before_first).unwrap();
    assert!(
        a.writing_new_round(
            completed.id,
            completed.round_id,
            Sequence::HearWriteCompareSay
        )
        .is_err()
    );
    assert!(
        a.writing_check_original(
            completed.id,
            completed.round_id,
            completed.original.as_ref().unwrap()
        )
        .is_err()
    );
    assert!(a.prepare_foundation_writing().unwrap().is_none());
    let second = a.foundation_writing_checkpoint().unwrap().unwrap();
    assert_eq!(second.task_id, "foundation-hand-print-am");
    assert_ne!(second.item_session, first_view.item_session);
    assert_eq!(
        a.repo
            .get::<serde_json::Value>(&receipt_key)
            .unwrap()
            .unwrap(),
        receipt
    );
    assert_eq!(a.writing_load(completed.id).unwrap().history.len(), 0);
    let before_skip = a.foundation_state().unwrap();
    a.foundation_skip_writing_checkpoint(second.item_session)
        .unwrap();
    let mut after_skip = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    a.foundation_save(&before_skip).unwrap();
    assert!(a.prepare_foundation_writing().unwrap().is_none());
    let mut recovered = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    // The next step had no durable session before the failed write.
    after_skip.as_object_mut().unwrap().remove("session");
    recovered.as_object_mut().unwrap().remove("session");
    assert_eq!(recovered, after_skip);
    assert!(a.foundation_writing_checkpoint().unwrap().is_none());
    assert_eq!(
        a.repo
            .metadata_with_prefix("foundation.writing.visit:")
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn continuous_recall_native_help_is_taken_from_original_not_post_compare_visibility() {
    for (help, expected) in [
        (
            Some(
                serde_json::json!({"history_from_start":true,"visible_at_start":false,"revealed_at_ms":[]}),
            ),
            Some(Vec::<String>::new()),
        ),
        (
            Some(
                serde_json::json!({"history_from_start":true,"visible_at_start":false,"revealed_at_ms":[123500]}),
            ),
            Some(vec!["visual_model".into()]),
        ),
        (
            Some(
                serde_json::json!({"history_from_start":false,"visible_at_start":true,"revealed_at_ms":[]}),
            ),
            None,
        ),
        (None, None),
    ] {
        let mut a = app();
        start_checkpoint(&mut a, "f2-radio-no");
        let run = a.prepare_foundation_writing().unwrap().unwrap();
        a.writing_audio_completed(run.id, run.round_id, "foundation-kot")
            .unwrap();
        // Boundary fixture: generic original classification, not a device archive validation.
        let payload = serde_json::json!({"model_help":help});
        let reference = a
            .writing_store_artifact(
                run.attempt_id,
                &run.scope(),
                &run.task().id,
                &run.task_revision,
                "apple-pencilkit-v1",
                payload,
            )
            .unwrap();
        a.writing_accept_original(run.id, run.round_id, reference)
            .unwrap();
        assert_eq!(
            a.active_handwriting().unwrap().pre_submission_cues,
            expected
        );
        a.writing_model_revealed(run.id, run.round_id).unwrap();
        assert_eq!(
            a.active_handwriting().unwrap().pre_submission_cues,
            expected
        );
    }
}

#[test]
fn continuous_recall_reference_after_comparison_does_not_rewrite_original_support() {
    let mut a = app();
    start_checkpoint(&mut a, "f2-radio-no");
    let run = a.prepare_foundation_writing().unwrap().unwrap();
    a.writing_audio_completed(run.id, run.round_id, "foundation-kot")
        .unwrap();
    save(&mut a, &run, &drawing(&run, true));
    let compared = a.active_handwriting().unwrap();
    assert_eq!(compared.pre_submission_cues, Some(vec![]));
    a.open_writing_catalog().unwrap();
    a.start_foundations().unwrap();
    let returned = a.prepare_foundation_writing().unwrap().unwrap();
    assert_eq!(returned.pre_submission_cues, compared.pre_submission_cues);
    assert_eq!(returned.original, compared.original);
    assert_eq!(returned.round_id, compared.round_id);
}

#[test]
fn continuous_recall_late_base_callbacks_cannot_mutate_the_suspended_course() {
    let mut a = app();
    source(&mut a, "c6-window-it-say");
    let state = a.foundation_state().unwrap();
    let step = rufren_content::foundations().episodes[state.episode].steps[state.step].clone();
    a.foundation_edit(vec!["I tried aloud (self-report).".into()])
        .unwrap();
    a.foundation_submit(&step).unwrap();
    a.foundation_next(false).unwrap();
    assert!(a.foundation_writing_checkpoint().unwrap().is_some());
    let before = serde_json::to_value(a.foundation_state().unwrap()).unwrap();
    let events = a.repo.library().unwrap().events;
    let revision = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "rufren-foundation:{}:{}",
            rufren_content::foundations::foundation_step_pack_revision(&step.id),
            step.id
        )
        .as_bytes(),
    )
    .to_string();
    let recording = rufren_core::AudioRecordMetadata {
        asset_id: "late-course-recording".into(),
        content_hash: "a".repeat(64),
        role: "attempt".into(),
        name: "Late recorded original".into(),
        passage_id: revision.clone(),
        revision_id: revision,
        duration_ms: 1000,
        sample_rate: 16000,
        channels: 1,
        frames: 16000,
        byte_length: 32044,
        created_at_ms: repository::now_ms(),
        mime_type: "audio/wav".into(),
        interrupted: false,
        device: serde_json::Value::Null,
        annotations: vec![],
    };
    recording.validate().unwrap();
    macro_rules! rejects_without_mutating {
        ($call:expr) => {{
            assert!($call.is_err());
            assert_eq!(
                serde_json::to_value(a.foundation_state().unwrap()).unwrap(),
                before
            );
            assert_eq!(a.repo.library().unwrap().events, events);
        }};
    }
    rejects_without_mutating!(a.foundation_cannot_yet(&step));
    rejects_without_mutating!(a.foundation_save_recording(recording));
    let read_step = rufren_content::foundations().episodes[0]
        .steps
        .iter()
        .find(|s| matches!(s.action, rufren_content::FoundationAction::Read { .. }))
        .unwrap()
        .clone();
    rejects_without_mutating!(a.foundation_read_try(&read_step));
    rejects_without_mutating!(a.foundation_replayed("foundation-ono"));
    rejects_without_mutating!(a.foundation_edit(vec!["late edit".into()]));
    rejects_without_mutating!(a.foundation_help());
    rejects_without_mutating!(a.foundation_retry());
    rejects_without_mutating!(a.foundation_submit(&step));
    rejects_without_mutating!(a.foundation_next(false));
    rejects_without_mutating!(a.foundation_repeat_episode());
}

#[test]
fn continuous_recall_blank_help_allows_later_practice_without_inventing_ink() {
    for portable in [true, false] {
        let mut a = app();
        let first = start_checkpoint(&mut a, "f1-radio-no");
        let run = a.prepare_foundation_writing().unwrap().unwrap();
        let events = a.repo.library().unwrap().events;
        if portable {
            let mut blank = drawing(&run, false);
            blank.strokes.clear();
            blank.mode = InkMode::Copy;
            blank.aids.push(InkAid {
                kind: "visual_model".into(),
                value: "copy".into(),
                at_ms: repository::now_ms(),
            });
            a.handwriting_store_draft(&run.scope(), &run.task(), &blank)
                .unwrap();
        } else {
            // Native Show example event is an explicit request, not post-Compare reveal.
            a.writing_model_revealed(run.id, run.round_id).unwrap();
        }
        let now = repository::now_ms();
        a.foundation_finish_writing_checkpoint_at(first.item_session, true, now)
            .unwrap();
        let second = a.foundation_writing_checkpoint().unwrap().unwrap();
        a.foundation_finish_writing_checkpoint_at(second.item_session, true, now + 1)
            .unwrap();
        let key = format!("foundation.writing.visit:{}", first.item_session);
        let help = a.repo.get::<serde_json::Value>(&key).unwrap().unwrap();
        assert_eq!(help["help_requested"], true);
        assert!(help["originals"].as_array().unwrap().is_empty());
        assert!(
            a.handwriting_originals(&run.scope(), &run.task())
                .unwrap()
                .is_empty()
        );
        let later = a.foundation_writing_returns(now + rufren_core::DAY_MS + 10);
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].task_id, "foundation-hand-print-ma");
        assert_eq!(a.repo.library().unwrap().events, events);
        let skip_key = format!("foundation.writing.visit:{}", second.item_session);
        let mut skipped = a.repo.get::<serde_json::Value>(&skip_key).unwrap().unwrap();
        assert_eq!(skipped["help_requested"], false);
        // Older receipts missing this optional field must not infer a help request.
        skipped.as_object_mut().unwrap().remove("help_requested");
        a.repo.set(&skip_key, &skipped).unwrap();
        assert_eq!(
            a.foundation_writing_returns(now + rufren_core::DAY_MS + 10),
            later
        );
    }
}

#[test]
fn continuous_recall_repair_from_resumed_speaking_preserves_exact_stage_and_original() {
    let mut a = app();
    let checkpoint = start_checkpoint(&mut a, "f2-radio-no");
    let first = a.prepare_foundation_writing().unwrap().unwrap();
    a.writing_audio_completed(first.id, first.round_id, "foundation-kot")
        .unwrap();
    save(&mut a, &first, &drawing(&first, true));
    a.writing_go_say(first.id, first.round_id).unwrap();
    let speaking = a.active_handwriting().unwrap();
    assert_eq!(speaking.stage, WritingStage::Say);
    let resumed = AppState::from_repository(a.repo.clone()).unwrap();
    resumed
        .writing_new_round(first.id, first.round_id, Sequence::HearWriteCompareSay)
        .unwrap();
    let retry = resumed.active_handwriting().unwrap();
    assert_eq!(retry.history.len(), 1);
    assert_eq!(retry.history[0].stage, Some(WritingStage::Say));
    assert_eq!(retry.history[0].original, speaking.original);
    assert_eq!(retry.history[0].say, SayDisposition::NotReported);
    assert_ne!(retry.round_id, first.round_id);
    assert_eq!(retry.stage, WritingStage::Writing);
    assert_eq!(
        retry.pre_submission_cues,
        Some(vec!["after_comparison".into()])
    );
    assert_eq!(
        resumed.foundation_writing_checkpoint().unwrap().unwrap(),
        checkpoint
    );
}
