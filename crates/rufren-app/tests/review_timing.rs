//! Bounded mixed practice, actual response completion, and interruption recovery. No providers.
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
use std::{collections::BTreeSet, rc::Rc};
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn target(s: &mut AppState) -> PracticeBatchKey {
    let event = starter(s.device, 1);
    let key = PracticeBatchKey::Passage {
        revision: event.id,
        skill: Skill::Reading,
    };
    s.repo.append(&[event]).unwrap();
    s.refresh().unwrap();
    key
}
#[test]
fn shared_preferences_reject_unseen_heads_and_keep_issued_start() {
    let mut s = app();
    let key = target(&mut s);
    s.change_review_cadence(ReviewCadence::MoreSpace, BTreeSet::new(), 10)
        .unwrap();
    assert!(
        s.change_review_cadence(ReviewCadence::MoreOften, BTreeSet::new(), 11)
            .is_err()
    );
    let session = Uuid::new_v4();
    let decision = s
        .prepare_review_start(
            session,
            key,
            20,
            12,
            12,
            "test-policy",
            "Explicit practice",
            BTreeSet::new(),
        )
        .unwrap();
    s.record_review_start(&decision).unwrap();
    let heads = s.review_cadence(13).heads;
    s.change_review_cadence(ReviewCadence::MoreOften, heads, 13)
        .unwrap();
    let retry = s
        .prepare_review_start(
            session,
            key,
            99,
            14,
            14,
            "changed-policy",
            "New reason",
            BTreeSet::new(),
        )
        .unwrap();
    assert_eq!(retry, decision);
    assert_eq!(retry.cadence, ReviewCadence::MoreSpace);
}
#[test]
fn availability_changes_do_not_create_attempts_or_replace_baseline() {
    let mut s = app();
    let key = target(&mut s);
    s.change_review_timing(
        key,
        ReviewTimingAction::AvailableAt { at_ms: 30 },
        BTreeSet::new(),
        10,
    )
    .unwrap();
    let projected = s.review_timing(key, 100, 20);
    assert_eq!(projected.baseline_due_at_ms, 100);
    assert_eq!(projected.effective_due_at_ms, Some(30));
    assert_eq!(s.library.attempts().count(), 0);
    assert!(
        s.change_review_timing(key, ReviewTimingAction::Pause, BTreeSet::new(), 21)
            .is_err()
    );
    s.change_review_timing(key, ReviewTimingAction::Pause, projected.heads, 21)
        .unwrap();
    let paused = s.review_timing(key, 100, 22);
    assert!(paused.paused);
    assert_eq!(paused.effective_due_at_ms, None);
    assert_eq!(s.library.attempts().count(), 0);
}
#[test]
fn one_shot_choice_survives_abandonment_then_consumes_only_its_completed_start() {
    let mut s = app();
    let key = target(&mut s);
    s.change_review_timing(
        key,
        ReviewTimingAction::AvailableAt { at_ms: 10 },
        BTreeSet::new(),
        2,
    )
    .unwrap();
    let session = Uuid::new_v4();
    let issued = s
        .prepare_review_start(
            session,
            key,
            100,
            10,
            10,
            "test-policy",
            "Explicit response",
            BTreeSet::new(),
        )
        .unwrap();
    s.record_review_start(&issued).unwrap();
    assert_eq!(s.review_timing(key, 100, 11).effective_due_at_ms, Some(10));
    let PracticeBatchKey::Passage { revision, skill } = key else {
        unreachable!()
    };
    let attempt = Event::new(
        s.device,
        12,
        Fact::AttemptRecorded(Attempt {
            session,
            revision,
            skill,
            response: "Synthetic actual response".into(),
            cues: vec![],
            self_rating: SelfRating::NotReported,
            policy: "test-policy".into(),
        }),
    )
    .unwrap();
    s.repo.append(&[attempt.clone()]).unwrap();
    s.refresh().unwrap();
    assert_eq!(s.review_timing(key, 100, 13).effective_due_at_ms, Some(100));
    assert_eq!(
        s.review_cadence_for_attempt(&attempt, s.library.attempt(attempt.id).unwrap()),
        ReviewCadence::Standard
    );
}
#[test]
fn prepared_start_retains_preference_after_interrupted_append() {
    let mut s = app();
    let key = target(&mut s);
    let session = Uuid::new_v4();
    let issued = s
        .prepare_review_start(
            session,
            key,
            100,
            10,
            10,
            "test-policy",
            "First intent",
            BTreeSet::new(),
        )
        .unwrap();
    s.change_review_cadence(ReviewCadence::MoreSpace, BTreeSet::new(), 11)
        .unwrap();
    let retry = s
        .prepare_review_start(
            session,
            key,
            999,
            12,
            12,
            "another-policy",
            "Replacement",
            BTreeSet::new(),
        )
        .unwrap();
    assert_eq!(issued, retry);
    s.record_review_start(&retry).unwrap();
    assert_eq!(s.review_start_decision(session, key).unwrap(), Some(issued));
}
#[test]
fn guided_start_pins_cadence_and_legacy_draft_remains_unassigned() {
    let mut s = app();
    let answer: rufren_tutor::TeacherResponse = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/teacher-notebook-response.json"
    ))
    .unwrap();
    s.change_review_cadence(ReviewCadence::MoreSpace, BTreeSet::new(), 1)
        .unwrap();
    let draft = s.teacher_practice_draft(&answer, 0).unwrap();
    let key = PracticeBatchKey::Teacher {
        document_id: teacher_document_id(&answer),
        practice_index: 0,
    };
    assert_eq!(
        s.review_start_decision(draft.id, key)
            .unwrap()
            .unwrap()
            .cadence,
        ReviewCadence::MoreSpace
    );
    let legacy = teacher_practice::TeacherPracticeDraft {
        id: Uuid::new_v4(),
        source: teacher_practice::practice_source(&answer),
        index: 1,
        response: "An existing unfinished response".into(),
        started_at_ms: 2,
        saved_attempt: None,
        audio: None,
        review: None,
    };
    s.save_teacher_practice_draft(&legacy).unwrap();
    let restored = s.teacher_practice_draft(&answer, 1).unwrap();
    assert_eq!(restored.id, legacy.id);
    assert!(
        s.review_start_decision(
            legacy.id,
            PracticeBatchKey::Teacher {
                document_id: teacher_document_id(&answer),
                practice_index: 1
            }
        )
        .unwrap()
        .is_none()
    );
}
#[test]
fn large_known_input_set_keeps_digest_and_explicit_sample_limit() {
    let mut s = app();
    let key = target(&mut s);
    let ids: BTreeSet<_> = (0..700)
        .map(|i| {
            Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("synthetic-input-{i}").as_bytes(),
            )
        })
        .collect();
    let decision = s
        .prepare_review_start(
            Uuid::new_v4(),
            key,
            100,
            10,
            10,
            "test-policy",
            "Bounded input snapshot",
            ids.clone(),
        )
        .unwrap();
    assert_eq!(decision.evidence_total_count, 700);
    assert_eq!(decision.evidence_ids.len(), MAX_REVIEW_EVIDENCE_IDS);
    assert_eq!(
        decision.evidence_ids_sha256,
        review_evidence_ids_sha256(&ids)
    );
    assert_eq!(
        decision.evidence_ids,
        ids.into_iter().take(MAX_REVIEW_EVIDENCE_IDS).collect()
    );
}
fn practiced(s: &mut AppState, at: i64) -> PracticeBatchKey {
    let event = starter(s.device, at);
    let revision = event.id;
    let attempt = Event::new(
        s.device,
        at,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision,
            skill: Skill::Reading,
            response: "Synthetic previous response".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "free-practice-v1".into(),
        }),
    )
    .unwrap();
    s.repo.append(&[event, attempt]).unwrap();
    s.refresh().unwrap();
    PracticeBatchKey::Passage {
        revision,
        skill: Skill::Reading,
    }
}
#[test]
fn bounded_plan_keeps_paused_pending_item_and_sooner_selects_future_return() {
    use daily_plan::{DailyPlanItemState, DailyPlanSettings};
    let now = repository::now_ms();
    let mut s = app();
    let key = practiced(&mut s, now - 2 * DAY_MS);
    s.save_daily_plan_settings(DailyPlanSettings {
        response_budget: 1,
        quiet: false,
    })
    .unwrap();
    let plan = s.prepare_daily_plan(now).unwrap();
    assert_eq!(plan.items[0].key, key);
    s.change_review_timing(key, ReviewTimingAction::Pause, BTreeSet::new(), now)
        .unwrap();
    let paused = s.daily_plan(now).unwrap().unwrap();
    assert_eq!(paused.id, plan.id);
    assert!(matches!(
        paused.items[0].state,
        DailyPlanItemState::Unavailable { .. }
    ));
    assert!(!paused.is_finished());
    assert!(s.new_daily_plan(now).is_err());
    let mut future = app();
    let key = practiced(&mut future, now);
    assert!(future.prepare_daily_plan(now).unwrap().items.is_empty());
    future
        .change_review_timing(
            key,
            ReviewTimingAction::AvailableAt { at_ms: now },
            BTreeSet::new(),
            now,
        )
        .unwrap();
    let selected = future.new_daily_plan(now).unwrap();
    assert_eq!(selected.items[0].key, key);
}
#[test]
fn bounded_generic_start_records_its_actual_reason_and_existing_paused_draft_resumes() {
    let now = repository::now_ms();
    let mut s = app();
    let key = practiced(&mut s, now - 2 * DAY_MS);
    let plan = s.prepare_daily_plan(now).unwrap();
    let index = plan.items.iter().position(|i| i.key == key).unwrap();
    s.start_daily_plan_item(index, now).unwrap();
    let session = s.session.as_ref().unwrap().id;
    let issued = s.review_start_decision(session, key).unwrap().unwrap();
    assert_eq!(issued.policy, PRACTICE_BATCH_POLICY_ID);
    assert!(issued.reason.starts_with("From your short session:"));
    s.change_review_timing(key, ReviewTimingAction::Pause, BTreeSet::new(), now)
        .unwrap();
    s.begin().unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, session);
    assert_eq!(s.review_start_decision(session, key).unwrap(), Some(issued));
}
#[test]
fn generic_begin_recovers_interrupted_start_without_reassigning_preference() {
    let mut s = app();
    let now = repository::now_ms();
    let key = practiced(&mut s, now - 2 * DAY_MS);
    let PracticeBatchKey::Passage { revision, skill } = key else {
        unreachable!()
    };
    let session = Uuid::new_v4();
    let issued = s
        .prepare_review_start(
            session,
            key,
            now,
            now,
            now,
            "interruption-test",
            "The actual reserved reason",
            s.library.events.keys().copied().collect(),
        )
        .unwrap();
    // Exact durable boundary after session reservation and before ReviewStarted append.
    let draft = Session {
        id: session,
        revision,
        skill,
        phase: Phase::Encounter,
        draft: "Unfinished exact text".into(),
        cues: vec![],
    };
    s.repo.set("session", &Some(&draft)).unwrap();
    s.session = Some(draft);
    s.change_review_cadence(ReviewCadence::MoreSpace, BTreeSet::new(), now)
        .unwrap();
    let repo = s.repo.clone();
    drop(s);
    let mut restored = AppState::from_repository(repo).unwrap();
    restored.begin().unwrap();
    assert_eq!(restored.session.as_ref().unwrap().id, session);
    assert_eq!(
        restored.session.as_ref().unwrap().draft,
        "Unfinished exact text"
    );
    assert_eq!(
        restored.review_start_decision(session, key).unwrap(),
        Some(issued)
    );
}
