//! Passage editing survives reload, switching drafts, and interrupted publication.
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
#[path = "../src/reader.rs"]
mod reader;
#[path = "../src/teacher_dialogue.rs"]
mod teacher_dialogue;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/review_timing.rs"]
mod review_timing;
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

use editor_drafts::EditorField;
use model::{AppState, Editor};
use repository::{Repository, now_ms};
use rufren_core::*;
use std::{collections::BTreeSet, rc::Rc};
use uuid::Uuid;

fn fill(state: &mut AppState, title: &str, russian: &str, meaning: &str, source: &str) {
    for (field, value) in [
        (EditorField::Title, title),
        (EditorField::Russian, russian),
        (EditorField::Translation, meaning),
        (EditorField::Source, source),
    ] {
        state.update_editor(field, value.into()).unwrap();
    }
}

#[test]
fn audit_editor_reload_new_existing_and_ai_drafts_preserve_every_source_and_unfinished_edit() {
    let path = std::env::temp_dir().join(format!("rufren-editor-audit-{}.sqlite", Uuid::new_v4()));
    let open = || AppState::from_repository(Rc::new(Repository::open_at(&path).unwrap())).unwrap();
    let mut state = open();
    fill(
        &mut state,
        "A personal line",
        "Это книга.",
        "This is a book.",
        "Author · edition · page 12",
    );
    let first = state.editor.draft_id;
    drop(state);
    let mut state = open();
    assert_eq!(state.editor.draft_id, first);
    assert_eq!(state.editor.russian, "Это книга.");
    assert_eq!(state.editor.source, "Author · edition · page 12");
    state.save_passage(false).unwrap();
    let revision = state.editor.parent.unwrap();
    state
        .update_editor(EditorField::Russian, "Это другая книга.".into())
        .unwrap();
    state.new_passage().unwrap();
    fill(
        &mut state,
        "Second thought",
        "Где робот?",
        "Where is the robot?",
        "Conversation with coach",
    );
    let second = state.editor.draft_id;
    state
        .open_passage_draft(Editor {
            title: "AI example".into(),
            russian: "Робот читает.".into(),
            translation: "The robot reads.".into(),
            source: "Unreviewed AI suggestion · editorial example".into(),
            ..Default::default()
        })
        .unwrap();
    let third = state.editor.draft_id;
    assert_eq!(state.editor_drafts().unwrap().len(), 2);
    state.edit(revision).unwrap();
    assert_eq!(
        state.editor.draft_id, first,
        "choosing a passage resumes its unfinished edit"
    );
    assert_eq!(state.editor.russian, "Это другая книга.");
    assert_eq!(
        state.library.revision(revision).unwrap().passage.russian,
        "Это книга.",
        "draft text must not mutate an immutable original"
    );
    drop(state);
    let mut state = open();
    assert_eq!(state.editor.russian, "Это другая книга.");
    state.resume_editor_draft(second).unwrap();
    assert_eq!(state.editor.russian, "Где робот?");
    assert_eq!(state.editor.source, "Conversation with coach");
    state.resume_editor_draft(third).unwrap();
    assert_eq!(
        state.editor.source,
        "Unreviewed AI suggestion · editorial example"
    );
    assert_eq!(state.editor_drafts().unwrap().len(), 2);
    drop(state);
    std::fs::remove_file(path).unwrap(); // Only this test's unique database.
}

