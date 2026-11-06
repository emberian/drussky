//! Executable teacher exercises retain the actual question, reference and answer.
use crate::{
    model::AppState,
    repository::{Result, now_ms},
};
use rufren_core::*;
use rufren_tutor::{ReviewStatus, TeacherRequest, TeacherResponse, TeacherSkill};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TeacherPracticeDraft {
    pub id: Uuid,
    pub source: String,
    pub index: usize,
    pub response: String,
    pub started_at_ms: i64,
    pub saved_attempt: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<crate::teacher_audio_practice::TeacherAudioDraft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<crate::teacher_revisit::TeacherRevisitDraft>,
}
impl TeacherPracticeDraft {
    pub(crate) fn fork_saved_answer(&mut self) {
        if self.saved_attempt.take().is_some() {
            self.id = Uuid::new_v4();
            self.started_at_ms = now_ms();
        }
    }
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
        Skill::Reading | Skill::Listening => TaskKind::MeaningRecall,
        Skill::Writing => TaskKind::Composition,
        Skill::Speaking => TaskKind::FormRecall,
    };
    let policy_ok = match a.skill {
        Skill::Reading | Skill::Writing => {
            a.policy == "teacher-guided-text-v1" && a.cues == [Cue::ReferenceMaterial]
        }
        Skill::Listening | Skill::Speaking => {
            a.policy == crate::teacher_audio_practice::TEACHER_AUDIO_POLICY
                && crate::teacher_audio_practice::valid_submitted_audio(library, attempt, c)
        }
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
        || !policy_ok
        || a.self_rating != SelfRating::NotReported
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
    /// Retry the proven original task as supported guided work, without replacing
    /// either the submitted response or the feedback currently being read.
    pub(crate) fn start_teacher_feedback_repair(
        &mut self,
        feedback: &TeacherResponse,
    ) -> Result<(TeacherResponse, usize, Uuid)> {
        self.refresh()?;
        feedback.request.validate().map_err(|e| e.to_string())?;
        feedback
            .content
            .validate_for(&feedback.request)
            .map_err(|e| e.to_string())?;
        let origin = self.teacher_feedback_task_origin(feedback)
            .ok_or("This feedback cannot be linked to an exact saved Teacher question. Its original response is kept.")?;
        let answer = self
            .library
            .teacher_document(origin.key.document_id)
            .ok_or("Sync the original Teacher question before trying it again.")?
            .response
            .clone();
        let index = origin.key.practice_index;
        // Read the durable slot rather than a component snapshot: an unfinished
        // response made elsewhere must be resumed, never replaced by a blank one.
        let saved = self.teacher_practice_draft(&answer, index)?;
        if saved.saved_attempt.is_none() {
            return Ok((answer, index, saved.id));
        }
        let next = TeacherPracticeDraft {
            id: Uuid::new_v4(),
            source: saved.source,
            index,
            response: String::new(),
            started_at_ms: now_ms(),
            saved_attempt: None,
            audio: None,
            review: None,
        };
        let target = PracticeBatchKey::Teacher {
            document_id: origin.key.document_id,
            practice_index: index,
        };
        let decision = self.prepare_review_start(
            next.id,
            target,
            next.started_at_ms,
            next.started_at_ms,
            next.started_at_ms,
            "teacher-guided-v1",
            "You chose a supported fresh response to this exact Teacher question after feedback.",
            self.library.events.keys().copied().collect(),
        )?;
        // The existing durable pending-start recovery covers interruption between
        // these writes. No editable form is exposed until the start is recorded.
        self.save_teacher_practice_draft(&next)?;
        self.record_review_start(&decision)?;
        Ok((answer, index, next.id))
    }

    pub(crate) fn teacher_practice_draft(
        &mut self,
        answer: &TeacherResponse,
        index: usize,
    ) -> Result<TeacherPracticeDraft> {
        let source = practice_source(answer);
        let key = format!("teacher_practice:{source}:{index}");
        let saved: Option<TeacherPracticeDraft> = self.repo.get(&key)?;
        if let Some(saved) = saved {
            if saved.source != source
                || saved.index != index
                || saved.id.is_nil()
                || saved.review.is_some()
            {
                return Err("This saved draft belongs to a different teacher exercise.".into());
            }
            self.recover_pending_review_start(
                saved.id,
                PracticeBatchKey::Teacher {
                    document_id: teacher_document_id(answer),
                    practice_index: index,
                },
            )?;
            return Ok(saved);
        }
        let draft = TeacherPracticeDraft {
            id: Uuid::new_v4(),
            source,
            index,
            response: String::new(),
            started_at_ms: now_ms(),
            saved_attempt: None,
            audio: None,
            review: None,
        };
        // A durable intent pins this new start before exposing any response controls.
        let target = PracticeBatchKey::Teacher {
            document_id: teacher_document_id(answer),
            practice_index: index,
        };
        let decision = self.prepare_review_start(
            draft.id,
            target,
            draft.started_at_ms,
            draft.started_at_ms,
            draft.started_at_ms,
            "teacher-guided-v1",
            "You chose a new Teacher exercise. Its initial cadence is pinned before practice.",
            self.library.events.keys().copied().collect(),
        )?;
        self.save_teacher_practice_draft(&draft)?;
        self.record_review_start(&decision)?;
        Ok(draft)
    }

    /// Fork a completed guided answer before accepting a new response or support action.
    pub(crate) fn fork_teacher_answer_for_edit(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<()> {
        if draft.saved_attempt.is_none() {
            return Ok(());
        }
        let stored = self.teacher_practice_draft(answer, draft.index)?;
        if stored.id != draft.id && stored.saved_attempt.is_none() {
            return Err("This task already has a newer unfinished response. Reopen it to continue; its draft was kept.".into());
        }
        let mut next = draft.clone();
        next.fork_saved_answer();
        let target = PracticeBatchKey::Teacher {
            document_id: teacher_document_id(answer),
            practice_index: next.index,
        };
        let decision = self.prepare_review_start(
            next.id,
            target,
            next.started_at_ms,
            next.started_at_ms,
            next.started_at_ms,
            "teacher-guided-v1",
            "You chose to try this Teacher exercise again with a fresh response.",
            self.library.events.keys().copied().collect(),
        )?;
        self.save_teacher_practice_draft(&next)?;
        self.record_review_start(&decision)?;
        *draft = next;
        Ok(())
    }

    pub(crate) fn save_teacher_practice_draft(&self, draft: &TeacherPracticeDraft) -> Result<()> {
        if draft.response.len() > 8192 || draft.id.is_nil() {
            return Err("Keep one answer under 8 KB; your earlier draft is still saved.".into());
        }
        if let Some(audio) = &draft.audio {
            audio.validate()?;
            if draft.review.is_none() && !audio.cues.contains(&Cue::ReferenceMaterial) {
                return Err("This guided oral draft is missing its original reference-support cue. Its earlier draft is kept.".into());
            }
        }
        if draft.review.is_some() {
            let merged = self.merged_teacher_revisit_draft(draft)?;
            return self.repo.set(
                &crate::teacher_revisit::teacher_revisit_storage_key(&merged)?,
                &merged,
            );
        }
        self.repo.set(
            &format!("teacher_practice:{}:{}", draft.source, draft.index),
            draft,
        )
    }

    pub(crate) fn teacher_practice_edit_response(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
        text: String,
    ) -> Result<()> {
        answer.request.validate().map_err(|e| e.to_string())?;
        answer
            .content
            .validate_for(&answer.request)
            .map_err(|e| e.to_string())?;
        if draft.source != practice_source(answer) || draft.id.is_nil() || text.len() > 8192 {
            return Err(
                "This response does not match the saved teacher draft or its text budget.".into(),
            );
        }
        let exercise = answer
            .content
            .practice
            .get(draft.index)
            .ok_or("This teacher exercise is unavailable.")?;
        if exercise.skill == Some(TeacherSkill::Speaking) {
            return Err("Choose a speaking recording; typed labels are not spoken words.".into());
        }
        if draft.review.is_some() {
            *draft = self.merged_teacher_revisit_draft(draft)?;
            if draft.saved_attempt.is_some() {
                return Err("This revisit is already saved. Start another scheduled revisit for a fresh response.".into());
            }
        }
        if draft.response == text {
            return Ok(());
        }
        let mut next = draft.clone();
        self.fork_teacher_answer_for_edit(answer, &mut next)?;
        next.response = text;
        self.save_teacher_practice_draft(&next)?;
        *draft = if next.review.is_some() {
            self.merged_teacher_revisit_draft(&next)?
        } else {
            next
        };
        Ok(())
    }
    pub(crate) fn teacher_practice_exercise_id(
        &self,
        answer: &TeacherResponse,
        index: usize,
        source: Uuid,
        passage: Uuid,
        context_id: Uuid,
        attempt_id: Uuid,
    ) -> String {
        let legacy = format!("teacher:{passage}");
        if self.library.events.get(&context_id).is_some_and(|e|matches!(&e.fact,Fact::AttemptContextRecorded(old) if old.attempt==attempt_id&&old.exercise_id.as_deref()==Some(legacy.as_str()))){return legacy;}
        if let TeacherRequest::TeachLesson(lesson) = &answer.request {
            if self.content.lesson(&lesson.lesson_id).is_some() {
                let origin = TeacherPracticeOrigin {
                    lesson_id: lesson.lesson_id.clone(),
                    source,
                    index,
                };
                let value = origin.encoded();
                if TeacherPracticeOrigin::parse(&value).is_some() {
                    return value;
                }
            }
        }
        legacy
    }
    pub(crate) fn submit_teacher_practice(
        &mut self,
        answer: &TeacherResponse,
        draft: &mut TeacherPracticeDraft,
    ) -> Result<Uuid> {
        if answer.content.practice.get(draft.index).is_some_and(|e| {
            matches!(
                e.skill,
                Some(TeacherSkill::Listening | TeacherSkill::Speaking)
            )
        }) {
            return self.submit_teacher_audio_practice(answer, draft);
        }
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
        if draft.review.is_some() {
            *draft = self.merged_teacher_revisit_draft(draft)?;
            self.validate_teacher_revisit(answer, draft)?;
            self.qualify_teacher_revisit_support(answer, draft)?;
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
            cues: draft.review.as_ref().map_or_else(
                || vec![Cue::ReferenceMaterial],
                |review| review.cues.clone(),
            ),
            self_rating: draft
                .review
                .as_ref()
                .map_or(SelfRating::NotReported, |review| review.self_rating),
            policy: if draft.review.is_some() {
                crate::teacher_revisit::TEACHER_REVISIT_POLICY
            } else {
                "teacher-guided-text-v1"
            }
            .into(),
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
            exercise_id: Some(if draft.review.is_some() {
                crate::teacher_revisit::teacher_revisit_exercise_id(draft)?
            } else {
                exercise_id
            }),
            prompt: Some(prompt),
            reference_answer: Some(if draft.review.is_some() {
                crate::teacher_revisit::teacher_revisit_context_reference(&reference, draft)?
            } else {
                reference
            }),
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

pub(crate) fn teacher_reference_fact(
    answer: &TeacherResponse,
    index: usize,
) -> Result<(Uuid, Uuid, Fact)> {
    rufren_core::teacher_practice_reference_fact(answer, index).map_err(|error| error.to_string())
}
