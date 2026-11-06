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
use daily_plan::{DailyPlanItemState, DailyPlanSettings};
use model::AppState;
use repository::{Repository, now_ms};
use rufren_core::*;
use rufren_tutor::*;
use std::rc::Rc;
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
fn generic(s: &mut AppState, skill: Skill, at: i64) -> Uuid {
    let mut r = starter(s.device, at);
    if let Fact::PassageRevised(r) = &mut r.fact {
        r.passage.title = "ANSWER TRANSLATION MUST STAY HIDDEN".into();
        r.passage.source = "Public synthetic saved passage".into();
    }
    let a = Event::new(
        s.device,
        at,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: r.id,
            skill,
            response: "Public old response".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "free-practice-v1".into(),
        }),
    )
    .unwrap();
    s.repo.append(&[r.clone(), a]).unwrap();
    s.refresh().unwrap();
    r.id
}
fn teacher(s: &mut AppState, at: i64) -> TeacherRevisitKey {
    let mut answer: TeacherResponse = serde_json::from_str(include_str!(
        "../../../scripts/platform/fixtures/teacher-notebook-response.json"
    ))
    .unwrap();
    let TeacherRequest::TeachLesson(r) = &mut answer.request else {
        unreachable!()
    };
    r.lesson_id = s.content.lessons[0].id.clone();
    answer.provenance.response_id = Uuid::new_v4().to_string();
    let (source, revision, fact) = teacher_practice_reference_fact(&answer, 0).unwrap();
    let source_id = source;
    let reference = Event {
        version: 2,
        id: revision,
        device: source,
        recorded_at_ms: 0,
        fact,
    };
    let session = Uuid::new_v4();
    let attempt = Uuid::new_v5(&session, b"attempt");
    let context = Uuid::new_v5(&session, b"context");
    let (prompt, reference_answer) = teacher_practice_prompt_reference(&answer, 0).unwrap();
    let e = Event {
        version: EVENT_VERSION,
        id: attempt,
        device: s.device,
        recorded_at_ms: at,
        fact: Fact::AttemptRecorded(Attempt {
            session,
            revision,
            skill: Skill::Writing,
            response: "Public synthetic guided response".into(),
            cues: vec![Cue::ReferenceMaterial],
            self_rating: SelfRating::NotReported,
            policy: "teacher-guided-text-v1".into(),
        }),
    };
    let TeacherRequest::TeachLesson(r) = &answer.request else {
        unreachable!()
    };
    let c = Event {
        version: EVENT_VERSION,
        id: context,
        device: s.device,
        recorded_at_ms: at,
        fact: Fact::AttemptContextRecorded(AttemptContext {
            attempt,
            content_pack: None,
            content_revision: None,
            exercise_id: Some(format!("teacher:v2:{}:{}:0", r.lesson_id, source)),
            prompt: Some(prompt),
            reference_answer: Some(reference_answer),
            task: TaskKind::Composition,
            target_features: Default::default(),
            novel_context: false,
            delayed_probe: false,
            practice_duration_ms: None,
            effort: None,
            grammar_response: None,
            supersedes: Default::default(),
        }),
    };
    let archive = Event::new(
        s.device,
        at,
        Fact::TeacherDocumentRecorded(TeacherDocument {
            format_version: 1,
            document_id: source,
            response: answer,
            receipt: TeacherDocumentReceipt::RecoveredLocalCache,
            context_source_fact_ids: None,
        }),
    )
    .unwrap();
    s.repo.append(&[archive, reference, e, c]).unwrap();
    s.refresh().unwrap();
    assert!(
        s.teacher_revisits(now_ms())
            .entries
            .iter()
            .any(|e| e.key.document_id == source_id)
    );
    TeacherRevisitKey {
        document_id: source,
        practice_index: 0,
    }
}
fn type_response(s: &mut AppState, text: &str) {
    let mut session = s.session.clone().unwrap();
    session.draft = text.into();
    s.repo.set("session", &Some(&session)).unwrap();
    s.session = Some(session);
}
fn finish(s: &mut AppState) {
    type_response(s, "Public completed response");
    s.phase(Phase::Reflect).unwrap();
    s.record(SelfRating::NotReported).unwrap();
}
struct Disk {
    path: std::path::PathBuf,
}
impl Disk {
    fn new() -> Self {
        Self {
            path: std::env::temp_dir().join(format!("rufren-daily-plan-{}.sqlite", Uuid::new_v4())),
        }
    }
    fn app(&self) -> AppState {
        AppState::from_repository(Rc::new(Repository::open_at(&self.path).unwrap())).unwrap()
    }
    fn sql(&self, sql: &str) {
        rusqlite::Connection::open(&self.path)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
    }
}
impl Drop for Disk {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

#[test]
fn daily_mixed_session_does_not_turn_an_imported_lesson_into24_obligations() {
    let mut s = app();
    let lesson = s.content.lessons[0].id.clone();
    s.import_lesson(&lesson).unwrap();
    let empty = s.prepare_daily_plan(now_ms()).unwrap();
    assert!(empty.items.is_empty());
    // Enrollment is by completed revision AND skill, not every material/skill cross-product.
    let now = now_ms();
    let revision = generic(&mut s, Skill::Reading, now - 5 * DAY_MS);
    let plan = s.new_daily_plan(now_ms()).unwrap();
    assert_eq!(plan.items.len(), 1);
    assert_eq!(
        plan.items[0].key,
        PracticeBatchKey::Passage {
            revision,
            skill: Skill::Reading
        }
    );
    assert!(!plan.items[0].title.contains("ANSWER"));
    assert!(!plan.items[0].reason.contains("ANSWER"));
}

#[test]
fn daily_large_teacher_backlog_still_includes_enrolled_passage_and_quiet_is_explicit() {
    let mut s = app();
    let now = now_ms();
    for _ in 0..8 {
        teacher(&mut s, now - 5 * DAY_MS);
    }
    generic(&mut s, Skill::Listening, now - 4 * DAY_MS);
    generic(&mut s, Skill::Reading, now - 3 * DAY_MS);
    let plan = s.prepare_daily_plan(now).unwrap();
    assert_eq!(plan.items.len(), 3);
    assert!(
        plan.items
            .iter()
            .any(|i| matches!(i.key, PracticeBatchKey::Passage { .. }))
    );
    assert!(
        plan.items
            .iter()
            .any(|i| matches!(i.key, PracticeBatchKey::Teacher { .. }))
    );
    assert_eq!(plan.completed_count(), 0);
    s.save_daily_plan_settings(DailyPlanSettings {
        response_budget: 1,
        quiet: true,
    })
    .unwrap();
    assert_eq!(s.prepare_daily_plan(now).unwrap().id, plan.id);
    assert_eq!(
        s.daily_plan(now).unwrap().unwrap().settings.response_budget,
        3
    );
    for i in 0..plan.items.len() {
        s.skip_daily_plan_item(i).unwrap();
    }
    let quiet = s.new_daily_plan(now_ms()).unwrap();
    assert_eq!(quiet.items.len(), 1);
    assert!(
        quiet
            .items
            .iter()
            .all(|i| matches!(i.skill, Skill::Reading | Skill::Writing))
    );
}

#[test]
fn daily_saved_response_completes_exact_item_and_stays_finished_until_explicit_new_session() {
    let mut s = app();
    let now = now_ms();
    generic(&mut s, Skill::Writing, now - 5 * DAY_MS);
    let plan = s.prepare_daily_plan(now).unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    finish(&mut s);
    let done = s.daily_plan(now_ms()).unwrap().unwrap();
    assert_eq!(done.completed_count(), 1);
    assert!(done.is_finished());
    assert!(s.skip_daily_plan_item(0).is_err());
    assert!(matches!(
        s.daily_plan(now_ms()).unwrap().unwrap().items[0].state,
        DailyPlanItemState::Completed { .. }
    ));
    generic(&mut s, Skill::Reading, now - 5 * DAY_MS);
    assert_eq!(s.prepare_daily_plan(now_ms()).unwrap().id, plan.id);
    let next = s.new_daily_plan(now_ms()).unwrap();
    assert_ne!(next.id, plan.id);
}

#[test]
fn daily_repeated_try_and_restart_resume_exact_draft_and_support() {
    let disk = Disk::new();
    let mut s = disk.app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    let plan = s.prepare_daily_plan(now_ms()).unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    let id = s.session.as_ref().unwrap().id;
    type_response(&mut s, "A partly written public answer");
    s.hint().unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, id);
    assert_eq!(
        s.session.as_ref().unwrap().draft,
        "A partly written public answer"
    );
    assert_eq!(s.session.as_ref().unwrap().cues, vec![Cue::Translation]);
    assert_eq!(s.library.decisions(id).len(), 1);
    drop(s);
    let mut s = disk.app();
    s.begin().unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, id);
    assert_eq!(s.daily_plan(now_ms()).unwrap().unwrap().id, plan.id);
    assert_eq!(
        s.session.as_ref().unwrap().draft,
        "A partly written public answer"
    );
    // A paused exact session is reused, not silently replaced by a blank one.
    s.archive_current().unwrap();
    s.repo.set("session", &None::<Session>).unwrap();
    s.session = None;
    s.plan = None;
    s.start_daily_plan_item(0, now_ms()).unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, id);
    assert_eq!(s.session.as_ref().unwrap().cues, vec![Cue::Translation]);
}

