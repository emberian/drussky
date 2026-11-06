//! Saved originals are shared facts; reading position, help and personal notes stay local.
use crate::{model::AppState, repository::Result};
use rufren_core::*;
use rufren_tutor::{DialogueAction, DialogueConfig, DialogueIntent, TeacherDialogueFocus};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const SELECTION: &str = "reader_selection";
const IMPORT: &str = "reader_import_draft";
fn state_key(id: Uuid) -> String {
    format!("reader_state:{id}")
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReaderMode {
    #[default]
    Continuous,
    Focus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReaderSupportKind {
    Meaning,
    Gloss,
    Grammar,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReaderState {
    pub document_id: Uuid,
    pub chunk_id: String,
    pub mode: ReaderMode,
    pub show_stress: bool,
    pub supports: BTreeMap<String, BTreeSet<ReaderSupportKind>>,
    pub own_words: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReaderImportDraft {
    pub title: String,
    pub original: String,
    #[serde(default)]
    pub source_note: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReaderShelfEntry {
    pub document_id: Uuid,
    pub title: String,
    pub source_title: String,
    pub imported: bool,
}
impl AppState {
    /// Retry unsaved visible edits before leaving their surface. Each pending
    /// value survives failures; only an acknowledged local write clears it.
    pub(crate) fn flush_pending_reader_edits(&mut self) -> Result<()> {
        if let Some(draft) = self.reader_pending_import.clone() {
            self.save_reader_import_draft_with_source(
                &draft.title,
                &draft.original,
                &draft.source_note,
            )?;
            self.reader_pending_import = None;
        }
        let docs: BTreeSet<Uuid> = self
            .reader_pending_edits
            .keys()
            .map(|(doc, _)| *doc)
            .collect();
        for doc in docs {
            let def = self.reader_definition(doc)?;
            let mut state = self.reader_local(doc)?;
            let edits: Vec<_> = self
                .reader_pending_edits
                .iter()
                .filter(|((id, _), _)| *id == doc)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            for ((_, chunk), text) in &edits {
                if text.len() > 8192 || def.chunk(chunk).is_none() {
                    return Err("Keep your note under 8 KiB in an existing chunk. Your unsaved words remain here.".into());
                }
                state.own_words.insert(chunk.clone(), text.clone());
            }
            self.repo.set(&state_key(doc), &state)?;
            for (key, _) in edits {
                self.reader_pending_edits.remove(&key);
            }
        }
        Ok(())
    }
    pub(crate) fn edit_reader_own_words(
        &mut self,
        doc: Uuid,
        chunk: &str,
        text: &str,
    ) -> Result<()> {
        if self.reader_definition(doc)?.chunk(chunk).is_none() {
            return Err("That exact chunk is unavailable.".into());
        }
        self.reader_pending_edits
            .insert((doc, chunk.into()), text.into());
        self.flush_pending_reader_edits()
    }
    pub(crate) fn edit_reader_import_draft(
        &mut self,
        title: &str,
        original: &str,
        source_note: &str,
    ) -> Result<()> {
        self.reader_pending_import = Some(ReaderImportDraft {
            title: title.into(),
            original: original.into(),
            source_note: source_note.into(),
        });
        self.flush_pending_reader_edits()
    }
    /// An edited phrase retains the exact saved extraction in its ancestry.
    pub(crate) fn reader_sources_for_revision(&self, revision: Uuid) -> Vec<ReaderExcerptLink> {
        let mut todo = vec![revision];
        let mut visited = BTreeSet::new();
        let mut sources = BTreeMap::new();
        while let Some(id) = todo.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(link) = self.library.reader_excerpt_link(id) {
                sources.insert(
                    (link.document_id, link.span.start, link.span.end),
                    link.clone(),
                );
            }
            if let Some(saved) = self.library.revision(id) {
                todo.extend(saved.parents.iter().copied());
            }
        }
        sources.into_values().collect()
    }
    pub(crate) fn reader_shelf(&self) -> Result<Vec<ReaderShelfEntry>> {
        let library = self.repo.library()?;
        let mut entries = BTreeMap::new();
        for (_, doc) in library.reader_documents() {
            entries.insert(
                doc.document_id,
                ReaderShelfEntry {
                    document_id: doc.document_id,
                    title: doc.definition.title.clone(),
                    source_title: doc.definition.source.title.clone(),
                    imported: true,
                },
            );
        }
        let mut shelf = Vec::new();
        for def in rufren_content::load_reader_anthology()
            .map_err(|e| e.to_string())?
            .documents
        {
            let doc = reader_document(&def).map_err(|e| e.to_string())?;
            if let Some(saved) = library.reader_document(doc.document_id) {
                if saved.definition != def {
                    return Err("The embedded anthology conflicts with an already saved original. Neither has been substituted.".into());
                }
            }
            shelf.push(
                entries
                    .remove(&doc.document_id)
                    .unwrap_or(ReaderShelfEntry {
                        document_id: doc.document_id,
                        title: def.title,
                        source_title: def.source.title,
                        imported: false,
                    }),
            );
        }
        let mut personal: Vec<_> = entries.into_values().collect();
        personal.sort_by(|a, b| {
            a.title
                .cmp(&b.title)
                .then(a.document_id.cmp(&b.document_id))
        });
        shelf.extend(personal);
        Ok(shelf)
    }
    pub(crate) fn reader_definition(&self, id: Uuid) -> Result<ReaderDefinition> {
        let library = self.repo.library()?;
        let saved = library.reader_document(id);
        for def in rufren_content::load_reader_anthology()
            .map_err(|e| e.to_string())?
            .documents
        {
            if reader_document(&def)
                .map_err(|e| e.to_string())?
                .document_id
                == id
            {
                if let Some(saved) = saved {
                    if saved.definition != def {
                        return Err("The embedded anthology conflicts with this saved original. Neither has been substituted.".into());
                    }
                }
                return Ok(def);
            }
        }
        if let Some(saved) = saved {
            return Ok(saved.definition.clone());
        }
        Err("This exact reading is unavailable. Sync its original before opening it.".into())
    }
    fn reader_local(&self, id: Uuid) -> Result<ReaderState> {
        let def = self.reader_definition(id)?;
        let state = self
            .repo
            .get::<ReaderState>(&state_key(id))?
            .unwrap_or(ReaderState {
                document_id: id,
                chunk_id: def
                    .chunks
                    .first()
                    .ok_or("This reading has no chunks.")?
                    .id
                    .clone(),
                mode: ReaderMode::Continuous,
                show_stress: false,
                supports: BTreeMap::new(),
                own_words: BTreeMap::new(),
            });
        if state.document_id != id
            || !def.chunks.iter().any(|c| c.id == state.chunk_id)
            || state
                .supports
                .keys()
                .chain(state.own_words.keys())
                .any(|key| !def.chunks.iter().any(|c| &c.id == key))
            || state.own_words.values().any(|s| s.len() > 8192)
        {
            return Err(
                "The saved reading position does not match this original. Your text has been kept."
                    .into(),
            );
        }
        Ok(state)
    }
    pub(crate) fn reader_state(&self) -> Result<Option<ReaderState>> {
        self.repo
            .get::<Uuid>(SELECTION)?
            .map(|id| {
                let mut state = self.reader_local(id)?;
                for ((doc, chunk), text) in &self.reader_pending_edits {
                    if *doc == id {
                        state.own_words.insert(chunk.clone(), text.clone());
                    }
                }
                Ok(state)
            })
            .transpose()
    }
    pub(crate) fn open_reader_document(
        &mut self,
        id: Uuid,
        chunk: Option<&str>,
    ) -> Result<ReaderState> {
        self.flush_pending_reader_edits()?;
        let def = self.reader_definition(id)?;
        let mut state = self.reader_local(id)?;
        if let Some(chunk) = chunk {
            if !def.chunks.iter().any(|c| c.id == chunk) {
                return Err("That chunk is not part of this saved original.".into());
            }
            state.chunk_id = chunk.into();
        }
        self.note_reference_access()?;
        self.repo
            .append(&[reader_document_event(&def).map_err(|e| e.to_string())?])?;
        self.repo.set(&state_key(id), &state)?;
        self.repo.set(SELECTION, &id)?;
        self.refresh()?;
        Ok(state)
    }
    fn edit_reader(
        &mut self,
        edit: impl FnOnce(&ReaderDefinition, &mut ReaderState) -> Result<()>,
    ) -> Result<()> {
        self.flush_pending_reader_edits()?;
        let mut state = self.reader_state()?.ok_or("Open a reading first.")?;
        let def = self.reader_definition(state.document_id)?;
        edit(&def, &mut state)?;
        self.note_reference_access()?;
        self.repo.set(&state_key(state.document_id), &state)
    }
    pub(crate) fn set_reader_mode(&mut self, mode: ReaderMode) -> Result<()> {
        self.edit_reader(|_, s| {
            s.mode = mode;
            Ok(())
        })
    }
    pub(crate) fn set_reader_stress(&mut self, show: bool) -> Result<()> {
        self.edit_reader(|_, s| {
            s.show_stress = show;
            Ok(())
        })
    }
    pub(crate) fn set_reader_chunk(&mut self, chunk: &str) -> Result<()> {
        self.edit_reader(|d, s| {
            if !d.chunks.iter().any(|c| c.id == chunk) {
                return Err("That chunk is unavailable.".into());
            }
            s.chunk_id = chunk.into();
            Ok(())
        })
    }
    /// Current panel visibility only. Actual presented help is recorded separately
    /// by the visible-text observer; this local set is not learning evidence.
    pub(crate) fn set_reader_support(
        &mut self,
        doc: Uuid,
        chunk: &str,
        kind: ReaderSupportKind,
        show: bool,
    ) -> Result<()> {
        self.edit_reader(|d, s| {
            if s.document_id != doc || d.chunk(chunk).is_none() {
                return Err("Reopen this exact reading before changing help.".into());
            }
            if show {
                s.supports.entry(chunk.into()).or_default().insert(kind);
            } else if let Some(open) = s.supports.get_mut(chunk) {
                open.remove(&kind);
                if open.is_empty() {
                    s.supports.remove(chunk);
                }
            }
            Ok(())
        })
    }
    pub(crate) fn mark_reader_support(
        &mut self,
        doc: Uuid,
        chunk: &str,
        kind: ReaderSupportKind,
    ) -> Result<()> {
        self.set_reader_support(doc, chunk, kind, true)
    }
    pub(crate) fn save_reader_own_words(&self, doc: Uuid, chunk: &str, text: &str) -> Result<()> {
        if text.len() > 8192 {
            return Err("Keep your note under 8 KiB. Your earlier words are saved.".into());
        }
        let def = self.reader_definition(doc)?;
        if !def.chunks.iter().any(|c| c.id == chunk) {
            return Err("That chunk is unavailable.".into());
        }
        let mut state = self.reader_local(doc)?;
        state.own_words.insert(chunk.into(), text.into());
        self.repo.set(&state_key(doc), &state)
    }
    pub(crate) fn reader_import_draft(&self) -> Result<ReaderImportDraft> {
        if let Some(pending) = &self.reader_pending_import {
            return Ok(pending.clone());
        }
        Ok(self.repo.get(IMPORT)?.unwrap_or_default())
    }
    pub(crate) fn save_reader_import_draft(&self, title: &str, original: &str) -> Result<()> {
        let previous = self.reader_import_draft()?;
        self.save_reader_import_draft_with_source(title, original, &previous.source_note)
    }
    pub(crate) fn save_reader_import_draft_with_source(
        &self,
        title: &str,
        original: &str,
        source_note: &str,
    ) -> Result<()> {
        if title.len() > 256
            || original.len() > READER_MAX_ORIGINAL_BYTES
            || source_note.len() > 1024
        {
            return Err(format!(
                "Use a title under 256 bytes, source note under 1024 bytes and reading under {} bytes. Your earlier draft is kept.",
                READER_MAX_ORIGINAL_BYTES
            ));
        }
        self.repo.set(
            IMPORT,
            &ReaderImportDraft {
                title: title.into(),
                original: original.into(),
                source_note: source_note.into(),
            },
        )
    }
}

// Include separators in the preceding chunk: joining exact chunk text reconstructs
// every byte of the imported original, including CRLF and blank lines.
fn import_chunks(original: &str) -> Vec<ReaderChunk> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    let mut previous_blank = false;
    for line in original.split_inclusive('\n') {
        offset += line.len();
        let blank = line.trim().is_empty();
        if previous_blank && !blank && offset - line.len() > start {
            ranges.push((start, offset - line.len()));
            start = offset - line.len();
        }
        previous_blank = blank;
    }
    if start < original.len() {
        ranges.push((start, original.len()));
    }
    // Long paragraphs still need bounded source chunks. Split at a nearby newline
    // or word boundary where possible, with UTF-8 boundaries as the final fallback.
    // Very fragmented pastes still fit the source-size contract. Coalesce their
    // paragraph boundaries before bounded splitting rather than rejecting valid
    // text merely because it contains more than 64 short paragraphs.
    if ranges.len() > 64 {
        ranges = vec![(0, original.len())];
    }
    let mut bounded = Vec::new();
    for (mut start, end) in ranges {
        while end - start > 3000 {
            let mut cut = start + 3000;
            while !original.is_char_boundary(cut) {
                cut -= 1;
            }
            if let Some((i, _)) = original[start..cut]
                .char_indices()
                .rev()
                .find(|(i, c)| *i > 1500 && c.is_whitespace())
            {
                cut = start + i;
            }
            bounded.push((start, cut));
            start = cut;
        }
        if start < end {
            bounded.push((start, end));
        }
    }
    if bounded.len() > 64 {
        bounded.clear();
        let mut start = 0;
        while start < original.len() {
            let mut end = (start + 3000).min(original.len());
            while !original.is_char_boundary(end) {
                end -= 1;
            }
            bounded.push((start, end));
            start = end;
        }
    }
    bounded
        .into_iter()
        .enumerate()
        .map(|(i, (start, end))| ReaderChunk {
            id: format!("chunk-{}", i + 1),
            span: TextSpan { start, end },
            text: original[start..end].into(),
            stress: None,
            translation: None,
            glosses: Vec::new(),
            grammar_notes: Vec::new(),
            prompts: Vec::new(),
        })
        .collect()
}
impl AppState {
    pub(crate) fn import_reader(&mut self, title: &str, original: &str) -> Result<Uuid> {
        let previous = self.reader_import_draft()?;
        let source = if previous.title == title && previous.original == original {
            previous.source_note
        } else {
            String::new()
        };
        self.import_reader_with_source(title, original, &source)
    }
    pub(crate) fn import_reader_with_source(
        &mut self,
        title: &str,
        original: &str,
        source_note: &str,
    ) -> Result<Uuid> {
        self.flush_pending_reader_edits()?;
        // Save the exact paste before attempting an immutable import. A validation
        // or disk failure leaves both the old reading and this recoverable paste.
        self.save_reader_import_draft_with_source(title, original, source_note)?;
        if title.trim().is_empty() || title.len() > 256 || original.trim().is_empty() {
            return Err("Give this reading a title and some original text.".into());
        }
        let identity = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            serde_json::to_vec(&(title, original, source_note))
                .map_err(|e| e.to_string())?
                .as_slice(),
        );
        let def = ReaderDefinition {
            id: format!("reader.custom.{identity}"),
            version: "1".into(),
            title: title.into(),
            original: original.into(),
            original_sha256: reader_text_sha256(original),
            source: ReaderSource {
                kind: ReaderSourceKind::LearnerProvided,
                title: title.into(),
                author: "Not supplied".into(),
                edition: "Learner-provided copy".into(),
                url: None,
                rights: "Not supplied".into(),
                provenance: format!(
                    "Exact text pasted by the learner; authorship and publication rights have not been independently verified. Learner source note: {}",
                    if source_note.trim().is_empty() {
                        "Not supplied"
                    } else {
                        source_note
                    }
                ),
            },
            help_provenance: "No translation, stress scaffold, gloss, grammar or prompts supplied."
                .into(),
            chunks: import_chunks(original),
        };
        let event = reader_document_event(&def).map_err(|e| e.to_string())?;
        let id = reader_document(&def)
            .map_err(|e| e.to_string())?
            .document_id;
        self.repo.append(&[event])?;
        self.refresh()?;
        self.open_reader_document(id, None)?;
        Ok(id)
    }
    fn exact_reader_document(&self, id: Uuid) -> Result<ReaderDocument> {
        let library = self.repo.library()?;
        library
            .reader_document(id)
            .cloned()
            .ok_or("Open and save this original before keeping a phrase or asking about it.".into())
    }
    pub(crate) fn keep_reader_span(
        &mut self,
        doc: Uuid,
        chunk: &str,
        span: TextSpan,
    ) -> Result<Uuid> {
        self.flush_pending_reader_edits()?;
        let document = self.exact_reader_document(doc)?;
        let source = document
            .definition
            .chunk(chunk)
            .ok_or("That exact chunk is unavailable.")?;
        if span.start < source.span.start || span.end > source.span.end {
            return Err("Choose a phrase entirely inside this original chunk.".into());
        }
        let (revision, link) = reader_excerpt_events(&document, span).map_err(|e| e.to_string())?;
        let id = revision.id;
        self.repo.append(&[revision, link])?;
        self.refresh()?;
        Ok(id)
    }
    pub(crate) fn keep_reader_phrase(
        &mut self,
        doc: Uuid,
        chunk: &str,
        text: &str,
    ) -> Result<Uuid> {
        let definition = self.reader_definition(doc)?;
        let source = definition
            .chunk(chunk)
            .ok_or("That exact chunk is unavailable.")?;
        if text.trim().is_empty() {
            return Err("Choose some unchanged original text first.".into());
        }
        let mut found = source.text.match_indices(text);
        let (start,_)=found.next().ok_or("The selected phrase must match the original exactly, including stress marks and spacing.")?;
        if found.next().is_some() {
            return Err("That phrase occurs more than once here. Choose a longer unique phrase or an exact source span.".into());
        }
        self.keep_reader_span(
            doc,
            chunk,
            TextSpan {
                start: source.span.start + start,
                end: source.span.start + start + text.len(),
            },
        )
    }
    pub(crate) fn prepare_reader_audio(
        &mut self,
        doc: Uuid,
        chunk: &str,
        layer: ReaderTextLayer,
    ) -> Result<Uuid> {
        self.flush_pending_reader_edits()?;
        let document = self.exact_reader_document(doc)?;
        let (revision, link) =
            reader_reference_events(&document, chunk, layer).map_err(|e| e.to_string())?;
        let id = revision.id;
        self.note_reference_access()?;
        self.repo.append(&[revision, link])?;
        self.refresh()?;
        Ok(id)
    }
    pub(crate) fn prepare_reader_question(
        &mut self,
        doc: Uuid,
        chunk: &str,
        text: &str,
    ) -> Result<Uuid> {
        if text.len() > rufren_tutor::DIALOGUE_MAX_TEXT_BYTES {
            return Err("Keep one Teacher question under 2 KiB. Your reading note is kept.".into());
        }
        self.flush_pending_reader_edits()?;
        let document = self.exact_reader_document(doc)?;
        document
            .definition
            .chunk(chunk)
            .ok_or("That exact chunk is unavailable.")?;
        self.note_reference_access()?;
        // This is an editable local composer, never an immutable sent message.
        let draft = self.new_anchored_dialogue_with_text(
            doc,
            TeacherDialogueFocus::ReaderChunk {
                chunk_id: chunk.into(),
            },
            DialogueConfig {
                intent: DialogueIntent::Clarification,
                topic: "Reading a saved text".into(),
                teacher_role: None,
                learner_role: None,
            },
            DialogueAction::Ask,
            text.into(),
        )?;
        Ok(draft.id)
    }
}
