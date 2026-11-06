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
    /// Explicit opt-in: omitted requests retain the original response shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lesson_flow: Option<LessonFlowCapability>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonFlowCapability {
    GuidedV1,
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

/// Oral references must fit the speech service's bounded single-utterance input.
/// Contract tests compare these limits and validation with `rufren-speech`.
pub const ORAL_REFERENCE_MAX_BYTES: usize = 1024;
pub const ORAL_REFERENCE_MAX_CHARS: usize = 256;

fn oral_reference(text: &str) -> bool {
    bounded(text, ORAL_REFERENCE_MAX_BYTES)
        && text.chars().count() <= ORAL_REFERENCE_MAX_CHARS
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        && text.chars().any(char::is_alphabetic)
        && !text.chars().any(|c| matches!(c, '<' | '>'))
        && !text.contains("[[")
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
    pub fn wants_lesson_flow(&self) -> bool {
        matches!(
            self,
            Self::TeachLesson(LessonTeachingRequest {
                lesson_flow: Some(LessonFlowCapability::GuidedV1),
                ..
            })
        )
    }

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
                    && [
                        &r.attempt_id,
                        &r.context_id,
                        &r.exercise_id,
                        &r.confirmed_transcript_id,
                    ]
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
                        TextEvidence::Transcript => {
                            r.attempt_id.is_none() || r.confirmed_transcript_id.is_some()
                        }
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
    /// Exact displayed reading stimulus, hidden listening stimulus, or model answer
    /// for writing/speaking. Listening text stays concealed until response/reveal.
    /// Absent together with meaning/skill in older cached responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_russian: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_english: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<TeacherSkill>,
}