#[test]
fn daily_late_imported_response_and_changed_revision_cannot_fabricate_completion() {
    let mut s = app();
    let now = now_ms();
    let revision = generic(&mut s, Skill::Reading, now - 8 * DAY_MS);
    s.prepare_daily_plan(now).unwrap();
    let late = Event::new(
        s.device,
        now - 7 * DAY_MS,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision,
            skill: Skill::Reading,
            response: "Older imported response".into(),
            cues: vec![],
            self_rating: SelfRating::Recalled,
            policy: "free-practice-v1".into(),
        }),
    )
    .unwrap();
    s.repo.append(&[late]).unwrap();
    s.refresh().unwrap();
    assert_eq!(
        s.daily_plan(now_ms()).unwrap().unwrap().completed_count(),
        0
    );
    let mut r = s.library.revision(revision).unwrap().clone();
    r.parents.insert(revision);
    r.passage.russian = "Это новая фраза.".into();
    let changed = Event::new(s.device, now, Fact::PassageRevised(r)).unwrap();
    s.repo.append(&[changed]).unwrap();
    s.refresh().unwrap();
    let unavailable = s.daily_plan(now_ms()).unwrap().unwrap();
    assert!(matches!(
        unavailable.items[0].state,
        DailyPlanItemState::Unavailable { .. }
    ));
    assert!(!unavailable.is_finished());
    assert!(s.start_daily_plan_item(0, now_ms()).is_err());
    s.skip_daily_plan_item(0).unwrap();
    assert!(s.daily_plan(now_ms()).unwrap().unwrap().is_finished());
}

