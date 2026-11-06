//! Full immutable Teacher documents. Local selection/cursors never become practice evidence.
use crate::{
    model::AppState,
    repository::{CachedTeacherMetadata, Result, now_ms},
    teacher_context::PreparedTeacherContext,
};
use rufren_core::*;
use rufren_tutor::{TeacherRequest, TeacherResponse, TeacherSkill, TextEvidence};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct TeacherNotebookEntry {
    pub document_id: Uuid,
    pub response: TeacherResponse,
    /// Unknown for recovered caches; archive time is never substituted for receipt time.
    pub received_at_ms: Option<i64>,
    pub archived_at_ms: i64,
    pub recovered: bool,
    pub context_source_fact_ids: Option<Vec<Uuid>>,
}

fn context_sources(
    response: &TeacherResponse,
    prepared: Option<&PreparedTeacherContext>,
) -> Option<Vec<Uuid>> {
    let TeacherRequest::TeachLesson(request) = &response.request else {
        return None;
    };
    let prepared = prepared?;
    (prepared.request_id == request.request_id
        && request.learner_context.as_deref() == Some(prepared.summary.provider_context())
        && prepared.summary.lesson_id == request.lesson_id
        && prepared.source_fact_ids == prepared.summary.source_fact_ids())
    .then(|| prepared.source_fact_ids.clone())
}

fn recovered_document(cache: CachedTeacherMetadata) -> Result<TeacherDocument> {
    let response: TeacherResponse = serde_json::from_value(cache.response).map_err(
        |_| "A saved teacher response could not be recovered; its original cache was kept",
    )?;
    // A missing/old auxiliary context snapshot does not invalidate the exact response.
    // Only the complete matching snapshot can supply source IDs; never infer them now.
    let prepared = cache
        .prepared_context
        .and_then(|value| serde_json::from_value::<PreparedTeacherContext>(value).ok());
    Ok(TeacherDocument {
        format_version: TEACHER_DOCUMENT_FORMAT_VERSION,
        document_id: teacher_document_id(&response),
        context_source_fact_ids: context_sources(&response, prepared.as_ref()),
        response,
        receipt: TeacherDocumentReceipt::RecoveredLocalCache,
    })
}

impl AppState {
    pub(crate) fn teacher_notebook(&self) -> Vec<TeacherNotebookEntry> {
        self.library
            .teacher_documents()
            .into_iter()
            .filter_map(|summary| {
                let document = self.library.teacher_document(summary.document_id)?;
                let mut sources = None::<Vec<Uuid>>;
                for (_, receipt) in self.library.teacher_document_receipts(summary.document_id) {
                    if let Some(ids) = &receipt.context_source_fact_ids {
                        let known = sources.get_or_insert_with(Vec::new);
                        known.extend(ids);
                        known.sort();
                        known.dedup();
                    }
                }
                Some(TeacherNotebookEntry {
                    document_id: summary.document_id,
                    response: document.response.clone(),
                    received_at_ms: summary.latest_received_at_ms,
                    archived_at_ms: summary.first_archived_at_ms,
                    recovered: summary.latest_received_at_ms.is_none(),
                    context_source_fact_ids: sources,
                })
            })
            .collect()
    }

    /// Return only the exact generated lesson/exercise recorded in this feedback's task.
    /// Old authored/partial history remains navigable without guessing a lesson by recency.
    pub(crate) fn teacher_lesson_for_feedback(
        &self,
        response: &TeacherResponse,
    ) -> Option<(Uuid, usize)> {
        let TeacherRequest::AnswerFeedback(request) = &response.request else {
            return None;
        };
        let attempt_id = Uuid::parse_str(request.attempt_id.as_deref()?).ok()?;
        let context_id = Uuid::parse_str(request.context_id.as_deref()?).ok()?;
        let attempt = self.library.attempt(attempt_id)?;
        let Fact::AttemptContextRecorded(context) = &self.library.events.get(&context_id)?.fact
        else {
            return None;
        };
        let origin = crate::teacher_practice::saved_teacher_practice_origin(
            &self.library,
            attempt_id,
            context_id,
        )?;
        let document = self.library.teacher_document(origin.source)?;
        let TeacherRequest::TeachLesson(lesson) = &document.response.request else {
            return None;
        };
        let (_, revision_id, reference) =
            crate::teacher_practice::teacher_reference_fact(&document.response, origin.index)
                .ok()?;
        let skill = match attempt.skill {
            Skill::Reading => TeacherSkill::Reading,
            Skill::Writing => TeacherSkill::Writing,
            Skill::Listening => TeacherSkill::Listening,
            Skill::Speaking => TeacherSkill::Speaking,
        };
        let words = if let Some(confirmation) = &request.confirmed_transcript_id {
            if request.evidence != TextEvidence::Transcript {
                return None;
            }
            self.confirmed_words(Uuid::parse_str(confirmation).ok()?, attempt_id)
                .ok()?
        } else {
            if request.evidence != TextEvidence::WrittenText || attempt.skill == Skill::Speaking {
                return None;
            }
            attempt.response.clone()
        };
        let task: TaskKind =
            serde_json::from_value(serde_json::Value::String(request.task.clone())).ok()?;
        let cues: Vec<_> = attempt
            .cues
            .iter()
            .map(|cue| serde_json::to_value(cue).ok()?.as_str().map(str::to_owned))
            .collect::<Option<_>>()?;
        (lesson.lesson_id == origin.lesson_id
            && self.library.events.get(&revision_id)?.fact == reference
            && attempt.revision == revision_id
            && request.revision_id == revision_id.to_string()
            && request.exercise_id == context.exercise_id
            && Some(request.prompt.as_str()) == context.prompt.as_deref()
            && request.reference == context.reference_answer
            && request.skill == skill
            && request.response == words
            && request.cues == cues
            && task == context.task)
            .then_some((origin.source, origin.index))
    }

