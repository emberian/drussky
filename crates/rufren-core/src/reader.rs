//! Immutable supported reading. Original source, AI assistance, audio derivatives
//! and explicitly kept phrases are distinct; none creates a practice attempt.
use crate::*;
use rufren_tutor_contract::{TeacherDialogueAnchor, TeacherDialogueFocus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const READER_MAX_ORIGINAL_BYTES: usize = 65_536;
pub const READER_MAX_DOCUMENT_BYTES: usize = 524_288;
const AUDIO_PREFIX: &str = "Rufren Reader audio reference · ";
const EXCERPT_PREFIX: &str = "Rufren Reader excerpt · ";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReaderSourceKind {
    AiAuthored,
    AuthenticExcerpt,
    LearnerProvided,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderSource {
    pub kind: ReaderSourceKind,
    pub title: String,
    pub author: String,
    pub edition: String,
    pub url: Option<String>,
    pub rights: String,
    pub provenance: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderNote {
    /// GLOBAL UTF-8 byte coordinates in the unchanged original, inside its chunk.
    pub span: TextSpan,
    pub text: String,
    pub explanation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderPrompt {
    pub id: String,
    pub question: String,
    pub teaching_note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderChunk {
    pub id: String,
    pub span: TextSpan,
    pub text: String,
    pub stress: Option<String>,
    pub translation: Option<String>,
    pub glosses: Vec<ReaderNote>,
    pub grammar_notes: Vec<ReaderNote>,
    pub prompts: Vec<ReaderPrompt>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderDefinition {
    pub id: String,
    pub version: String,
    pub title: String,
    pub original: String,
    pub original_sha256: String,
    pub source: ReaderSource,
    /// All supplied stress, translation, gloss, grammar and prompt help is
    /// AI-authored assistance, separately attributed from the source's authorship.
    pub help_provenance: String,
    pub chunks: Vec<ReaderChunk>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderDocument {
    pub document_id: Uuid,
    pub definition: ReaderDefinition,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReaderTextLayer {
    Original,
    StressScaffold,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReaderExcerptPurpose {
    KeptPhrase,
    AudioReference,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderExcerptLink {
    pub document_id: Uuid,
    pub span: TextSpan,
    pub chunk_id: Option<String>,
    pub layer: ReaderTextLayer,
    pub purpose: ReaderExcerptPurpose,
    pub text: String,
    pub revision: Uuid,
}

fn invalid() -> Error {
    Error::Invalid("Reader source, UTF-8 chunk, derivative or exact kept-phrase link does not match its immutable document".into())
}
fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty()
        && s.len() <= max
        && !s
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t' | '\r'))
}
fn identity(s: &str) -> bool {
    bounded(s, 128)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn reader_text_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn exact_span<'a>(text: &'a str, span: TextSpan) -> Result<&'a str, Error> {
    if span.start >= span.end
        || span.end > text.len()
        || !text.is_char_boundary(span.start)
        || !text.is_char_boundary(span.end)
    {
        return Err(invalid());
    }
    Ok(&text[span.start..span.end])
}
impl ReaderDefinition {
    pub fn chunk(&self, id: &str) -> Option<&ReaderChunk> {
        self.chunks.iter().find(|c| c.id == id)
    }
    pub fn validate(&self) -> Result<(), Error> {
        if !identity(&self.id)
            || !identity(&self.version)
            || !bounded(&self.title, 256)
            || !bounded(&self.original, READER_MAX_ORIGINAL_BYTES)
            || self.original_sha256 != reader_text_sha256(&self.original)
            || !bounded(&self.help_provenance, 2048)
            || self.chunks.is_empty()
            || self.chunks.len() > 64
        {
            return Err(invalid());
        }
        let source = &self.source;
        if !bounded(&source.title, 512)
            || !bounded(&source.author, 256)
            || !bounded(&source.edition, 512)
            || !bounded(&source.rights, 1024)
            || !bounded(&source.provenance, 2048)
            || source.url.as_ref().is_some_and(|url| {
                !bounded(url, 1024)
                    || !url.starts_with("https://") && !url.starts_with("http://")
                    || url.chars().any(char::is_whitespace)
            })
            || (source.kind == ReaderSourceKind::AuthenticExcerpt && source.url.is_none())
            || serde_json::to_vec(source).map_err(|_| invalid())?.len() > 2048
        {
            return Err(invalid());
        }
        let mut end = 0;
        let mut ids = BTreeSet::new();
        let mut prompt_ids = BTreeSet::new();
        for chunk in &self.chunks {
            if !identity(&chunk.id)
                || !ids.insert(&chunk.id)
                || chunk.span.start != end
                || chunk.text != exact_span(&self.original, chunk.span)?
                || chunk.text.len() > 4096
                || chunk.stress.as_ref().is_some_and(|stress| {
                    stress.len() > 8192
                        || stress
                            .chars()
                            .filter(|c| *c != '\u{301}')
                            .collect::<String>()
                            != chunk.text
                })
                || chunk
                    .translation
                    .as_ref()
                    .is_some_and(|t| !bounded(t, 4096))
                || chunk.glosses.len() > 24
                || chunk.grammar_notes.len() > 16
                || chunk.prompts.len() > 8
            {
                return Err(invalid());
            }
            for note in chunk.glosses.iter().chain(&chunk.grammar_notes) {
                if note.span.start < chunk.span.start
                    || note.span.end > chunk.span.end
                    || note.text != exact_span(&self.original, note.span)?
                    || !bounded(&note.explanation, 2048)
                {
                    return Err(invalid());
                }
            }
            for prompt in &chunk.prompts {
                if !identity(&prompt.id)
                    || !prompt_ids.insert(&prompt.id)
                    || !bounded(&prompt.question, 2048)
                    || !bounded(&prompt.teaching_note, 2048)
                {
                    return Err(invalid());
                }
            }
            end = chunk.span.end;
        }
        if end != self.original.len()
            || serde_json::to_vec(self).map_err(|_| invalid())?.len() > READER_MAX_DOCUMENT_BYTES
        {
            return Err(invalid());
        }
        Ok(())
    }
}
pub fn reader_document_identity(id: &str, version: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("rufren:reader:{id}:{version}").as_bytes(),
    )
}
pub fn reader_document(definition: &ReaderDefinition) -> Result<ReaderDocument, Error> {
    definition.validate()?;
    Ok(ReaderDocument {
        document_id: reader_document_identity(&definition.id, &definition.version),
        definition: definition.clone(),
    })
}
pub fn reader_document_event(definition: &ReaderDefinition) -> Result<Event, Error> {
    let document = reader_document(definition)?;
    let id = document.document_id;
    Ok(Event {
        version: 7,
        id,
        device: Uuid::new_v5(&id, b"reader-document-writer"),
        recorded_at_ms: 0,
        fact: Fact::ReaderDocumentRecorded(document),
    })
}
impl ReaderDocument {
    pub fn validate(&self) -> Result<(), Error> {
        self.definition.validate()?;
        if self.document_id
            != reader_document_identity(&self.definition.id, &self.definition.version)
        {
            Err(invalid())
        } else {
            Ok(())
        }
    }
}
pub fn is_reader_audio_reference(event: &Event) -> bool {
    matches!(&event.fact,Fact::PassageRevised(r) if r.passage.source.starts_with(AUDIO_PREFIX))
}
pub(crate) fn is_reader_kept_excerpt(event: &Event) -> bool {
    matches!(&event.fact,Fact::PassageRevised(r) if r.passage.source.starts_with(EXCERPT_PREFIX))
}
pub(crate) fn reader_dialogue_focus(focus: &TeacherDialogueFocus) -> bool {
    matches!(
        focus,
        TeacherDialogueFocus::ReaderWhole | TeacherDialogueFocus::ReaderChunk { .. }
    )
}

fn link_identity(link: &ReaderExcerptLink) -> (Uuid, Uuid, Uuid) {
    let purpose = match link.purpose {
        ReaderExcerptPurpose::KeptPhrase => "kept",
        ReaderExcerptPurpose::AudioReference => "audio",
    };
    let layer = match link.layer {
        ReaderTextLayer::Original => "original",
        ReaderTextLayer::StressScaffold => "stress",
    };
    let passage = Uuid::new_v5(
        &link.document_id,
        format!(
            "reader:{purpose}:{layer}:{}:{}:{}",
            link.span.start,
            link.span.end,
            link.chunk_id.as_deref().unwrap_or("")
        )
        .as_bytes(),
    );
    let revision = Uuid::new_v5(&passage, b"reader-reference-v1");
    let fact = Uuid::new_v5(&revision, b"reader-excerpt-link-v1");
    (passage, revision, fact)
}
fn build_excerpt(
    document: &ReaderDocument,
    span: TextSpan,
    chunk_id: Option<String>,
    layer: ReaderTextLayer,
    purpose: ReaderExcerptPurpose,
) -> Result<(Event, Event), Error> {
    document.validate()?;
    let def = &document.definition;
    let original = exact_span(&def.original, span)?;
    let chunk = chunk_id
        .as_ref()
        .map(|id| def.chunk(id).ok_or_else(invalid))
        .transpose()?;
    let text = match (purpose, layer) {
        (ReaderExcerptPurpose::KeptPhrase, ReaderTextLayer::Original) => original.to_owned(),
        (ReaderExcerptPurpose::AudioReference, ReaderTextLayer::Original) => {
            if chunk.is_none_or(|c| c.span != span) {
                return Err(invalid());
            }
            original.to_owned()
        }
        (ReaderExcerptPurpose::AudioReference, ReaderTextLayer::StressScaffold) => {
            let chunk = chunk.filter(|c| c.span == span).ok_or_else(invalid)?;
            chunk.stress.clone().ok_or_else(invalid)?
        }
        _ => return Err(invalid()),
    };
    if !bounded(&text, 8192) {
        return Err(invalid());
    }
    let mut link = ReaderExcerptLink {
        document_id: document.document_id,
        span,
        chunk_id,
        layer,
        purpose,
        text,
        revision: Uuid::nil(),
    };
    let (passage, revision, link_id) = link_identity(&link);
    link.revision = revision;
    let source = match purpose {
        ReaderExcerptPurpose::KeptPhrase => format!(
            "{EXCERPT_PREFIX}{}:{}-{}",
            document.document_id, span.start, span.end
        ),
        ReaderExcerptPurpose::AudioReference => format!(
            "{AUDIO_PREFIX}{}:{}:{}",
            document.document_id,
            link.chunk_id.as_deref().unwrap_or(""),
            match layer {
                ReaderTextLayer::Original => "original",
                ReaderTextLayer::StressScaffold => "stress_scaffold",
            }
        ),
    };
    let translation = chunk
        .filter(|c| c.span == span)
        .and_then(|c| c.translation.clone())
        .unwrap_or_default();
    let revision_event = Event {
        version: 7,
        id: revision,
        device: Uuid::new_v5(&passage, b"reader-excerpt-writer"),
        recorded_at_ms: 0,
        fact: Fact::PassageRevised(Revision {
            parents: BTreeSet::new(),
            passage: Passage {
                id: passage,
                title: def.title.clone(),
                russian: link.text.clone(),
                translation,
                source,
                coach_reviewed: false,
            },
        }),
    };
    let link_event = Event {
        version: 7,
        id: link_id,
        device: Uuid::new_v5(&revision, b"reader-excerpt-link-writer"),
        recorded_at_ms: 0,
        fact: Fact::ReaderExcerptLinked(link),
    };
    Ok((revision_event, link_event))
}
pub fn reader_excerpt_events(
    document: &ReaderDocument,
    span: TextSpan,
) -> Result<(Event, Event), Error> {
    let chunk = document
        .definition
        .chunks
        .iter()
        .find(|c| span.start >= c.span.start && span.end <= c.span.end)
        .map(|c| c.id.clone());
    build_excerpt(
        document,
        span,
        chunk,
        ReaderTextLayer::Original,
        ReaderExcerptPurpose::KeptPhrase,
    )
}
pub fn reader_reference_events(
    document: &ReaderDocument,
    chunk_id: &str,
    layer: ReaderTextLayer,
) -> Result<(Event, Event), Error> {
    let chunk = document.definition.chunk(chunk_id).ok_or_else(invalid)?;
    build_excerpt(
        document,
        chunk.span,
        Some(chunk_id.into()),
        layer,
        ReaderExcerptPurpose::AudioReference,
    )
}

impl Library {
    pub fn reader_document(&self, id: Uuid) -> Option<&ReaderDocument> {
        match &self.events.get(&id)?.fact {
            Fact::ReaderDocumentRecorded(d) => Some(d),
            _ => None,
        }
    }
    pub fn reader_documents(&self) -> Vec<(&Event, &ReaderDocument)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::ReaderDocumentRecorded(d) => Some((e, d)),
                _ => None,
            })
            .collect()
    }
    pub fn reader_excerpt_links(&self, document: Uuid) -> Vec<(&Event, &ReaderExcerptLink)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::ReaderExcerptLinked(l) if l.document_id == document => Some((e, l)),
                _ => None,
            })
            .collect()
    }
    pub fn reader_excerpt_link(&self, revision: Uuid) -> Option<&ReaderExcerptLink> {
        self.events.values().find_map(|e| match &e.fact {
            Fact::ReaderExcerptLinked(link) if link.revision == revision => Some(link),
            _ => None,
        })
    }
}

impl ReaderExcerptLink {
    pub fn validate(&self) -> Result<(), Error> {
        if self.document_id.is_nil()
            || self.revision.is_nil()
            || self.span.start >= self.span.end
            || self.span.end > READER_MAX_ORIGINAL_BYTES
            || !bounded(&self.text, 8192)
            || self.chunk_id.as_ref().is_some_and(|s| !identity(s))
            || (self.layer == ReaderTextLayer::Original
                && self.text.len() != self.span.end - self.span.start)
            || (self.purpose == ReaderExcerptPurpose::KeptPhrase
                && self.layer != ReaderTextLayer::Original)
            || (self.purpose == ReaderExcerptPurpose::AudioReference && self.chunk_id.is_none())
            || self.revision != link_identity(self).1
        {
            return Err(invalid());
        }
        Ok(())
    }
}
pub(crate) fn validate_reader_fact(event: &Event) -> Result<(), Error> {
    if event.version < 7 {
        return Err(Error::Version(event.version));
    }
    match &event.fact {
        Fact::ReaderDocumentRecorded(d) => {
            d.validate()?;
            if event.id != d.document_id
                || event.device != Uuid::new_v5(&d.document_id, b"reader-document-writer")
                || event.recorded_at_ms != 0
            {
                return Err(invalid());
            }
        }
        Fact::ReaderExcerptLinked(link) => {
            link.validate()?;
            if event.id != link_identity(link).2
                || event.device != Uuid::new_v5(&link.revision, b"reader-excerpt-link-writer")
                || event.recorded_at_ms != 0
            {
                return Err(invalid());
            }
        }
        Fact::PassageRevised(r)
            if is_reader_audio_reference(event) || is_reader_kept_excerpt(event) =>
        {
            if (is_reader_audio_reference(event) || r.parents.is_empty())
                && (r.passage.coach_reviewed || !r.parents.is_empty() || event.recorded_at_ms != 0)
            {
                return Err(invalid());
            }
        }
        _ => (),
    }
    Ok(())
}
pub(crate) fn validate_reader_identities(events: &BTreeMap<Uuid, Event>) -> Result<(), Error> {
    let mut documents = BTreeMap::new();
    let mut links = BTreeMap::new();
    for e in events.values() {
        match &e.fact {
            Fact::ReaderDocumentRecorded(d) => {
                if documents
                    .insert(d.document_id, d)
                    .is_some_and(|old| old != d)
                {
                    return Err(invalid());
                }
            }
            Fact::ReaderExcerptLinked(l) => {
                if links.insert(l.revision, l).is_some_and(|old| old != l) {
                    return Err(invalid());
                }
            }
            _ => (),
        }
    }
    Ok(())
}
pub(crate) fn validate_reader_references(
    event: &Event,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    let Fact::ReaderExcerptLinked(link) = &event.fact else {
        return Ok(());
    };
    let (passage, revision, _) = link_identity(link);
    if let Some(target) = events.get(&revision) {
        match &target.fact {
            Fact::PassageRevised(r)
                if r.passage.id == passage
                    && r.passage.russian == link.text
                    && !r.passage.coach_reviewed
                    && r.parents.is_empty()
                    && target.recorded_at_ms == 0
                    && target.device == Uuid::new_v5(&passage, b"reader-excerpt-writer") =>
            {
                let expected = match link.purpose {
                    ReaderExcerptPurpose::KeptPhrase => format!(
                        "{EXCERPT_PREFIX}{}:{}-{}",
                        link.document_id, link.span.start, link.span.end
                    ),
                    ReaderExcerptPurpose::AudioReference => format!(
                        "{AUDIO_PREFIX}{}:{}:{}",
                        link.document_id,
                        link.chunk_id.as_deref().unwrap_or(""),
                        match link.layer {
                            ReaderTextLayer::Original => "original",
                            ReaderTextLayer::StressScaffold => "stress_scaffold",
                        }
                    ),
                };
                if r.passage.source != expected {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
    }
    if let Some(source) = events.get(&link.document_id) {
        let Fact::ReaderDocumentRecorded(document) = &source.fact else {
            return Err(invalid());
        };
        let expected = match link.purpose {
            ReaderExcerptPurpose::KeptPhrase => reader_excerpt_events(document, link.span)?,
            ReaderExcerptPurpose::AudioReference => reader_reference_events(
                document,
                link.chunk_id.as_deref().ok_or_else(invalid)?,
                link.layer,
            )?,
        };
        if event != &expected.1 || events.get(&link.revision).is_some_and(|e| e != &expected.0) {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Selected literal source has priority over every optional context/support item.
/// The complete source remains durable even when a whole-view anchor is partial.
pub(crate) fn reader_anchor_in(
    events: &BTreeMap<Uuid, Event>,
    document_id: Uuid,
    focus: TeacherDialogueFocus,
) -> Result<TeacherDialogueAnchor, Error> {
    let document = match events.get(&document_id).map(|e| &e.fact) {
        Some(Fact::ReaderDocumentRecorded(d)) => d,
        _ => return Err(invalid()),
    };
    document.validate()?;
    let d = &document.definition;
    let source = serde_json::to_string(&d.source).map_err(|_| invalid())?;
    let mut snapshot = format!(
        "Rufren Reader source v1\nDocument: {}@{}\nDocument UUID: {}\nTitle: {}\nOriginal UTF-8 bytes: {}\nOriginal SHA256: {}\nExact source metadata: {}\n",
        d.id,
        d.version,
        document_id,
        d.title,
        d.original.len(),
        d.original_sha256,
        source
    );
    let selected = match &focus {
        TeacherDialogueFocus::ReaderWhole => None,
        TeacherDialogueFocus::ReaderChunk { chunk_id } => {
            Some(d.chunk(chunk_id).ok_or_else(invalid)?)
        }
        _ => return Err(invalid()),
    };
    if let Some(chunk) = selected {
        snapshot.push_str(&format!(
            "Selected ORIGINAL chunk {} [UTF-8 bytes {}..{}]:\n",
            chunk.id, chunk.span.start, chunk.span.end
        ));
        snapshot.push_str(&chunk.text);
        snapshot.push('\n');
    } else {
        let mut end = d.original.len().min(4096);
        while !d.original.is_char_boundary(end) {
            end -= 1;
        }
        if end < d.original.len() {
            snapshot.push_str(&format!("Whole-passage view: bounded ORIGINAL excerpt [UTF-8 bytes 0..{end} of {}]. Omitted source was NOT supplied; ask the learner to choose a narrower chunk before discussing it.\n",d.original.len()));
        } else {
            snapshot.push_str("Whole ORIGINAL passage (complete):\n");
        }
        snapshot.push_str(&d.original[..end]);
        snapshot.push('\n');
    }
    if snapshot.len() > TEACHER_DIALOGUE_ANCHOR_BYTES {
        return Err(invalid());
    }
    let mut omitted = false;
    let append = |snapshot: &mut String, item: String, omitted: &mut bool| {
        if snapshot.len() + item.len() + 100 <= TEACHER_DIALOGUE_ANCHOR_BYTES {
            snapshot.push_str(&item);
        } else {
            *omitted = true;
        }
    };
    if let Some(chunk) = selected {
        let mut before = chunk.span.start.saturating_sub(768);
        while !d.original.is_char_boundary(before) {
            before += 1;
        }
        let mut after = (chunk.span.end + 768).min(d.original.len());
        while !d.original.is_char_boundary(after) {
            after -= 1;
        }
        if before < chunk.span.start {
            append(
                &mut snapshot,
                format!(
                    "Surrounding ORIGINAL context before selected chunk [bytes {before}..{}]:\n{}\n",
                    chunk.span.start,
                    &d.original[before..chunk.span.start]
                ),
                &mut omitted,
            );
        }
        if after > chunk.span.end {
            append(
                &mut snapshot,
                format!(
                    "Surrounding ORIGINAL context after selected chunk [bytes {}..{after}]:\n{}\n",
                    chunk.span.end,
                    &d.original[chunk.span.end..after]
                ),
                &mut omitted,
            );
        }
    }
    append(
        &mut snapshot,
        format!(
            "AI-authored assistance provenance (not original source): {}\n",
            d.help_provenance
        ),
        &mut omitted,
    );
    let chunks: Vec<_> = selected
        .into_iter()
        .chain(if selected.is_none() {
            d.chunks.iter().collect::<Vec<_>>()
        } else {
            Vec::new()
        })
        .collect();
    for chunk in chunks {
        if let Some(stress) = &chunk.stress {
            append(
                &mut snapshot,
                format!(
                    "AI stress scaffold for {} (derivative, not original):\n{}\n",
                    chunk.id, stress
                ),
                &mut omitted,
            );
        }
        if let Some(translation) = &chunk.translation {
            append(
                &mut snapshot,
                format!("AI translation for {}:\n{}\n", chunk.id, translation),
                &mut omitted,
            );
        }
        for (label, notes) in [
            ("AI scoped gloss", &chunk.glosses),
            ("AI scoped grammar note", &chunk.grammar_notes),
        ] {
            for note in notes {
                append(
                    &mut snapshot,
                    format!(
                        "{label} [original bytes {}..{}] {:?}: {}\n",
                        note.span.start, note.span.end, note.text, note.explanation
                    ),
                    &mut omitted,
                );
            }
        }
        for prompt in &chunk.prompts {
            append(
                &mut snapshot,
                format!(
                    "Optional AI comprehension prompt {}: {}\nTeaching note: {}\n",
                    prompt.id, prompt.question, prompt.teaching_note
                ),
                &mut omitted,
            );
        }
    }
    if omitted {
        snapshot.push_str(
            "[Further support/context omitted; selected original source above is unchanged.]\n",
        );
    }
    Ok(TeacherDialogueAnchor {
        document_id,
        focus,
        snapshot_version: 1,
        snapshot,
    })
}