#[test]
fn audit_editor_interrupted_save_reuses_exact_fact_before_and_after_commit_and_duplicate_click() {
    for committed in [false, true] {
        let repo = Rc::new(Repository::open_at(":memory:").unwrap());
        let mut state = AppState::from_repository(repo.clone()).unwrap();
        fill(
            &mut state,
            "A line",
            "  Это робот.  ",
            "A robot.",
            "Source preserved",
        );
        let event = Event::new(
            state.device,
            now_ms(),
            Fact::PassageRevised(Revision {
                passage: Passage {
                    id: Uuid::new_v4(),
                    title: "A line".into(),
                    russian: "Это робот.".into(),
                    translation: "A robot.".into(),
                    source: "Source preserved".into(),
                    coach_reviewed: false,
                },
                parents: BTreeSet::new(),
            }),
        )
        .unwrap();
        let mut pending = state.editor.clone();
        pending.pending_save = Some(event.clone());
        state.open_passage_draft(pending).unwrap();
        if committed {
            repo.append(&[event.clone()]).unwrap();
        }
        // Close either after the durable intent or after the immutable fact,
        // before updating the draft's parent/clearing the pending marker.
        drop(state);
        let mut state = AppState::from_repository(repo.clone()).unwrap();
        assert!(state.editor.pending_save.is_some());
        state.save_passage(false).unwrap();
        assert_eq!(state.editor.parent, Some(event.id));
        assert_eq!(state.editor.russian, "Это робот.");
        assert!(state.editor.pending_save.is_none());
        state.save_passage(false).unwrap();
        state.save_passage(true).unwrap();
        assert_eq!(
            state.library.events.len(),
            1,
            "retry, duplicate Save, and redundant resolve must not duplicate publication"
        );
        assert_eq!(
            state.library.events[&event.id], event,
            "the first save's source, identity, and timestamp stay exact"
        );
        drop(state);
        let state = AppState::from_repository(repo).unwrap();
        assert_eq!(state.editor.parent, Some(event.id));
        assert!(state.editor.pending_save.is_none());
    }
}

#[test]
fn audit_editor_interrupted_activation_keeps_old_active_and_staged_draft_recoverable() {
    let repo = Rc::new(Repository::open_at(":memory:").unwrap());
    let mut state = AppState::from_repository(repo.clone()).unwrap();
    fill(
        &mut state,
        "Original draft",
        "Первый текст.",
        "First text.",
        "First source",
    );
    let original = state.editor.draft_id;
    let staged = Editor {
        title: "Staged example".into(),
        russian: "Второй текст.".into(),
        translation: "Second text.".into(),
        source: "Second source".into(),
        ..Default::default()
    };
    // Prefix of a switch: the new draft and index commit, then changing the
    // active pointer is interrupted. The old active record must remain intact.
    repo.set(
        &format!("passage_draft.{}", staged.draft_id),
        &serde_json::json!({"version":1,"editor":staged,"updated_at_ms":now_ms()+1}),
    )
    .unwrap();
    let mut ids: BTreeSet<Uuid> = repo.get("passage_draft_ids").unwrap().unwrap();
    ids.insert(staged.draft_id);
    repo.set("passage_draft_ids", &ids).unwrap();
    drop(state);
    let mut state = AppState::from_repository(repo.clone()).unwrap();
    assert_eq!(state.editor.draft_id, original);
    assert_eq!(
        state.editor_drafts().unwrap()[0].editor.draft_id,
        staged.draft_id
    );
    state.resume_editor_draft(staged.draft_id).unwrap();
    assert_eq!(state.editor.source, "Second source");
    assert_eq!(state.editor_drafts().unwrap()[0].editor.draft_id, original);
    // An interrupted very first activation has no previous active pointer.
    repo.set("active_passage_draft", &Option::<Uuid>::None)
        .unwrap();
    drop(state);
    let state = AppState::from_repository(repo).unwrap();
    assert!(
        !state.editor.russian.is_empty(),
        "first-activation recovery must not replace available text with an empty editor"
    );
}

#[test]
fn audit_editor_rejected_oversized_input_preserves_durable_previous_text() {
    let repo = Rc::new(Repository::open_at(":memory:").unwrap());
    let mut state = AppState::from_repository(repo.clone()).unwrap();
    state
        .update_editor(EditorField::Russian, "Сохранить это.".into())
        .unwrap();
    assert!(
        state
            .update_editor(EditorField::Russian, "я".repeat(MAX_TEXT))
            .is_err()
    );
    assert_eq!(state.editor.russian, "Сохранить это.");
    drop(state);
    assert_eq!(
        AppState::from_repository(repo).unwrap().editor.russian,
        "Сохранить это."
    );
}