#[test]
fn daily_start_reservation_failure_has_no_exposed_or_unbound_session() {
    let disk = Disk::new();
    let mut s = disk.app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    s.prepare_daily_plan(now_ms()).unwrap();
    disk.sql("CREATE TRIGGER fail_reservation BEFORE UPDATE ON metadata WHEN NEW.key='daily_plan_active' AND json_extract(NEW.json,'$.items[0].start_pending')=1 BEGIN SELECT RAISE(ABORT,'public synthetic write failure'); END;");
    assert!(s.start_daily_plan_item(0, now_ms()).is_err());
    assert!(s.session.is_none());
    assert_eq!(s.library.attempts().count(), 1);
    disk.sql("DROP TRIGGER fail_reservation;");
    s.begin().unwrap();
    assert_eq!(s.session.as_ref().unwrap().phase, Phase::Retrieve);
    finish(&mut s);
    assert_eq!(
        s.daily_plan(now_ms()).unwrap().unwrap().completed_count(),
        1
    );
}

#[test]
fn daily_interrupted_generic_setup_and_final_ack_recover_same_reserved_identity_before_exposure() {
    let disk = Disk::new();
    let mut s = disk.app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    s.prepare_daily_plan(now_ms()).unwrap();
    disk.sql("CREATE TRIGGER fail_setup BEFORE INSERT ON metadata WHEN NEW.key='session_plan' BEGIN SELECT RAISE(ABORT,'public synthetic setup failure'); END;");
    assert!(s.start_daily_plan_item(0, now_ms()).is_err());
    let id = s.session.as_ref().unwrap().id;
    assert_eq!(s.session.as_ref().unwrap().phase, Phase::Encounter);
    assert!(s.page == model::Page::Today);
    disk.sql("DROP TRIGGER fail_setup;");
    drop(s);
    let mut s = disk.app();
    s.begin().unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, id);
    assert_eq!(s.session.as_ref().unwrap().phase, Phase::Retrieve);
    assert_eq!(s.library.decisions(id).len(), 1);
    type_response(&mut s, "A public answer retained across retry");
    s.hint().unwrap();
    disk.sql("CREATE TRIGGER fail_ack BEFORE UPDATE ON metadata WHEN NEW.key='daily_plan_active' AND json_extract(OLD.json,'$.items[0].start_pending')=1 AND json_extract(NEW.json,'$.items[0].start_pending')=0 BEGIN SELECT RAISE(ABORT,'public synthetic ack failure'); END;");
    assert!(s.start_daily_plan_item(0, now_ms()).is_err());
    assert!(s.page == model::Page::Today);
    disk.sql("DROP TRIGGER fail_ack;");
    drop(s);
    let mut s = disk.app();
    s.navigate(model::Page::Practice).unwrap();
    assert_eq!(s.session.as_ref().unwrap().id, id);
    assert_eq!(
        s.session.as_ref().unwrap().draft,
        "A public answer retained across retry"
    );
    assert_eq!(s.session.as_ref().unwrap().cues, vec![Cue::Translation]);
    assert_eq!(s.library.decisions(id).len(), 1);
    finish(&mut s);
    assert_eq!(
        s.daily_plan(now_ms()).unwrap().unwrap().completed_count(),
        1
    );
}