    pub(crate) fn select_teacher_document(&self, id: Uuid) -> Result<()> {
        if self.library.teacher_document(id).is_none() {
            return Err("That teacher notebook entry is unavailable. Sync and try again.".into());
        }
        self.repo.set("teacher_notebook_selection", &id)
    }

    pub(crate) fn selected_teacher_document(&self) -> Result<Option<TeacherResponse>> {
        let selected: Option<Uuid> = self.repo.get("teacher_notebook_selection")?;
        Ok(selected.and_then(|id| {
            self.library
                .teacher_document(id)
                .map(|doc| doc.response.clone())
        }))
    }

    pub(crate) fn clear_teacher_document_selection(&self) -> Result<()> {
        self.repo.set("teacher_notebook_selection", &None::<Uuid>)
    }

    pub(crate) fn teacher_lesson_cursor(&self, id: Uuid) -> Result<usize> {
        let step = self
            .repo
            .get(&format!("teacher_notebook_cursor:{id}"))?
            .unwrap_or(0);
        self.validate_teacher_lesson_cursor(id, step)?;
        Ok(step)
    }

    fn validate_teacher_lesson_cursor(&self, id: Uuid, step: usize) -> Result<()> {
        let document = self
            .library
            .teacher_document(id)
            .ok_or("That teacher lesson is unavailable. Sync and try again.")?;
        let count = document
            .response
            .content
            .lesson_flow
            .as_ref()
            .map_or(1 + document.response.content.practice.len(), |flow| {
                flow.steps.len()
            });
        if !matches!(document.response.request, TeacherRequest::TeachLesson(_)) || step > count {
            return Err(
                "That step is outside this saved teacher lesson. Its earlier place was kept."
                    .into(),
            );
        }
        Ok(())
    }

    pub(crate) fn set_teacher_lesson_cursor(&self, id: Uuid, step: usize) -> Result<()> {
        self.validate_teacher_lesson_cursor(id, step)?;
        self.repo
            .set(&format!("teacher_notebook_cursor:{id}"), &step)
    }

    /// Preserve every surviving legacy response before overwriting any latest-view slot.
    /// Valid candidates recover independently; skipped originals are never deleted.
    pub(crate) fn recover_cached_teacher_responses(&mut self) -> Result<usize> {
        self.refresh()?;
        let candidates = self.repo.cached_teacher_metadata()?;
        let mut projected = self.library.clone();
        let mut events = Vec::new();
        let mut skipped = 0usize;
        for cache in candidates {
            let document = match recovered_document(cache) {
                Ok(document) => document,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let covered = projected
                .teacher_document_receipts(document.document_id)
                .iter()
                .any(|(_, old)| {
                    old.response == document.response
                        && (document.context_source_fact_ids.is_none()
                            || old.context_source_fact_ids == document.context_source_fact_ids)
                });
            if covered {
                continue;
            }
            let event = match Event::new(
                self.device,
                now_ms(),
                Fact::TeacherDocumentRecorded(document),
            ) {
                Ok(event) => event,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            if projected.merge(&[event.clone()]).is_err() {
                skipped += 1;
                continue;
            }
            events.push(event);
        }
        self.repo.append(&events)?;
        self.refresh()?;
        if skipped > 0 {
            return Err(format!(
                "Recovered {} teacher responses; {skipped} older cached responses need recovery review. Their original caches were kept.",
                events.len()
            ));
        }
        Ok(events.len())
    }

    /// Called only after the existing exact-response acceptance checks have succeeded.
    pub(crate) fn archive_accepted_teacher_response(
        &mut self,
        response: &TeacherResponse,
    ) -> Result<bool> {
        let legacy_cache_writable = match self.recover_cached_teacher_responses() {
            Ok(_) => true,
            Err(problem) => {
                self.notice = format!(
                    "{problem} New responses are saved in your notebook; older cache slots are preserved."
                );
                false
            }
        };
        let prepared = match &response.request {
            TeacherRequest::TeachLesson(request) => self
                .prepared_lesson_context(&request.request_id)
                .ok()
                .flatten(),
            _ => None,
        };
        let document_id = teacher_document_id(response);
        let sources = context_sources(response, prepared.as_ref());
        if self
            .library
            .teacher_document_receipts(document_id)
            .iter()
            .any(|(_, old)| {
                old.response == *response
                    && matches!(old.receipt, TeacherDocumentReceipt::LiveReceived { .. })
                    && (sources.is_none() || old.context_source_fact_ids == sources)
            })
        {
            return Ok(legacy_cache_writable);
        }
        let event = Event::new(
            self.device,
            now_ms(),
            Fact::TeacherDocumentRecorded(TeacherDocument {
                format_version: TEACHER_DOCUMENT_FORMAT_VERSION,
                document_id,
                response: response.clone(),
                receipt: TeacherDocumentReceipt::LiveReceived {
                    received_at_ms: now_ms(),
                },
                context_source_fact_ids: sources,
            }),
        )
        .map_err(|error| error.to_string())?;
        self.repo.append(&[event])?;
        self.refresh()?;
        Ok(legacy_cache_writable)
    }
}
