//! Durable notebook migration, sync/export, and exact lesson recovery. No provider calls.
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
use reader::{ReaderMode, ReaderSupportKind};
use repository::Repository;
use rufren_core::*;
use std::rc::Rc;
use uuid::Uuid;
fn app() -> AppState {
    AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
}
const ORIGINAL: &str = "Ёж идёт.\r\n\r\nОн видит за́мок.\n\nСлова\u{a0}остаются.";
fn imported(s: &mut AppState) -> Uuid {
    s.import_reader_with_source(
        "Synthetic source",
        ORIGINAL,
        "Public synthetic fixture; no model",
    )
    .unwrap()
}
fn count_attempts(s: &AppState) -> usize {
    s.library
        .events
        .values()
        .filter(|e| matches!(e.fact, Fact::AttemptRecorded(_)))
        .count()
}
struct Disk {
    path: std::path::PathBuf,
}
impl Disk {
    fn new() -> Self {
        Self {
            path: std::env::temp_dir().join(format!("rufren-reader-{}.sqlite", Uuid::new_v4())),
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
fn reader_exact_original_and_per_document_place_resume_without_learning_fiction() {
    let mut s = app();
    let a = imported(&mut s);
    let def = s.reader_definition(a).unwrap();
    assert_eq!(def.original, ORIGINAL);
    assert_eq!(def.chunks.len(), 3);
    assert_eq!(
        def.chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<String>(),
        ORIGINAL
    );
    assert!(
        def.chunks
            .iter()
            .all(|c| c.stress.is_none() && c.translation.is_none())
    );
    s.set_reader_chunk(&def.chunks[2].id).unwrap();
    s.set_reader_mode(ReaderMode::Focus).unwrap();
    s.mark_reader_support(a, &def.chunks[2].id, ReaderSupportKind::Meaning)
        .unwrap();
    s.edit_reader_own_words(a, &def.chunks[2].id, "My own interpretation")
        .unwrap();
    let state = s.reader_state().unwrap().unwrap();
    let repo = s.repo.clone();
    let b = s.import_reader("Second", "Другой текст.").unwrap();
    assert_ne!(a, b);
    s.edit_reader_own_words(b, "chunk-1", "Different notes")
        .unwrap();
    let mut reopened = AppState::from_repository(repo).unwrap();
    reopened.open_reader_document(a, None).unwrap();
    assert_eq!(reopened.reader_state().unwrap().unwrap(), state);
    assert_eq!(count_attempts(&reopened), 0);
    assert!(
        !reopened
            .library
            .events
            .values()
            .any(|e| matches!(e.fact, Fact::StimulusPresented(_)))
    );
    assert!(
        reopened
            .reader_definition(a)
            .unwrap()
            .source
            .provenance
            .contains("Public synthetic fixture")
    );
}
#[test]
fn reader_excerpt_retries_are_one_exact_phrase_and_audio_is_separate() {
    let mut s = app();
    let doc = imported(&mut s);
    let def = s.reader_definition(doc).unwrap();
    let chunk = &def.chunks[1];
    let rev = s.keep_reader_phrase(doc, &chunk.id, "за́мок").unwrap();
    assert_eq!(rev, s.keep_reader_phrase(doc, &chunk.id, "за́мок").unwrap());
    assert_eq!(s.library.reader_excerpt_links(doc).len(), 1);
    assert_eq!(s.library.revision(rev).unwrap().passage.russian, "за́мок");
    assert!(
        s.library
            .revision(rev)
            .unwrap()
            .passage
            .translation
            .is_empty()
    );
    let voice = s
        .prepare_reader_audio(doc, &chunk.id, ReaderTextLayer::Original)
        .unwrap();
    assert_ne!(voice, rev);
    assert!(is_reader_audio_reference(&s.library.events[&voice]));
    assert_eq!(
        s.library.revision(voice).unwrap().passage.russian,
        chunk.text
    );
    assert!(
        s.prepare_reader_audio(doc, &chunk.id, ReaderTextLayer::StressScaffold)
            .is_err()
    );
    assert!(s.keep_reader_phrase(doc, &chunk.id, "замок").is_err());
    assert_eq!(count_attempts(&s), 0);
}
#[test]
fn reader_selection_rejects_ambiguous_text_and_non_utf8_span() {
    let mut s = app();
    let doc = s.import_reader("Repeated", "Да, да. Да, да.").unwrap();
    assert!(s.keep_reader_phrase(doc, "chunk-1", "Да").is_err());
    assert!(
        s.keep_reader_span(doc, "chunk-1", TextSpan { start: 1, end: 4 })
            .is_err()
    );
    let id = s
        .keep_reader_span(doc, "chunk-1", TextSpan { start: 0, end: 4 })
        .unwrap();
    assert_eq!(s.library.revision(id).unwrap().passage.russian, "Да");
}
#[test]
fn reader_empty_client_can_open_original_excerpt_and_exact_question_without_local_cursor() {
    let mut s = app();
    let doc = imported(&mut s);
    let def = s.reader_definition(doc).unwrap();
    let chunk = &def.chunks[2];
    let rev = s.keep_reader_phrase(doc, &chunk.id, "Слова").unwrap();
    s.edit_reader_own_words(doc, &chunk.id, "Only this device's own words")
        .unwrap();
    let mut remote = app();
    let events: Vec<_> = s.library.events.values().cloned().collect();
    remote.repo.append(&events).unwrap();
    remote.refresh().unwrap();
    assert!(remote.reader_state().unwrap().is_none());
    remote.open_reader_document(doc, Some(&chunk.id)).unwrap();
    assert!(remote.reader_state().unwrap().unwrap().own_words.is_empty());
    assert_eq!(
        remote.library.reader_excerpt_link(rev).unwrap().document_id,
        doc
    );
    let id = remote
        .prepare_reader_question(doc, &chunk.id, "Could this phrase mean something else?")
        .unwrap();
    let draft = remote.active_dialogue_draft().unwrap().unwrap();
    assert_eq!(draft.id, id);
    assert_eq!(draft.text, "Could this phrase mean something else?");
    assert!(draft.anchor.unwrap().snapshot.contains(&chunk.text));
    assert_eq!(count_attempts(&remote), 0);
    assert!(remote.library.dialogue_user(id).is_none());
}
#[test]
fn reader_failure_keeps_visible_note_blocks_navigation_and_retries_exactly() {
    let disk = Disk::new();
    let mut s = disk.app();
    let doc = imported(&mut s);
    let before = s.reader_state().unwrap().unwrap();
    let key = format!("reader_state:{doc}");
    disk.sql(&format!("CREATE TRIGGER fail_reader BEFORE UPDATE ON metadata WHEN NEW.key='{key}' BEGIN SELECT RAISE(ABORT,'synthetic note failure'); END;"));
    assert!(
        s.edit_reader_own_words(doc, &before.chunk_id, "Unsaved visible interpretation")
            .is_err()
    );
    assert_eq!(
        s.reader_state().unwrap().unwrap().own_words[&before.chunk_id],
        "Unsaved visible interpretation"
    );
    assert!(s.set_reader_chunk("chunk-2").is_err());
    assert_eq!(s.reader_state().unwrap().unwrap().chunk_id, before.chunk_id);
    assert!(s.navigate(model::Page::Today).is_err());
    disk.sql("DROP TRIGGER fail_reader;");
    s.flush_pending_reader_edits().unwrap();
    let reopened = disk.app();
    assert_eq!(
        reopened.reader_state().unwrap().unwrap().own_words[&before.chunk_id],
        "Unsaved visible interpretation"
    );
}
#[test]
fn reader_failed_help_and_document_selection_do_not_replace_prior_state() {
    let disk = Disk::new();
    let mut s = disk.app();
    let a = imported(&mut s);
    let before = s.reader_state().unwrap().unwrap();
    disk.sql("CREATE TRIGGER fail_reader BEFORE UPDATE ON metadata WHEN NEW.key LIKE 'reader_state:%' BEGIN SELECT RAISE(ABORT,'synthetic reader failure'); END;");
    assert!(
        s.mark_reader_support(a, &before.chunk_id, ReaderSupportKind::Grammar)
            .is_err()
    );
    assert!(s.reader_state().unwrap().unwrap().supports.is_empty());
    disk.sql("DROP TRIGGER fail_reader;");
    let b = s.import_reader("Second", "Другой текст.").unwrap();
    s.open_reader_document(a, None).unwrap();
    disk.sql("CREATE TRIGGER fail_selection BEFORE UPDATE ON metadata WHEN NEW.key='reader_selection' BEGIN SELECT RAISE(ABORT,'synthetic selection failure'); END;");
    assert!(s.open_reader_document(b, None).is_err());
    assert_eq!(s.reader_state().unwrap().unwrap().document_id, a);
}
#[test]
fn reader_question_write_failure_keeps_old_composer_and_creates_no_empty_draft() {
    let disk = Disk::new();
    let mut s = disk.app();
    let doc = imported(&mut s);
    let first = s
        .prepare_reader_question(doc, "chunk-1", "My first exact question")
        .unwrap();
    let before = s.dialogue_drafts().unwrap();
    disk.sql("CREATE TRIGGER fail_composer BEFORE UPDATE ON metadata WHEN NEW.key='teacher_dialogue_state' BEGIN SELECT RAISE(ABORT,'synthetic composer failure'); END;");
    assert!(
        s.prepare_reader_question(doc, "chunk-2", "My second exact question")
            .is_err()
    );
    assert_eq!(s.dialogue_drafts().unwrap(), before);
    assert_eq!(s.active_dialogue_draft().unwrap().unwrap().id, first);
}
#[test]
fn reader_failed_paste_is_visible_until_retry_and_does_not_replace_reading() {
    let disk = Disk::new();
    let mut s = disk.app();
    let doc = imported(&mut s);
    disk.sql("CREATE TRIGGER fail_paste BEFORE UPDATE ON metadata WHEN NEW.key='reader_import_draft' BEGIN SELECT RAISE(ABORT,'synthetic paste failure'); END;");
    assert!(
        s.edit_reader_import_draft("New source", "Новые слова.", "Source supplied by learner")
            .is_err()
    );
    assert_eq!(s.reader_import_draft().unwrap().original, "Новые слова.");
    assert!(s.open_reader_document(doc, Some("chunk-2")).is_err());
    disk.sql("DROP TRIGGER fail_paste;");
    s.flush_pending_reader_edits().unwrap();
    let reopened = disk.app();
    assert_eq!(
        reopened.reader_import_draft().unwrap().source_note,
        "Source supplied by learner"
    );
}

#[test]
fn reader_partial_excerpt_commit_recovers_same_ids_without_duplicate_passage() {
    let mut s = app();
    let doc = imported(&mut s);
    let document = s.library.reader_document(doc).unwrap().clone();
    let chunk = &document.definition.chunks[2];
    let span = TextSpan {
        start: chunk.span.start,
        end: chunk.span.start + "Слова".len(),
    };
    let (passage, link) = reader_excerpt_events(&document, span).unwrap();
    // Browser event storage may acknowledge one immutable key before interruption.
    s.repo.append(&[passage.clone()]).unwrap();
    s.refresh().unwrap();
    let actual = s.keep_reader_phrase(doc, &chunk.id, "Слова").unwrap();
    assert_eq!(actual, passage.id);
    assert_eq!(s.library.events[&link.id], link);
    assert_eq!(s.library.reader_excerpt_links(doc).len(), 1);
    assert_eq!(
        actual,
        s.keep_reader_phrase(doc, &chunk.id, "Слова").unwrap()
    );
}
#[test]
fn reader_long_paste_chunks_preserve_all_unicode_and_source_identity() {
    let mut s = app();
    let original = "Ёж идёт через лес. ".repeat(260);
    assert!(original.len() > 4096);
    let doc = s
        .import_reader_with_source(
            "Long synthetic source",
            &original,
            "A public synthetic sentence repeated for boundary testing",
        )
        .unwrap();
    let def = s.reader_definition(doc).unwrap();
    assert!(def.chunks.len() > 1);
    assert!(def.chunks.iter().all(|c| c.text.len() <= 4096));
    assert_eq!(
        def.chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<String>(),
        original
    );
    assert_eq!(
        doc,
        s.import_reader_with_source(
            "Long synthetic source",
            &original,
            "A public synthetic sentence repeated for boundary testing"
        )
        .unwrap()
    );
    assert_ne!(
        doc,
        s.import_reader_with_source("Long synthetic source", &original, "Different attribution")
            .unwrap()
    );
    assert_eq!(count_attempts(&s), 0);
}
#[test]
fn reader_embedded_identity_collision_does_not_silently_substitute_new_content() {
    let mut s = app();
    let mut def = rufren_content::load_reader_anthology()
        .unwrap()
        .documents
        .remove(0);
    let id = reader_document(&def).unwrap().document_id;
    def.title.push_str(" changed");
    s.repo
        .append(&[reader_document_event(&def).unwrap()])
        .unwrap();
    s.refresh().unwrap();
    assert!(s.reader_definition(id).is_err());
    assert!(s.reader_shelf().is_err());
    assert!(s.reader_state().unwrap().is_none());
}

#[test]
fn reader_many_short_paragraphs_fit_bounded_chunks_without_losing_source_bytes() {
    let mut s = app();
    let original = "Короткая строка.\n\n".repeat(100);
    let doc = s
        .import_reader("Many synthetic paragraphs", &original)
        .unwrap();
    let def = s.reader_definition(doc).unwrap();
    assert!(def.chunks.len() <= 64);
    assert_eq!(
        def.chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<String>(),
        original
    );
}

#[test]
fn reader_advertised_paste_limit_includes_serialized_chunk_duplication() {
    let mut s = app();
    let original = "Я".repeat(READER_MAX_ORIGINAL_BYTES / 2);
    assert_eq!(original.len(), READER_MAX_ORIGINAL_BYTES);
    let doc = s
        .import_reader("At the advertised source limit", &original)
        .unwrap();
    assert_eq!(s.reader_definition(doc).unwrap().original, original);
    assert!(
        s.edit_reader_import_draft("Too long", &format!("{original}Я"), "")
            .is_err()
    );
    assert_eq!(s.reader_state().unwrap().unwrap().document_id, doc);
    assert!(s.reader_pending_import.is_some());
}

#[test]
fn reader_kept_phrase_can_gain_meaning_and_retain_its_original_source() {
    let mut state = app();
    let doc = imported(&mut state);
    let definition = state.reader_definition(doc).unwrap();
    let chunk = &definition.chunks[0];
    let original = state.keep_reader_phrase(doc, &chunk.id, "Ёж").unwrap();
    let frozen = state.library.events[&original].clone();
    state.edit(original).unwrap();
    state
        .update_editor(editor_drafts::EditorField::Translation, "A hedgehog".into())
        .unwrap();
    state.save_passage(false).unwrap();
    let edited = state.editor.parent.unwrap();
    assert_ne!(edited, original);
    assert_eq!(state.library.events[&original], frozen);
    assert_eq!(
        state.library.revision(edited).unwrap().passage.translation,
        "A hedgehog"
    );
    let sources = state.reader_sources_for_revision(edited);
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].document_id, doc);
    assert_eq!(sources[0].text, "Ёж");
    state.start(edited, Skill::Reading).unwrap();
    assert_eq!(count_attempts(&state), 0);
}

#[test]
fn reader_dialogue_return_restores_exact_chunk_on_an_empty_client() {
    let mut producer = app();
    let doc = imported(&mut producer);
    let definition = producer.reader_definition(doc).unwrap();
    let chunk = &definition.chunks[2];
    producer
        .prepare_reader_question(doc, &chunk.id, "What does this sentence mean?")
        .unwrap();
    let anchor = producer
        .active_dialogue_draft()
        .unwrap()
        .unwrap()
        .anchor
        .unwrap();
    let mut consumer = app();
    consumer
        .repo
        .append(
            &producer
                .library
                .events
                .values()
                .cloned()
                .collect::<Vec<_>>(),
        )
        .unwrap();
    consumer.refresh().unwrap();
    assert!(consumer.reader_state().unwrap().is_none());
    let before = consumer.library.events.clone();
    consumer.return_to_dialogue_source(&anchor).unwrap();
    assert!(consumer.page == model::Page::Reader);
    let restored = consumer.reader_state().unwrap().unwrap();
    assert_eq!(restored.document_id, doc);
    assert_eq!(restored.chunk_id, chunk.id);
    assert!(restored.own_words.is_empty());
    assert_eq!(consumer.library.events, before);
    let mut changed = anchor;
    changed.snapshot.push_str("An invented continuation.");
    assert!(consumer.return_to_dialogue_source(&changed).is_err());
    assert_eq!(consumer.reader_state().unwrap().unwrap(), restored);
}

#[test]
fn reader_current_help_reopens_after_reload_and_failed_close_remains_visible() {
    let disk = Disk::new();
    let mut s = disk.app();
    let def = rufren_content::load_reader_anthology()
        .unwrap()
        .documents
        .into_iter()
        .find(|d| {
            d.chunks
                .iter()
                .any(|c| c.translation.is_some() && !c.glosses.is_empty())
        })
        .unwrap();
    let doc = reader_document(&def).unwrap().document_id;
    let chunk = def
        .chunks
        .iter()
        .find(|c| c.translation.is_some() && !c.glosses.is_empty())
        .unwrap()
        .id
        .as_str();
    s.open_reader_document(doc, Some(chunk)).unwrap();
    s.set_reader_support(doc, chunk, ReaderSupportKind::Meaning, true)
        .unwrap();
    s.set_reader_support(doc, chunk, ReaderSupportKind::Gloss, true)
        .unwrap();
    let mut reopened = disk.app();
    assert!(
        reopened.reader_state().unwrap().unwrap().supports[chunk]
            .contains(&ReaderSupportKind::Meaning)
    );
    let key = format!("reader_state:{doc}");
    disk.sql(&format!("CREATE TRIGGER fail_help_close BEFORE UPDATE ON metadata WHEN NEW.key='{key}' BEGIN SELECT RAISE(ABORT,'synthetic help close failure'); END;"));
    assert!(
        reopened
            .set_reader_support(doc, chunk, ReaderSupportKind::Meaning, false)
            .is_err()
    );
    assert!(
        reopened.reader_state().unwrap().unwrap().supports[chunk]
            .contains(&ReaderSupportKind::Meaning)
    );
    disk.sql("DROP TRIGGER fail_help_close;");
    reopened
        .set_reader_support(doc, chunk, ReaderSupportKind::Meaning, false)
        .unwrap();
    let restored = disk.app().reader_state().unwrap().unwrap();
    assert!(!restored.supports[chunk].contains(&ReaderSupportKind::Meaning));
    assert!(restored.supports[chunk].contains(&ReaderSupportKind::Gloss));
    reopened
        .set_reader_support(doc, chunk, ReaderSupportKind::Gloss, false)
        .unwrap();
    assert!(
        !disk
            .app()
            .reader_state()
            .unwrap()
            .unwrap()
            .supports
            .contains_key(chunk)
    );
    assert_eq!(count_attempts(&reopened), 0);
}
