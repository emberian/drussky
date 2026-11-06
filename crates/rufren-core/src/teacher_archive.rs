//! Immutable AI teaching documents. Saving a document records neither presentation nor learning.
use crate::{Error, Event, Fact, Library};
use rufren_tutor_contract::{ReviewStatus, TeacherRequest, TeacherResponse};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const TEACHER_DOCUMENT_FORMAT_VERSION: u32 = 1;
pub const MAX_TEACHER_DOCUMENT_BYTES: usize = 131_072;
pub const MAX_TEACHER_CONTEXT_SOURCES: usize = 64;

/// This namespace is also the existing generated-practice source identity. Do not change it
/// when the archive, event protocol, or teaching prompt version changes.
pub fn teacher_document_id(response: &TeacherResponse) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "rufren:teacher-practice:{}:{}:{}",
            response.provenance.provider,
            response.provenance.model,
            response.provenance.response_id
        )
        .as_bytes(),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TeacherDocumentReceipt {
    /// The original response was received live at this known local wall-clock time.
    LiveReceived { received_at_ms: i64 },
    /// The cache did not preserve its original receive time. The event time is archive time only.
    RecoveredLocalCache,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TeacherDocument {
    pub format_version: u32,
    pub document_id: Uuid,
    /// Exact accepted request, generated content, source provenance and usage accounting.
    pub response: TeacherResponse,
    pub receipt: TeacherDocumentReceipt,
    /// Exact prepared-context snapshot IDs, when available; never reconstructed from current history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_source_fact_ids: Option<Vec<Uuid>>,
}

impl TeacherDocument {
    pub fn validate(&self) -> Result<(), Error> {
        let invalid = || Error::Invalid("invalid archived teacher document".into());
        if self.format_version != TEACHER_DOCUMENT_FORMAT_VERSION
            || self.document_id != teacher_document_id(&self.response)
        {
            return Err(invalid());
        }
        self.response.request.validate().map_err(|_| invalid())?;
        self.response
            .content
            .validate_for(&self.response.request)
            .map_err(|_| invalid())?;
        let p = &self.response.provenance;
        if p.review_status != ReviewStatus::AiGeneratedTeaching
            || [&p.provider, &p.model, &p.response_id, &p.prompt_version]
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 256 || s.chars().any(char::is_control))
            || p.scope.trim().is_empty()
            || p.scope.len() > 2048
            || matches!(self.receipt, TeacherDocumentReceipt::LiveReceived { received_at_ms } if received_at_ms < 0)
        {
            return Err(invalid());
        }
        if let Some(ids) = &self.context_source_fact_ids {
            if ids.len() > MAX_TEACHER_CONTEXT_SOURCES
                || ids.iter().any(Uuid::is_nil)
                || ids.iter().copied().collect::<BTreeSet<_>>().len() != ids.len()
            {
                return Err(invalid());
            }
        }
        if serde_json::to_vec(self).map_err(|_| invalid())?.len() > MAX_TEACHER_DOCUMENT_BYTES {
            return Err(invalid());
        }
        Ok(())
    }
}

pub(crate) fn validate_teacher_document_identities(
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    let mut documents = BTreeMap::new();
    for event in events.values() {
        if let Fact::TeacherDocumentRecorded(document) = &event.fact {
            if documents
                .insert(document.document_id, &document.response)
                .is_some_and(|old| old != &document.response)
            {
                return Err(Error::Invalid(
                    "teacher document source reused with different response".into(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeacherDocumentKind {
    Lesson,
    Feedback,
}

/// Navigation metadata, not an estimate of learning or a mutable latest-content winner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherDocumentSummary {
    pub document_id: Uuid,
    pub kind: TeacherDocumentKind,
    pub request_id: String,
    pub title: String,
    pub lesson_id: Option<String>,
    pub attempt_id: Option<String>,
    pub practice_count: usize,
    pub receipt_count: usize,
    /// None means the original receive time is unknown, even when archive time is known.
    pub latest_received_at_ms: Option<i64>,
    pub first_archived_at_ms: i64,
    pub last_archived_at_ms: i64,
}

impl Library {
    /// All valid receipts share the exact response. Selection here is by stable event ID, not time.
    pub fn teacher_document(&self, document_id: Uuid) -> Option<&TeacherDocument> {
        self.events.values().find_map(|event| match &event.fact {
            Fact::TeacherDocumentRecorded(document) if document.document_id == document_id => {
                Some(document)
            }
            _ => None,
        })
    }

    pub fn teacher_document_receipts(&self, document_id: Uuid) -> Vec<(&Event, &TeacherDocument)> {
        self.events
            .values()
            .filter_map(|event| match &event.fact {
                Fact::TeacherDocumentRecorded(document) if document.document_id == document_id => {
                    Some((event, document))
                }
                _ => None,
            })
            .collect()
    }

    /// Known live receipt times order display first; recovered-only documents follow by archive
    /// time. Neither timestamp chooses content or implies the lesson was opened/completed.
    pub fn teacher_documents(&self) -> Vec<TeacherDocumentSummary> {
        let mut grouped = BTreeMap::<Uuid, Vec<(&Event, &TeacherDocument)>>::new();
        for event in self.events.values() {
            if let Fact::TeacherDocumentRecorded(document) = &event.fact {
                grouped
                    .entry(document.document_id)
                    .or_default()
                    .push((event, document));
            }
        }
        let mut summaries = Vec::new();
        for (id, receipts) in grouped {
            let (event, document) = receipts[0];
            let (kind, request_id, title, lesson_id, attempt_id) = match &document.response.request
            {
                TeacherRequest::TeachLesson(r) => (
                    TeacherDocumentKind::Lesson,
                    r.request_id.clone(),
                    r.title.clone(),
                    Some(r.lesson_id.clone()),
                    None,
                ),
                TeacherRequest::AnswerFeedback(r) => (
                    TeacherDocumentKind::Feedback,
                    r.request_id.clone(),
                    r.task.clone(),
                    None,
                    r.attempt_id.clone(),
                ),
            };
            let mut summary = TeacherDocumentSummary {
                document_id: id,
                kind,
                request_id,
                title,
                lesson_id,
                attempt_id,
                practice_count: document.response.content.practice.len(),
                receipt_count: receipts.len(),
                latest_received_at_ms: None,
                first_archived_at_ms: event.recorded_at_ms,
                last_archived_at_ms: event.recorded_at_ms,
            };
            for (event, document) in receipts {
                summary.first_archived_at_ms =
                    summary.first_archived_at_ms.min(event.recorded_at_ms);
                summary.last_archived_at_ms = summary.last_archived_at_ms.max(event.recorded_at_ms);
                if let TeacherDocumentReceipt::LiveReceived { received_at_ms } = document.receipt {
                    summary.latest_received_at_ms = Some(
                        summary
                            .latest_received_at_ms
                            .map_or(received_at_ms, |t| t.max(received_at_ms)),
                    );
                }
            }
            summaries.push(summary);
        }
        summaries.sort_by_key(|s| {
            (
                std::cmp::Reverse(s.latest_received_at_ms),
                std::cmp::Reverse(s.last_archived_at_ms),
                s.document_id,
            )
        });
        summaries
    }
}
