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

/// Qualified generated-question identity; it does not label an authored pack or tested feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeacherPracticeOrigin {
    pub lesson_id: String,
    pub source: Uuid,
    pub index: usize,
}
impl TeacherPracticeOrigin {
    fn encoded(&self) -> String {
        format!(
            "teacher:v2:{}:{}:{}",
            self.lesson_id, self.source, self.index
        )
    }
    fn parse(value: &str) -> Option<Self> {
        if value.len() > 128 {
            return None;
        }
        let mut parts = value.strip_prefix("teacher:v2:")?.rsplitn(3, ':');
        let index_text = parts.next()?;
        let index: usize = index_text.parse().ok()?;
        let source_text = parts.next()?;
        let source = Uuid::parse_str(source_text).ok()?;
        let lesson_id = parts.next()?;
        if index >= 4
            || index.to_string() != index_text
            || source.is_nil()
            || source.to_string() != source_text
            || lesson_id.is_empty()
            || !lesson_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
        {
            return None;
        }
        Some(Self {
            lesson_id: lesson_id.into(),
            source,
            index,
        })
    }
}
/// Resolve only the explicit identity against actual immutable generated facts. A legacy
/// `teacher:<passage>` question is never attributed from its title, body or latest response.
pub(crate) fn saved_teacher_practice_origin(
    library: &Library,
    attempt: Uuid,
    context: Uuid,
) -> Option<TeacherPracticeOrigin> {
    let a = library.attempt(attempt)?;
    let Fact::AttemptContextRecorded(c) = &library.events.get(&context)?.fact else {
        return None;
    };
    let origin = TeacherPracticeOrigin::parse(c.exercise_id.as_deref()?)?;
    let passage = Uuid::new_v5(&origin.source, origin.index.to_string().as_bytes());
    let revision = Uuid::new_v5(&passage, b"reference-v1");
    let reference = library.events.get(&revision)?;
    let Fact::PassageRevised(r) = &reference.fact else {
        return None;
    };
    let task = match a.skill {
        Skill::Reading => TaskKind::MeaningRecall,
        Skill::Writing => TaskKind::Composition,
        _ => return None,
    };
    if c.attempt != attempt
        || a.revision != revision
        || r.passage.id != passage
        || reference.device != origin.source
        || reference.version != 2
        || reference.recorded_at_ms != 0
        || !r.parents.is_empty()
        || attempt != Uuid::new_v5(&a.session, b"attempt")
        || context != Uuid::new_v5(&a.session, b"context")
        || a.policy != "teacher-guided-text-v1"
        || a.self_rating != SelfRating::NotReported
        || a.cues != [Cue::ReferenceMaterial]
        || c.task != task
        || c.content_pack.is_some()
        || c.content_revision.is_some()
        || !c.target_features.is_empty()
        || c.prompt.is_none()
        || c.reference_answer.is_none()
    {
        return None;
    }
    let fields: Vec<_> = r.passage.source.split(" · ").collect();
    if fields.len() != 5
        || fields[0] != "AI teacher exercise"
        || fields[4] != origin.index.to_string()
        || Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!(
                "rufren:teacher-practice:{}:{}:{}",
                fields[1], fields[2], fields[3]
            )
            .as_bytes(),
        ) != origin.source
    {
        return None;
    }
    Some(origin)
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
        let legacy_exercise = format!("teacher:{passage_id}");
        let qualified = match &answer.request {
            TeacherRequest::TeachLesson(lesson)
                if self.content.lesson(&lesson.lesson_id).is_some() =>
            {
                let origin = TeacherPracticeOrigin {
                    lesson_id: lesson.lesson_id.clone(),
                    source: source_id,
                    index: draft.index,
                };
                let encoded = origin.encoded();
                TeacherPracticeOrigin::parse(&encoded).map(|_| encoded)
            }
            _ => None,
        };
        let exercise_id = match self
            .library
            .events
            .get(&context_id)
            .map(|event| &event.fact)
        {
            Some(Fact::AttemptContextRecorded(old))
                if old.attempt == attempt_id
                    && old.exercise_id.as_deref() == Some(legacy_exercise.as_str()) =>
            {
                legacy_exercise
            }
            _ => qualified.unwrap_or(legacy_exercise),
        };
        let context = Fact::AttemptContextRecorded(AttemptContext {
            attempt: attempt_id,
            content_pack: None,
            content_revision: None,
            exercise_id: Some(exercise_id),
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
