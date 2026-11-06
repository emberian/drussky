//! Offline anthology definitions, independent of the lesson pack's historical IDs.
//! Core owns the Reader schema, original/chunk rules, hashes, and deterministic
//! document identity. This wrapper adds catalog lookup and anthology invariants.
use crate::ContentError;
use rufren_core::{
    Event, ReaderDefinition, ReaderDocument, reader_document, reader_document_event,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const READER_ANTHOLOGY_SCHEMA_VERSION: u32 = 1;
pub const READER_ANTHOLOGY_JSON: &str = include_str!("../content/reader-v1.json");
const MAX_ANTHOLOGY_DOCUMENTS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReaderAnthology {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub documents: Vec<ReaderDefinition>,
}

fn invalid(message: impl Into<String>) -> ContentError {
    ContentError::Invalid(vec![message.into()])
}
fn core_error(error: rufren_core::Error) -> ContentError {
    invalid(format!("Reader definition: {error}"))
}

pub fn load_reader_anthology() -> Result<ReaderAnthology, ContentError> {
    ReaderAnthology::from_json(READER_ANTHOLOGY_JSON)
}

impl ReaderAnthology {
    pub fn from_json(json: &str) -> Result<Self, ContentError> {
        let anthology: Self = serde_json::from_str(json).map_err(ContentError::Json)?;
        anthology.validate()?;
        Ok(anthology)
    }

    pub fn validate(&self) -> Result<(), ContentError> {
        if self.schema_version != READER_ANTHOLOGY_SCHEMA_VERSION {
            return Err(ContentError::UnsupportedSchema(self.schema_version));
        }
        if !self.id.starts_with("ru.reader.")
            || self.id.len() <= "ru.reader.".len()
            || self.id.len() > 128
            || self.id.chars().any(|c| c.is_control() || c.is_whitespace())
            || self.version.trim().is_empty()
            || self.version.trim() != self.version
            || self.version.len() > 128
            || self
                .version
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || self.documents.is_empty()
            || self.documents.len() > MAX_ANTHOLOGY_DOCUMENTS
        {
            return Err(invalid(
                "Invalid Reader anthology identity, version, or document count",
            ));
        }
        let mut keys = BTreeSet::new();
        let mut document_ids = BTreeSet::new();
        for definition in &self.documents {
            definition.validate().map_err(core_error)?;
            if !keys.insert((&definition.id, &definition.version)) {
                return Err(invalid(format!(
                    "Duplicate Reader definition {}@{}",
                    definition.id, definition.version
                )));
            }
            let document = reader_document(definition).map_err(core_error)?;
            if !document_ids.insert(document.document_id) {
                return Err(invalid(
                    "Reader definitions produced the same immutable document identity",
                ));
            }
        }
        Ok(())
    }

    /// Exact authored identity only. A different version is never substituted.
    pub fn definition(&self, id: &str, version: &str) -> Option<&ReaderDefinition> {
        self.documents
            .iter()
            .find(|definition| definition.id == id && definition.version == version)
    }

    pub fn document(&self, id: &str, version: &str) -> Result<ReaderDocument, ContentError> {
        let definition = self
            .definition(id, version)
            .ok_or_else(|| invalid(format!("Reader definition unavailable: {id}@{version}")))?;
        reader_document(definition).map_err(core_error)
    }

    pub fn document_event(&self, id: &str, version: &str) -> Result<Event, ContentError> {
        let definition = self
            .definition(id, version)
            .ok_or_else(|| invalid(format!("Reader definition unavailable: {id}@{version}")))?;
        reader_document_event(definition).map_err(core_error)
    }

    pub fn document_events(&self) -> Result<Vec<Event>, ContentError> {
        self.validate()?;
        self.documents
            .iter()
            .map(|definition| reader_document_event(definition).map_err(core_error))
            .collect()
    }
}
