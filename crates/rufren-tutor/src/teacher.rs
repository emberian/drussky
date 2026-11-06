//! AI teaching and language-content feedback, bound to the exact submitted task.
use crate::{TutorError, TutorExample, TutorProvenance, TutorUsage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TeacherSkill {
    Reading,
    Writing,
    Listening,
    Speaking,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEvidence {
    WrittenText,
    Transcript,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LessonTeachingRequest {
    pub request_id: String,
    pub lesson_id: String,
    pub title: String,
    pub objective: String,
    pub material: String,
    pub learner_context: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerFeedbackRequest {
    pub request_id: String,
    pub revision_id: String,
    pub attempt_id: Option<String>,
    pub context_id: Option<String>,
    pub exercise_id: Option<String>,
    /// Exact learner confirmation/edit of an independently transcribed recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_transcript_id: Option<String>,
    pub skill: TeacherSkill,
    pub task: String,
    pub prompt: String,
    pub reference: Option<String>,
    pub response: String,
    pub cues: Vec<String>,
    pub evidence: TextEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "mode",
    content = "input",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TeacherRequest {
    TeachLesson(LessonTeachingRequest),
    AnswerFeedback(AnswerFeedbackRequest),
}

fn bounded(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit
}
fn identity(text: &str) -> bool {
    bounded(text, 128)
        && text
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:/".contains(&c))
}

impl TeacherRequest {
    pub fn validate(&self) -> Result<(), TutorError> {
        let valid = match self {
            Self::TeachLesson(r) => {
                identity(&r.request_id)
                    && identity(&r.lesson_id)
                    && bounded(&r.title, 512)
                    && bounded(&r.objective, 2048)
                    && bounded(&r.material, 8192)
                    && r.learner_context.as_ref().is_none_or(|s| s.len() <= 2048)
            }
            Self::AnswerFeedback(r) => {
                identity(&r.request_id)
                    && identity(&r.revision_id)
                    && [&r.attempt_id, &r.context_id, &r.exercise_id, &r.confirmed_transcript_id]
                        .into_iter()
                        .all(|id| id.as_ref().is_none_or(|s| identity(s)))
                    && bounded(&r.task, 512)
                    && bounded(&r.prompt, 8192)
                    && r.reference.as_ref().is_none_or(|s| s.len() <= 8192)
                    && bounded(&r.response, 8192)
                    && r.cues.len() <= 16
                    && r.cues.iter().all(|c| bounded(c, 256))
                    && match r.evidence {
                        TextEvidence::WrittenText => r.confirmed_transcript_id.is_none(),
                        TextEvidence::Transcript => r.attempt_id.is_none() || r.confirmed_transcript_id.is_some(),
                    }
            }
        };
        if !valid {
            return Err(TutorError::InvalidRequest);
        }
        let size = serde_json::to_vec(self)
            .map_err(|_| TutorError::InvalidRequest)?
            .len();
        if size > 24_576 {
            return Err(TutorError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiVerdict {
    Correct,
    PartlyCorrect,
    Incorrect,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextCorrection {
    pub original: String,
    pub replacement: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AiAnswerFeedback {
    pub verdict: AiVerdict,
    pub suggested_response: Option<String>,
    pub corrections: Vec<TextCorrection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherPractice {
    pub prompt: String,
    pub expected_response: Option<String>,
    pub teaching_note: String,
    /// Concrete context for executable text practice. Absent in older cached responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_russian: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_english: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<TeacherSkill>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherContent {
    pub explanation: String,
    pub examples: Vec<TutorExample>,
    pub next_step: String,
    pub caveats: Vec<String>,
    pub feedback: Option<AiAnswerFeedback>,
    pub practice: Vec<TeacherPractice>,
}

impl TeacherContent {
    pub fn validate_for(&self, request: &TeacherRequest) -> Result<(), TutorError> {
        if !bounded(&self.explanation, 12000)
            || !bounded(&self.next_step, 2000)
            || self.examples.len() > 6
            || self.caveats.len() > 6
            || self.practice.len() > 4
            || self.examples.iter().any(|e| {
                !bounded(&e.russian, 2000) || !bounded(&e.english, 2000) || !bounded(&e.note, 2000)
            })
            || self.caveats.iter().any(|s| !bounded(s, 2000))
            || self.practice.iter().any(|p| {
                !bounded(&p.prompt, 2000)
                    || !bounded(&p.teaching_note, 2000)
                    || p.expected_response
                        .as_ref()
                        .is_some_and(|s| !bounded(s, 2000))
                    || match (&p.reference_russian, &p.reference_english, &p.skill) {
                        (None, None, None) => false,
                        (Some(russian), Some(english), Some(TeacherSkill::Reading | TeacherSkill::Writing)) =>
                            !bounded(russian, 2000) || !bounded(english, 2000),
                        _ => true,
                    }
            })
        {
            return Err(TutorError::InvalidResponse);
        }
        match (request, &self.feedback) {
            (TeacherRequest::TeachLesson(_), None) => {
                if self.examples.is_empty() || self.practice.is_empty() {
                    return Err(TutorError::InvalidResponse);
                }
            }
            (TeacherRequest::AnswerFeedback(r), Some(f)) => {
                if f.corrections.len() > 6
                    || f.suggested_response
                        .as_ref()
                        .is_some_and(|s| !bounded(s, 8192))
                    || f.corrections.iter().any(|c| {
                        c.original.len() > 2000
                            || !r.response.contains(&c.original)
                            || c.replacement.len() > 2000
                            || (c.original.is_empty() && c.replacement.is_empty())
                            || !bounded(&c.reason, 2000)
                    })
                {
                    return Err(TutorError::InvalidResponse);
                }
            }
            _ => return Err(TutorError::InvalidResponse),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TeacherResponse {
    /// Filled by the gateway from the submitted request, never from model-generated JSON.
    pub request: TeacherRequest,
    pub content: TeacherContent,
    pub provenance: TutorProvenance,
    pub usage: TutorUsage,
}
