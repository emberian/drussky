//! Teacher context and durable model feedback, independent of the UI.
use crate::{
    model::{AppState, Editor},
    repository::{now_ms, Result},
};
use rufren_core::*;
use rufren_tutor::*;
use uuid::Uuid;

impl AppState {
    pub(crate) fn lesson_teaching_request(&self) -> Result<TeacherRequest> {
        let lesson = self
            .lesson()
            .or_else(|| self.content.lessons.first())
            .ok_or("Choose a lesson first.")?;
        let mut material = String::new();
        for phrase in self.content.lesson_phrases(lesson) {
            material.push_str(&format!("{} — {}\n", phrase.stressed, phrase.translation));
        }
        for explanation in &lesson.explanations {
            material.push_str(&format!("{}: {}\n", explanation.title, explanation.text));
        }
        let summary = self.lesson_practice_summary()?;
        let learner_context = summary.provider_context().to_owned();
        let request = TeacherRequest::TeachLesson(LessonTeachingRequest {
            request_id: Uuid::new_v4().to_string(),
            lesson_id: lesson.id.clone(),
            title: lesson.title.clone(),
            objective: lesson.goal.clone(),
            material,
            learner_context: Some(learner_context),
        });
        request.validate().map_err(|e| e.to_string())?;
        if let TeacherRequest::TeachLesson(lesson_request) = &request {
            self.keep_prepared_lesson_context(lesson_request, &summary)?;
        }
        Ok(request)
    }

    pub(crate) fn answer_feedback_request(&self, id: Uuid) -> Result<TeacherRequest> {
        self.feedback_request(id, None)
    }

    pub(crate) fn confirmed_transcript_feedback_request(
        &self,
        confirmation: Uuid,
    ) -> Result<TeacherRequest> {
        let c = self
            .library
            .transcript_confirmation(confirmation)
            .ok_or("Those confirmed words are unavailable.")?;
        let t = self
            .library
            .speech_transcription(c.transcription)
            .ok_or("The original transcript is unavailable.")?;
        self.feedback_request(t.attempt, Some(confirmation))
    }