#[test]
fn daily_skip_failure_is_recoverable_and_later_real_response_outranks_skip() {
    let disk = Disk::new();
    let mut s = disk.app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    s.prepare_daily_plan(now_ms()).unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    type_response(&mut s, "Public unfinished response");
    let id = s.session.as_ref().unwrap().id;
    disk.sql("CREATE TRIGGER fail_skip_ack BEFORE UPDATE ON metadata WHEN NEW.key='daily_plan_active' AND json_extract(OLD.json,'$.items[0].skip_cleanup_pending')=1 AND json_extract(NEW.json,'$.items[0].skip_cleanup_pending')=0 BEGIN SELECT RAISE(ABORT,'public synthetic skip ack failure'); END;");
    assert!(s.skip_daily_plan_item(0).is_err());
    assert!(s.session.is_none());
    assert_eq!(s.paused_practices().unwrap()[0].session.id, id);
    disk.sql("DROP TRIGGER fail_skip_ack;");
    s.reconcile_daily_plan(now_ms()).unwrap();
    assert!(s.daily_plan(now_ms()).unwrap().unwrap().is_finished());
    s.resume_practice(id).unwrap();
    finish(&mut s);
    assert_eq!(
        s.daily_plan(now_ms()).unwrap().unwrap().completed_count(),
        1
    );
    assert!(matches!(
        s.daily_plan(now_ms()).unwrap().unwrap().items[0].state,
        DailyPlanItemState::Completed { .. }
    ));
}

#[test]
fn daily_empty_preview_replans_on_explicit_begin_but_nonempty_finished_plan_does_not() {
    let mut s = app();
    let empty = s.prepare_daily_plan(now_ms()).unwrap();
    assert!(empty.items.is_empty());
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    let selected = s.prepare_daily_plan(now_ms()).unwrap();
    assert_ne!(selected.id, empty.id);
    assert_eq!(selected.items.len(), 1);
    s.skip_daily_plan_item(0).unwrap();
    generic(&mut s, Skill::Writing, now_ms() - 5 * DAY_MS);
    assert_eq!(s.prepare_daily_plan(now_ms()).unwrap().id, selected.id);
}

#[test]
fn daily_prebatch_teacher_response_keeps_its_start_and_is_not_counted_as_a_fresh_plan_response() {
    let mut s = app();
    let key = teacher(&mut s, now_ms() - 5 * DAY_MS);
    let start = now_ms();
    let mut draft = s.start_teacher_revisit(key, start).unwrap();
    s.clear_active_teacher_revisit().unwrap();
    let plan = s.prepare_daily_plan(now_ms().max(start + 1)).unwrap();
    assert_eq!(plan.items.len(), 1);
    assert!(matches!(
        plan.items[0].state,
        DailyPlanItemState::Unavailable { .. }
    ));
    assert!(s.start_daily_plan_item(0, now_ms()).is_err());
    let answer = s
        .library
        .teacher_document(key.document_id)
        .unwrap()
        .response
        .clone();
    s.teacher_practice_edit_response(
        &answer,
        &mut draft,
        "Public completed earlier-started response".into(),
    )
    .unwrap();
    s.submit_teacher_practice(&answer, &mut draft).unwrap();
    assert_eq!(
        s.daily_plan(now_ms()).unwrap().unwrap().completed_count(),
        0
    );
}

