//! Executable teacher exercises retain the actual question, reference and answer.
use crate::{
    model::AppState,
    repository::{now_ms, Result},
};
use rufren_core::*;
use rufren_tutor::{ReviewStatus, TeacherRequest, TeacherResponse, TeacherSkill};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TeacherPracticeDraft {
    pub id: Uuid,
    pub source: String,
    pub index: usize,
    pub response: String,
    pub started_at_ms: i64,
    pub saved_attempt: Option<Uuid>,
}

pub(crate) fn practice_source(answer: &TeacherResponse) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "rufren:teacher-practice:{}:{}:{}",
            answer.provenance.provider, answer.provenance.model, answer.provenance.response_id,
        )
        .as_bytes(),
    )
    .to_string()
}

impl AppState {
    pub(crate) fn teacher_practice_draft(
        &self,
        answer: &TeacherResponse,
        index: usize,
    ) -> Result<TeacherPracticeDraft> {
        let source = practice_source(answer);
        let key = format!("teacher_practice:{source}:{index}");
        let saved: Option<TeacherPracticeDraft> = self.repo.get(&key)?;
        if let Some(saved) = saved {
            if saved.source != source || saved.index != index || saved.id.is_nil() {
                return Err("This saved draft belongs to a different teacher exercise.".into());
            }
            return Ok(saved);
        }
        Ok(TeacherPracticeDraft {
            id: Uuid::new_v4(),
            source,
            index,
            response: String::new(),
            started_at_ms: now_ms(),
            saved_attempt: None,
        })
    }

    pub(crate) fn save_teacher_practice_draft(&self, draft: &TeacherPracticeDraft) -> Result<()> {
        if draft.response.len() > 8192 || draft.id.is_nil() {
            return Err("Keep one answer under 8 KB; your earlier draft is still saved.".into());
        }
        self.repo.set(
            &format!("teacher_practice:{}:{}", draft.source, draft.index),
            draft,
        )
    }

    pub(crate) fn submit_teacher_practice(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<Uuid> {
        answer.request.validate().map_err(|e| e.to_string())?;
        answer
            .content
            .validate_for(&answer.request)
            .map_err(|e| e.to_string())?;
        if answer.provenance.review_status != ReviewStatus::AiGeneratedTeaching
            || answer.provenance.response_id.is_empty()
            || draft.source != practice_source(answer)
            || draft.response.trim().is_empty()
            || draft.response.len() > 8192
            || draft.id.is_nil()
        {
            return Err("Write a short response to this teacher exercise first.".into());
        }
        let exercise = answer
            .content
            .practice
            .get(draft.index)
            .ok_or("This teacher exercise is unavailable.")?;
        let russian = exercise
            .reference_russian
            .as_ref()
            .ok_or("Ask for this lesson again to get interactive practice.")?;
        let english = exercise
            .reference_english
            .as_ref()
            .ok_or("This exercise needs its reference meaning.")?;
        let skill = match exercise.skill {
            Some(TeacherSkill::Reading) => Skill::Reading,
            Some(TeacherSkill::Writing) => Skill::Writing,
            _ => return Err("This exercise does not support a written answer.".into()),
        };
        self.refresh()?;
        let source_id = Uuid::parse_str(&draft.source).map_err(|e| e.to_string())?;
        let passage_id = Uuid::new_v5(&source_id, draft.index.to_string().as_bytes());
        let revision_id = Uuid::new_v5(&passage_id, b"reference-v1");
        let attempt_id = Uuid::new_v5(&draft.id, b"attempt");
        let context_id = Uuid::new_v5(&draft.id, b"context");
        let title = match &answer.request {
            TeacherRequest::TeachLesson(lesson) => lesson.title.clone(),
            _ => "A teacher follow-up".into(),
        };
        let revision = Fact::PassageRevised(Revision {
            passage: Passage {
                id: passage_id,
                title: format!("{title} · practice {}", draft.index + 1),
                russian: russian.clone(),
                translation: english.clone(),
                source: format!(
                    "AI teacher exercise · {} · {} · {} · {}",
                    answer.provenance.provider,
                    answer.provenance.model,
                    answer.provenance.response_id,
                    draft.index
                ),
                coach_reviewed: false,
            },
            parents: Default::default(),
        });
        let attempt = Fact::AttemptRecorded(Attempt {
            session: draft.id,
            revision: revision_id,
            skill,
            response: draft.response.clone(),
            cues: vec![Cue::ReferenceMaterial],
            self_rating: SelfRating::NotReported,
            policy: "teacher-guided-text-v1".into(),
        });
        let prompt = if skill == Skill::Reading {
            format!("{}\n\nRussian text:\n{russian}", exercise.prompt)
        } else {
            exercise.prompt.clone()
        };
        let reference = format!(
            "Example answer: {}\nReference Russian: {russian}\nReference meaning: {english}\nTeaching note: {}",
            exercise
                .expected_response
                .as_deref()
                .unwrap_or("An open response; accept valid alternatives."),
            exercise.teaching_note
        );
        let context = Fact::AttemptContextRecorded(AttemptContext {
            attempt: attempt_id,
            content_pack: None,
            content_revision: None,
            exercise_id: Some(format!("teacher:{passage_id}")),
            prompt: Some(prompt),
            reference_answer: Some(reference),
            task: if skill == Skill::Reading {
                TaskKind::MeaningRecall
            } else {
                TaskKind::Composition
            },
            target_features: Default::default(),
            novel_context: false,
            delayed_probe: false,
            practice_duration_ms: None,
            effort: None,
            grammar_response: None,
            supersedes: Default::default(),
        });
        let now = now_ms();
        let mut events = Vec::new();
        for (id, fact) in [
            (revision_id, revision),
            (attempt_id, attempt),
            (context_id, context),
        ] {
            if let Some(old) = self.library.events.get(&id) {
                if old.fact != fact {
                    return Err("This saved response has different contents. Start a fresh answer; the earlier one is kept.".into());
                }
                continue;
            }
            let reference = id == revision_id;
            events.push(Event {
                version: if reference { 2 } else { EVENT_VERSION },
                id,
                device: if reference { source_id } else { self.device },
                recorded_at_ms: if reference { 0 } else { now },
                fact,
            });
        }
        self.repo.append(&events)?;
        self.refresh()?;
        draft.saved_attempt = Some(attempt_id);
        self.save_teacher_practice_draft(draft)?;
        Ok(attempt_id)
    }
}