    fn feedback_request(&self, id: Uuid, confirmation: Option<Uuid>) -> Result<TeacherRequest> {
        let attempt = self
            .library
            .attempt(id)
            .ok_or("That saved response is unavailable.")?;
        if attempt.skill == Skill::Speaking && confirmation.is_none() {
            return Err("This response is a recording. Text feedback needs a transcript; a recording label is not your spoken answer.".into());
        }
        if attempt.skill != Skill::Speaking && confirmation.is_some() {
            return Err("A transcript can only describe a speaking response.".into());
        }
        let response = if let Some(confirmation) = confirmation {
            self.confirmed_words(confirmation, id)?
        } else {
            attempt.response.clone()
        };
        let revision = self
            .library
            .revision(attempt.revision)
            .ok_or("Sync the original passage before asking about this response.")?;
        let contexts = self.library.contexts(id);
        if contexts.len() > 1 {
            return Err("This response has competing task descriptions. Preserve them for review before requesting a critique.".into());
        }
        let context = contexts.first().copied();
        let task = context.map(|(_, c)| c.task).unwrap_or(match attempt.skill {
            Skill::Reading | Skill::Listening => TaskKind::MeaningRecall,
            _ => TaskKind::FormRecall,
        });
        let prompt = context.and_then(|(_,c)|c.prompt.clone()).or_else(|| {
            // Older authored responses lack their exact choice mapping. Do not substitute a
            // possibly revised bundled exercise when the original question is unavailable.
            if context.is_some_and(|(_,c)|c.exercise_id.is_some()) { return None; }
            Some(match attempt.skill {Skill::Writing|Skill::Speaking=>format!("Express this meaning in Russian: {}",revision.passage.translation),_=>format!("Explain the meaning of this Russian passage: {}",revision.passage.russian)})
        }).ok_or("This older exercise did not save its complete question. You can ask about the passage below, or try the exercise again.")?;
        let reference = context
            .and_then(|(_, c)| c.reference_answer.clone())
            .unwrap_or_else(|| {
                format!(
                    "Russian: {}\nMeaning: {}",
                    revision.passage.russian, revision.passage.translation
                )
            });
        let request = TeacherRequest::AnswerFeedback(AnswerFeedbackRequest {
            request_id: Uuid::new_v4().to_string(),
            revision_id: attempt.revision.to_string(),
            attempt_id: Some(id.to_string()),
            context_id: context.map(|(e, _)| e.id.to_string()),
            exercise_id: context.and_then(|(_, c)| c.exercise_id.clone()),
            confirmed_transcript_id: confirmation.map(|id| id.to_string()),
            skill: match attempt.skill {
                Skill::Reading => TeacherSkill::Reading,
                Skill::Writing => TeacherSkill::Writing,
                Skill::Listening => TeacherSkill::Listening,
                Skill::Speaking => TeacherSkill::Speaking,
            },
            task: serde_json::to_value(task)
                .map_err(|e| e.to_string())?
                .as_str()
                .ok_or("Invalid task")?
                .into(),
            prompt,
            reference: Some(reference),
            response,
            cues: attempt
                .cues
                .iter()
                .map(|c| {
                    serde_json::to_value(c)
                        .map(|s| s.as_str().unwrap_or("support").to_owned())
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>>>()?,
            evidence: if confirmation.is_some() {
                TextEvidence::Transcript
            } else {
                TextEvidence::WrittenText
            },
        });
        request.validate().map_err(|e| e.to_string())?;
        Ok(request)
    }

    pub(crate) fn keep_teacher_response(
        &mut self,
        expected: &TeacherRequest,
        response: &TeacherResponse,
    ) -> Result<()> {
        expected.validate().map_err(|e| e.to_string())?;
        if response.request != *expected {
            return Err(
                "The teacher returned feedback for a different question. Your response is kept."
                    .into(),
            );
        }
        response
            .content
            .validate_for(expected)
            .map_err(|e| e.to_string())?;
        if response.provenance.review_status != ReviewStatus::AiGeneratedTeaching
            || response.provenance.response_id.is_empty()
        {
            return Err("The teacher response is missing its source.".into());
        }
        if let (TeacherRequest::AnswerFeedback(request), Some(feedback)) =
            (expected, &response.content.feedback)
        {
            let id = request
                .attempt_id
                .as_deref()
                .ok_or_else(|| "The response has not been saved yet.".to_string())
                .and_then(|id| Uuid::parse_str(id).map_err(|e| e.to_string()))?;
            self.refresh()?;
            let attempt = self
                .library
                .attempt(id)
                .ok_or("The saved response is unavailable.")?;
            let task: TaskKind =
                serde_json::from_value(serde_json::Value::String(request.task.clone()))
                    .map_err(|e| e.to_string())?;
            let context_id = request
                .context_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|e| e.to_string())?;
            let context = context_id
                .map(|id| match self.library.events.get(&id).map(|e| &e.fact) {
                    Some(Fact::AttemptContextRecorded(c))
                        if c.attempt
                            == request
                                .attempt_id
                                .as_deref()
                                .and_then(|id| Uuid::parse_str(id).ok())
                                .unwrap_or_default() =>
                    {
                        Ok(c)
                    }
                    _ => {
                        Err("The teacher question refers to a different task context.".to_string())
                    }
                })
                .transpose()?;
            let revision = self
                .library
                .revision(attempt.revision)
                .ok_or("The original reference is unavailable.")?;
            let canonical_task = context.map(|c| c.task).unwrap_or(match attempt.skill {
                Skill::Reading | Skill::Listening => TaskKind::MeaningRecall,
                _ => TaskKind::FormRecall,
            });
            let canonical_prompt = context.and_then(|c| c.prompt.clone()).or_else(|| {
                if context.is_some_and(|c| c.exercise_id.is_some()) {
                    return None;
                }
                Some(match attempt.skill {
                    Skill::Writing | Skill::Speaking => format!(
                        "Express this meaning in Russian: {}",
                        revision.passage.translation
                    ),
                    _ => format!(
                        "Explain the meaning of this Russian passage: {}",
                        revision.passage.russian
                    ),
                })
            });
            let canonical_reference = context
                .and_then(|c| c.reference_answer.clone())
                .unwrap_or_else(|| {
                    format!(
                        "Russian: {}\nMeaning: {}",
                        revision.passage.russian, revision.passage.translation
                    )
                });
            let canonical_skill = match attempt.skill {
                Skill::Reading => TeacherSkill::Reading,
                Skill::Writing => TeacherSkill::Writing,
                Skill::Listening => TeacherSkill::Listening,
                Skill::Speaking => TeacherSkill::Speaking,
            };
            let cues = attempt
                .cues
                .iter()
                .map(|c| {
                    serde_json::to_value(c)
                        .map(|v| v.as_str().unwrap_or("support").to_owned())
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>>>()?;
            let confirmed_transcript = request
                .confirmed_transcript_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|e| e.to_string())?;
            let canonical_response = if let Some(confirmation) = confirmed_transcript {
                self.confirmed_words(confirmation, id)?
            } else {
                attempt.response.clone()
            };
            let canonical_evidence = if confirmed_transcript.is_some() {
                TextEvidence::Transcript
            } else {
                TextEvidence::WrittenText
            };
            if request.skill != canonical_skill
                || task != canonical_task
                || canonical_prompt.as_deref() != Some(request.prompt.as_str())
                || request.reference.as_deref() != Some(canonical_reference.as_str())
                || request.exercise_id != context.and_then(|c| c.exercise_id.clone())
                || request.cues != cues
                || request.evidence != canonical_evidence
                || request.response != canonical_response
                || (attempt.skill == Skill::Speaking) != confirmed_transcript.is_some()
            {
                return Err("The teacher critique does not match the saved question, skill, reference and support. The original response is kept.".into());
            }
            let dimension = match task {
                TaskKind::Spelling => JudgmentDimension::Spelling,
                _ if matches!(attempt.skill, Skill::Reading | Skill::Listening) => {
                    JudgmentDimension::Comprehension
                }
                _ => JudgmentDimension::Grammar,
            };
            let assessment = AiAssessment {
                request_id: request.request_id.clone(),
                attempt: id,
                revision: Uuid::parse_str(&request.revision_id).map_err(|e| e.to_string())?,
                skill: attempt.skill,
                task,
                response: request.response.clone(),
                context: request
                    .context_id
                    .as_deref()
                    .map(Uuid::parse_str)
                    .transpose()
                    .map_err(|e| e.to_string())?,
                evidence: match request.evidence {
                    TextEvidence::WrittenText => AiTextEvidence::WrittenText,
                    TextEvidence::Transcript => AiTextEvidence::Transcript,
                },
                confirmed_transcript,
                dimension,
                verdict: match feedback.verdict {
                    AiVerdict::Correct => Verdict::Correct,
                    AiVerdict::PartlyCorrect => Verdict::PartlyCorrect,
                    AiVerdict::Incorrect => Verdict::Incorrect,
                    AiVerdict::Uncertain => Verdict::Uncertain,
                },
                suggested_response: feedback.suggested_response.clone(),
                corrections: feedback
                    .corrections
                    .iter()
                    .map(|c| AiCorrection {
                        original: c.original.clone(),
                        replacement: c.replacement.clone(),
                        reason: c.reason.clone(),
                    })
                    .collect(),
                explanation: response.content.explanation.clone(),
                next_step: response.content.next_step.clone(),
                provenance: AiProvenance {
                    provider: response.provenance.provider.clone(),
                    model: response.provenance.model.clone(),
                    response_id: response.provenance.response_id.clone(),
                    prompt_version: response.provenance.prompt_version.clone(),
                },
                supersedes: Default::default(),
            };
            let mut event = Event::new(
                self.device,
                now_ms(),
                Fact::AiAssessmentRecorded(assessment),
            )
            .map_err(|e| e.to_string())?;
            event.id = Uuid::new_v5(
                &id,
                format!(
                    "teacher:{}:{}",
                    request.request_id, response.provenance.response_id
                )
                .as_bytes(),
            );
            // Retries after an interrupted metadata write reuse the exact durable fact.
            if let Some(saved) = self.library.events.get(&event.id) {
                if saved.fact != event.fact {
                    return Err("A repeated teacher response changed its contents. The earlier feedback is kept.".into());
                }
                event = saved.clone();
            }
            self.repo.append(&[event])?;
            self.refresh()?;
        }
        // Feedback replaces the current view, but the lesson's remaining exercises stay
        // available. Only fully validated lesson responses may replace this slot.
        if matches!(expected, TeacherRequest::TeachLesson(_)) {
            self.repo.set("teacher_lesson_response", response)?;
        }
        self.repo.set("teacher_response", response)?;
        Ok(())
    }

    pub(crate) fn keep_teacher_example(&mut self, example: TutorExample) -> Result<()> {
        self.open_passage_draft(Editor {
            title: example.english.clone(),
            russian: example.russian,
            translation: example.english,
            source: format!("AI teacher example · {}", example.note),
            ..Default::default()
        })
    }
}