#[test]
fn daily_new_session_reconciles_interrupted_skip_before_checking_active_work() {
    let disk = Disk::new();
    let mut s = disk.app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    let old = s.prepare_daily_plan(now_ms()).unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    type_response(&mut s, "Public paused response");
    let id = s.session.as_ref().unwrap().id;
    disk.sql("CREATE TRIGGER fail_skip_ack BEFORE UPDATE ON metadata WHEN NEW.key='daily_plan_active' AND json_extract(OLD.json,'$.items[0].skip_cleanup_pending')=1 AND json_extract(NEW.json,'$.items[0].skip_cleanup_pending')=0 BEGIN SELECT RAISE(ABORT,'public synthetic skip failure'); END;");
    assert!(s.skip_daily_plan_item(0).is_err());
    disk.sql("DROP TRIGGER fail_skip_ack;");
    let next = s.new_daily_plan(now_ms()).unwrap();
    assert_ne!(next.id, old.id);
    assert!(s.session.is_none());
    assert!(
        s.paused_practices()
            .unwrap()
            .iter()
            .any(|p| p.session.id == id)
    );
}

#[test]
fn daily_explicit_new_session_pauses_unrelated_work_after_finished_plan_without_discarding_it() {
    let mut s = app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    let old = s.prepare_daily_plan(now_ms()).unwrap();
    s.skip_daily_plan_item(0).unwrap();
    let other = generic(&mut s, Skill::Writing, now_ms() - 4 * DAY_MS);
    s.start(other, Skill::Writing).unwrap();
    s.phase(Phase::Retrieve).unwrap();
    type_response(&mut s, "An unrelated partial public response");
    s.hint().unwrap();
    let id = s.session.as_ref().unwrap().id;
    let next = s.new_daily_plan(now_ms()).unwrap();
    assert_ne!(next.id, old.id);
    assert!(s.session.is_none());
    let paused = s.paused_practices().unwrap();
    let record = paused.iter().find(|p| p.session.id == id).unwrap();
    assert_eq!(record.session.draft, "An unrelated partial public response");
    assert_eq!(record.session.cues, vec![Cue::OriginalText]);
    s.resume_practice(id).unwrap();
    assert_eq!(
        s.session.as_ref().unwrap().draft,
        "An unrelated partial public response"
    );
}

#[test]
fn daily_new_session_detaches_missing_teacher_source_without_erasing_its_draft() {
    let mut s = app();
    generic(&mut s, Skill::Reading, now_ms() - 5 * DAY_MS);
    s.prepare_daily_plan(now_ms()).unwrap();
    s.start_daily_plan_item(0, now_ms()).unwrap();
    finish(&mut s);
    let document = Uuid::new_v4();
    let draft = Uuid::new_v4();
    let key = format!("teacher_revisit:{document}:0");
    let saved = serde_json::json!({"id":draft,"source":document.to_string(),"index":0,"response":"Keep these words while the source is missing"});
    s.repo.set(&key, &saved).unwrap();
    s.repo
        .set(
            "teacher_revisit_active",
            &serde_json::json!({"document_id":document,"practice_index":0,"draft_id":draft}),
        )
        .unwrap();
    assert!(s.active_teacher_revisit().is_err());
    let attempts = s.library.attempts().count();
    s.new_daily_plan(now_ms()).unwrap();
    assert!(s.active_teacher_revisit().unwrap().is_none());
    assert_eq!(s.repo.get::<serde_json::Value>(&key).unwrap(), Some(saved));
    assert_eq!(s.library.attempts().count(), attempts);
}

#[test]
fn daily_missing_revision_does_not_pretend_to_be_recent_passage_practice() {
    let mut s = app();
    let now = now_ms();
    let passage = generic(&mut s, Skill::Reading, now - 5 * DAY_MS);
    teacher(&mut s, now - 4 * DAY_MS);
    let orphan = Event::new(
        s.device,
        now - DAY_MS,
        Fact::AttemptRecorded(Attempt {
            session: Uuid::new_v4(),
            revision: Uuid::new_v4(),
            skill: Skill::Writing,
            response: "A response whose exact source has not arrived".into(),
            cues: vec![],
            self_rating: SelfRating::NotReported,
            policy: "free-practice-v1".into(),
        }),
    )
    .unwrap();
    s.repo.append(&[orphan]).unwrap();
    s.save_daily_plan_settings(DailyPlanSettings {
        response_budget: 1,
        quiet: false,
    })
    .unwrap();
    let plan = s.prepare_daily_plan(now).unwrap();
    assert_eq!(
        plan.items[0].key,
        PracticeBatchKey::Passage {
            revision: passage,
            skill: Skill::Reading
        }
    );
}
