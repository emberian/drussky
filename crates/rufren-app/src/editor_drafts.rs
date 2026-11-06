//! Passage drafts are separate from published revisions and never replace one another.
use crate::{
    model::{AppState, Editor, Page},
    repository::{now_ms, Repository, Result},
};
use rufren_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

const INDEX: &str = "passage_draft_ids";
const ACTIVE: &str = "active_passage_draft";
fn key(id: Uuid) -> String {
    format!("passage_draft.{id}")
}

#[derive(Clone, Copy)]
pub(crate) enum EditorField {
    Title,
    Russian,
    Translation,
    Source,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditorDraft {
    version: u32,
    pub(crate) editor: Editor,
    pub(crate) updated_at_ms: i64,
}
fn validate(editor: &Editor) -> Result<()> {
    if editor.draft_id.is_nil()
        || [
            &editor.title,
            &editor.russian,
            &editor.translation,
            &editor.source,
        ]
        .iter()
        .any(|v| v.len() > MAX_TEXT)
        || editor.id.is_some_and(|id| id.is_nil())
        || editor.parent.is_some_and(|id| id.is_nil())
    {
        return Err(
            "Keep each passage field under 32 KB; split longer text into smaller passages.".into(),
        );
    }
    if let Some(event) = &editor.pending_save {
        event.validate().map_err(|e| e.to_string())?;
        let Fact::PassageRevised(revision) = &event.fact else {
            return Err("The interrupted passage save has invalid data.".into());
        };
        if editor.id.is_some_and(|id| id != revision.passage.id)
            || revision.passage.title != editor.title.trim()
            || revision.passage.russian != editor.russian.trim()
            || revision.passage.translation != editor.translation.trim()
            || revision.passage.source != editor.source.trim()
        {
            return Err(
                "The interrupted passage save does not match its draft. Your saved data was kept."
                    .into(),
            );
        }
    }
    Ok(())
}
fn read(repo: &Repository, id: Uuid) -> Result<EditorDraft> {
    let record = repo
        .get::<EditorDraft>(&key(id))?
        .ok_or("This passage draft is unavailable. Your other drafts are kept.")?;
    if record.version != 1 || record.editor.draft_id != id {
        return Err("This passage draft needs a newer version of Rufren.".into());
    }
    validate(&record.editor)?;
    Ok(record)
}
fn write(repo: &Repository, editor: &Editor) -> Result<()> {
    validate(editor)?;
    // A failed index/pointer write leaves the previous active draft untouched.
    // The caller never switches until this whole sequence has succeeded.
    repo.set(
        &key(editor.draft_id),
        &EditorDraft {
            version: 1,
            editor: editor.clone(),
            updated_at_ms: now_ms(),
        },
    )?;
    let mut ids: BTreeSet<Uuid> = repo.get(INDEX)?.unwrap_or_default();
    if ids.insert(editor.draft_id) {
        repo.set(INDEX, &ids)?;
    }
    Ok(())
}
pub(crate) fn load_editor(repo: &Repository) -> Result<Editor> {
    if let Some(id) = repo.get::<Uuid>(ACTIVE)? {
        return Ok(read(repo, id)?.editor);
    }
    // Recover a first draft if closing interrupted its initial activation after
    // its record/index were written. No existing active draft is displaced.
    let ids: BTreeSet<Uuid> = repo.get(INDEX)?.unwrap_or_default();
    let library = repo.library()?;
    let records = ids
        .into_iter()
        .map(|id| read(repo, id))
        .collect::<Result<Vec<_>>>()?;
    Ok(records
        .into_iter()
        .filter(|r| dirty(&r.editor, &library))
        .max_by_key(|r| (r.updated_at_ms, r.editor.draft_id))
        .map(|r| r.editor)
        .unwrap_or_default())
}
fn dirty(editor: &Editor, library: &Library) -> bool {
    if editor.pending_save.is_some() {
        return true;
    }
    if let Some(revision) = editor.parent.and_then(|id| library.revision(id)) {
        let p = &revision.passage;
        return editor.id != Some(p.id)
            || editor.title != p.title
            || editor.russian != p.russian
            || editor.translation != p.translation
            || editor.source != p.source;
    }
    [
        &editor.title,
        &editor.russian,
        &editor.translation,
        &editor.source,
    ]
    .iter()
    .any(|v| !v.is_empty())
}

impl AppState {
    fn persist_editor(&mut self, editor: Editor) -> Result<()> {
        write(&self.repo, &editor)?;
        if self.repo.get::<Uuid>(ACTIVE)? != Some(editor.draft_id) {
            self.repo.set(ACTIVE, &editor.draft_id)?;
        }
        self.editor = editor;
        Ok(())
    }
    /// AI examples use this same entry point, preserving whatever was open.
    pub(crate) fn open_passage_draft(&mut self, editor: Editor) -> Result<()> {
        self.navigate(Page::Library)?;
        write(&self.repo, &self.editor)?;
        self.persist_editor(editor)
    }
    pub(crate) fn new_passage(&mut self) -> Result<()> {
        self.open_passage_draft(Editor::default())
    }
    pub(crate) fn update_editor(&mut self, field: EditorField, value: String) -> Result<()> {
        if value.len() > MAX_TEXT {
            return Err(
                "Keep each passage field under 32 KB; split longer text into smaller passages."
                    .into(),
            );
        }
        if self.editor.pending_save.is_some() {
            self.save_editor_draft(false)?;
        }
        let mut editor = self.editor.clone();
        match field {
            EditorField::Title => editor.title = value,
            EditorField::Russian => editor.russian = value,
            EditorField::Translation => editor.translation = value,
            EditorField::Source => editor.source = value,
        }
        self.persist_editor(editor)
    }
    pub(crate) fn editor_drafts(&self) -> Result<Vec<EditorDraft>> {
        let library = self.repo.library()?;
        let ids: BTreeSet<Uuid> = self.repo.get(INDEX)?.unwrap_or_default();
        let mut records = Vec::new();
        for id in ids {
            if id == self.editor.draft_id {
                continue;
            }
            let record = read(&self.repo, id)?;
            if dirty(&record.editor, &library) {
                records.push(record);
            }
        }
        records.sort_by_key(|r| (std::cmp::Reverse(r.updated_at_ms), r.editor.draft_id));
        Ok(records)
    }
    pub(crate) fn resume_editor_draft(&mut self, id: Uuid) -> Result<()> {
        let record = read(&self.repo, id)?;
        self.open_passage_draft(record.editor)
    }
    pub(crate) fn edit(&mut self, id: Uuid) -> Result<()> {
        self.refresh()?;
        let passage = self
            .library
            .revision(id)
            .ok_or("That passage revision is unavailable.")?
            .passage
            .clone();
        if self.editor.parent == Some(id) {
            self.navigate(Page::Library)?;
            return Ok(());
        }
        // Selecting a saved passage picks up its existing unfinished edit.
        let draft = self
            .editor_drafts()?
            .into_iter()
            .find(|r| r.editor.parent == Some(id));
        if let Some(record) = draft {
            return self.open_passage_draft(record.editor);
        }
        self.open_passage_draft(Editor {
            id: Some(passage.id),
            parent: Some(id),
            title: passage.title,
            russian: passage.russian,
            translation: passage.translation,
            source: passage.source,
            ..Default::default()
        })
    }
    pub(crate) fn save_editor_draft(&mut self, resolve: bool) -> Result<()> {
        self.refresh()?;
        if self.editor.pending_save.is_none() {
            let already_saved = self.editor.parent.is_some() && !dirty(&self.editor, &self.library);
            let has_conflicts = self
                .editor
                .id
                .and_then(|id| self.library.heads().get(&id).map(|h| h.len() > 1))
                .unwrap_or(false);
            if already_saved && (!resolve || !has_conflicts) {
                self.notice = "This passage is already saved.".into();
                return Ok(());
            }
            let d = &self.editor;
            let id = d.id.unwrap_or_else(Uuid::new_v4);
            let parents = if resolve {
                self.library
                    .heads()
                    .get(&id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                d.parent.into_iter().collect()
            };
            let event = Event::new(
                self.device,
                now_ms(),
                Fact::PassageRevised(Revision {
                    passage: Passage {
                        id,
                        title: d.title.trim().into(),
                        russian: d.russian.trim().into(),
                        translation: d.translation.trim().into(),
                        source: d.source.trim().into(),
                        coach_reviewed: false,
                    },
                    parents,
                }),
            )
            .map_err(|e| e.to_string())?;
            let mut pending = self.editor.clone();
            pending.pending_save = Some(event);
            self.persist_editor(pending)?;
        }
        let event = self
            .editor
            .pending_save
            .clone()
            .ok_or("No passage save is pending.")?;
        self.repo.append(&[event.clone()])?;
        self.refresh()?;
        let Fact::PassageRevised(revision) = &event.fact else {
            unreachable!()
        };
        let p = &revision.passage;
        let saved = Editor {
            draft_id: self.editor.draft_id,
            id: Some(p.id),
            parent: Some(event.id),
            title: p.title.clone(),
            russian: p.russian.clone(),
            translation: p.translation.clone(),
            source: p.source.clone(),
            pending_save: None,
        };
        self.persist_editor(saved)?;
        self.notice = "Passage saved on this device. Your other drafts are kept.".into();
        Ok(())
    }
}