/// Response-local references preserve the original examples and practice identities.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherLessonFlow {
    pub steps: Vec<TeacherLessonStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeacherLessonStep {
    /// Stable within this immutable teacher response; never a cross-response identity.
    pub id: String,
    pub goal: String,
    pub orientation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub examples: Vec<LessonExampleRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practice_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LessonExampleRef {
    pub example_index: usize,
    pub presentation: ExamplePresentation,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExamplePresentation {
    Model,
    OptionalSupport,
}

pub const LESSON_FLOW_MAX_STEPS: usize = 6;

impl TeacherLessonFlow {
    fn validate_for(&self, content: &TeacherContent) -> Result<(), TutorError> {
        use std::collections::BTreeSet;
        if self.steps.is_empty() || self.steps.len() > LESSON_FLOW_MAX_STEPS {
            return Err(TutorError::InvalidResponse);
        }
        let mut ids = BTreeSet::new();
        let mut practices = BTreeSet::new();
        for step in &self.steps {
            if !bounded(&step.id, 64)
                || !step
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
                || !ids.insert(&step.id)
                || !bounded(&step.goal, 512)
                || !bounded(&step.orientation, 2000)
                || step.detail.as_ref().is_some_and(|s| !bounded(s, 6000))
                || step.examples.len() > 6
                || (step.examples.is_empty() && step.practice_index.is_none())
            {
                return Err(TutorError::InvalidResponse);
            }
            let practice = step
                .practice_index
                .map(|index| {
                    if !practices.insert(index) {
                        return Err(TutorError::InvalidResponse);
                    }
                    content
                        .practice
                        .get(index)
                        .ok_or(TutorError::InvalidResponse)
                })
                .transpose()?;
            let mut examples = BTreeSet::new();
            for example in &step.examples {
                if example.example_index >= content.examples.len()
                    || !examples.insert(example.example_index)
                    // Listening keeps written models behind a deliberate support reveal.
                    || (practice.is_some_and(|p| p.skill == Some(TeacherSkill::Listening))
                        && example.presentation == ExamplePresentation::Model)
                {
                    return Err(TutorError::InvalidResponse);
                }
            }
        }
        if practices.len() != content.practice.len() {
            return Err(TutorError::InvalidResponse);
        }
        Ok(())
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lesson_flow: Option<TeacherLessonFlow>,
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
                        (Some(russian), Some(english), Some(skill)) => {
                            !bounded(russian, 2000)
                                || !bounded(english, 2000)
                                || (matches!(
                                    skill,
                                    TeacherSkill::Listening | TeacherSkill::Speaking
                                ) && !oral_reference(russian))
                        }
                        _ => true,
                    }
            })
        {
            return Err(TutorError::InvalidResponse);
        }
        match (request.wants_lesson_flow(), &self.lesson_flow) {
            (true, Some(flow)) => flow.validate_for(self)?,
            (false, None) => {}
            _ => return Err(TutorError::InvalidResponse),
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

#[cfg(test)]
mod oral_tests {
    use super::*;
    use serde_json::{Value, json};

    fn lesson() -> TeacherRequest {
        TeacherRequest::TeachLesson(LessonTeachingRequest {
            request_id: "oral-contract-request".into(),
            lesson_id: "greeting".into(),
            title: "A greeting".into(),
            objective: "Understand and return a greeting.".into(),
            material: "Здравствуйте! — Hello!".into(),
            learner_context: None,
            lesson_flow: None,
        })
    }

    fn content(practice: Value) -> TeacherContent {
        serde_json::from_value(json!({
            "explanation":"Use this greeting for polite address.",
            "examples":[{"russian":"Здравствуйте!","english":"Hello!","note":"Polite."}],
            "next_step":"Try the practice item.","caveats":[],"feedback":null,
            "practice":[practice]
        }))
        .unwrap()
    }

    fn practice(skill: &str, russian: &str) -> Value {
        let (prompt, expected) = match skill {
            "reading" => (
                "What does the displayed greeting mean in English?",
                "Hello!",
            ),
            "writing" => ("Write a polite hello in Russian.", "Здравствуйте!"),
            "listening" => (
                "Listen, then tell me in English what the speaker means.",
                "Hello!",
            ),
            "speaking" => (
                "Record yourself greeting a stranger politely in Russian.",
                "Здравствуйте!",
            ),
            _ => unreachable!(),
        };
        json!({"prompt":prompt,"expected_response":expected,"teaching_note":"Accept an appropriate equivalent greeting.",
            "reference_russian":russian,"reference_english":"Hello!","skill":skill})
    }

    #[test]
    fn teacher_oral_all_four_skills_round_trip_and_legacy_practice_remains_readable() {
        for skill in ["reading", "writing", "listening", "speaking"] {
            let value = practice(skill, "Здравствуйте!");
            let parsed = content(value.clone());
            parsed.validate_for(&lesson()).unwrap();
            assert_eq!(serde_json::to_value(&parsed.practice[0]).unwrap(), value);
        }
        let legacy = json!({"prompt":"Say hello.","expected_response":"Здравствуйте!","teaching_note":"Polite."});
        let parsed = content(legacy.clone());
        parsed.validate_for(&lesson()).unwrap();
        assert_eq!(serde_json::to_value(&parsed.practice[0]).unwrap(), legacy);
    }

    #[test]
    fn teacher_oral_partial_reference_triples_and_oversized_task_sets_are_rejected() {
        for missing in ["reference_russian", "reference_english", "skill"] {
            let mut incomplete = practice("listening", "Здравствуйте!");
            incomplete.as_object_mut().unwrap().remove(missing);
            assert!(matches!(
                content(incomplete).validate_for(&lesson()),
                Err(TutorError::InvalidResponse)
            ));
        }
        let mut too_many = content(practice("speaking", "Здравствуйте!"));
        too_many.practice = vec![too_many.practice[0].clone(); 5];
        assert!(matches!(
            too_many.validate_for(&lesson()),
            Err(TutorError::InvalidResponse)
        ));
    }

    #[test]
    fn teacher_oral_reference_limits_match_speech_without_shrinking_text_practice() {
        assert_eq!(ORAL_REFERENCE_MAX_BYTES, rufren_speech::MAX_TEXT_BYTES);
        assert_eq!(ORAL_REFERENCE_MAX_CHARS, rufren_speech::MAX_TEXT_CHARS);
        let samples = [
            "Здравствуйте!".to_owned(),
            "Это за́мок.".into(),
            "Привет!\nКак дела?".into(),
            "а".repeat(256),
            "а".repeat(257),
            "".into(),
            "123".into(),
            "Привет\0".into(),
            "<voice>Привет</voice>".into(),
            "[[rate 100]]Привет".into(),
        ];
        for text in samples {
            let speech = rufren_speech::SpeechRequest {
                text: text.clone(),
                request_id: "oral-contract".into(),
                revision_id: None,
            };
            for skill in ["listening", "speaking"] {
                assert_eq!(
                    content(practice(skill, &text))
                        .validate_for(&lesson())
                        .is_ok(),
                    speech.validate().is_ok(),
                    "{skill} reference mismatch"
                );
            }
        }
        for skill in ["reading", "writing"] {
            content(practice(skill, &"а".repeat(700)))
                .validate_for(&lesson())
                .unwrap();
        }
    }
}
