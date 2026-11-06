//! Stable, reusable references for examples the learner chooses to hear.
use crate::{model::AppState, repository::Result, teacher_practice::practice_source};
use rufren_core::*;
use rufren_tutor::{ReviewStatus, TeacherResponse};
use uuid::Uuid;

pub(crate) fn example_reference(answer: &TeacherResponse, index: usize) -> Result<Event> {
    answer.request.validate().map_err(|e| e.to_string())?;
    answer
        .content
        .validate_for(&answer.request)
        .map_err(|e| e.to_string())?;
    if answer.provenance.review_status != ReviewStatus::AiGeneratedTeaching
        || answer.provenance.response_id.is_empty()
    {
        return Err("This example is missing its teaching source.".into());
    }
    let example = answer
        .content
        .examples
        .get(index)
        .ok_or("This example is unavailable.")?;
    let source = Uuid::parse_str(&practice_source(answer)).map_err(|e| e.to_string())?;
    let passage = Uuid::new_v5(&source, format!("voice-example-v1:{index}").as_bytes());
    Ok(Event {
        version: 2,
        id: Uuid::new_v5(&passage, b"reference-v1"),
        device: source,
        recorded_at_ms: 0,
        fact: Fact::PassageRevised(Revision {
            passage: Passage {
                id: passage,
                title: example.english.clone(),
                russian: example.russian.clone(),
                translation: example.english.clone(),
                source: format!(
                    "AI teacher example · {} · {} · {} · {} · {}",
                    answer.provenance.provider,
                    answer.provenance.model,
                    answer.provenance.response_id,
                    index,
                    example.note
                ),
                coach_reviewed: false,
            },
            parents: Default::default(),
        }),
    })
}

impl AppState {
    pub(crate) fn ensure_teacher_example(
        &mut self,
        answer: &TeacherResponse,
        index: usize,
    ) -> Result<Uuid> {
        let event = example_reference(answer, index)?;
        self.refresh()?;
        if let Some(existing) = self.library.events.get(&event.id) {
            if existing != &event {
                return Err(
                    "This example differs from its saved source. Your earlier reference is kept."
                        .into(),
                );
            }
        } else {
            self.repo.append(std::slice::from_ref(&event))?;
            self.refresh()?;
        }
        Ok(event.id)
    }

    pub(crate) fn heard_teacher_example(&mut self, revision: Uuid) -> Result<()> {
        let Some(reference) = self.library.revision(revision) else {
            return Ok(());
        };
        self.note_presentation(
            Uuid::new_v4(),
            StimulusPresentation {
                session: None,
                revision: Some(revision),
                russian: reference.passage.russian.clone(),
                purpose: PresentationPurpose::Audio,
                decision: None,
                utterance_family: None,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Repository;
    use rufren_tutor::*;
    use std::rc::Rc;

    fn response() -> TeacherResponse {
        TeacherResponse {
            request: TeacherRequest::TeachLesson(LessonTeachingRequest {
                lesson_flow: None,
                request_id: "example-fixture".into(),
                lesson_id: "greetings".into(),
                title: "A greeting".into(),
                objective: "Greet someone".into(),
                material: "Привет. — Hello.".into(),
                learner_context: None,
            }),
            provenance: TutorProvenance {
                provider: "fixture".into(),
                model: "fixture".into(),
                review_status: ReviewStatus::AiGeneratedTeaching,
                scope: "Text teaching".into(),
                prompt_version: "fixture-v1".into(),
                response_id: "example-response".into(),
            },
            usage: TutorUsage {
                accounting: UsageAccounting::SubscriptionQuota,
                input_tokens: 0,
                output_tokens: 0,
                estimated_micro_usd: 0,
                reserved_micro_usd: 0,
            },
            content: TeacherContent {
                lesson_flow: None,
                explanation: "A short greeting.".into(),
                examples: vec![TutorExample {
                    russian: "Привет.".into(),
                    english: "Hello.".into(),
                    note: "Informal.".into(),
                }],
                next_step: "Listen to Привет.".into(),
                caveats: vec![],
                feedback: None,
                practice: vec![TeacherPractice {
                    prompt: "What does Привет mean?".into(),
                    expected_response: Some("Hello.".into()),
                    teaching_note: "Accept hi.".into(),
                    reference_russian: Some("Привет.".into()),
                    reference_english: Some("Hello.".into()),
                    skill: Some(TeacherSkill::Reading),
                }],
            },
        }
    }
    fn state() -> AppState {
        AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap()
    }

    #[test]
    fn teacher_examples_reuse_exact_reference_without_fabricating_exposure() {
        let answer = response();
        let mut a = state();
        let mut b = state();
        let id = a.ensure_teacher_example(&answer, 0).unwrap();
        let size = a.library.events.len();
        assert_eq!(a.ensure_teacher_example(&answer, 0).unwrap(), id);
        assert_eq!(a.library.events.len(), size);
        assert_eq!(b.ensure_teacher_example(&answer, 0).unwrap(), id);
        assert_eq!(a.library.events.get(&id), b.library.events.get(&id));
        assert!(
            !a.library
                .events
                .values()
                .any(|e| matches!(e.fact, Fact::StimulusPresented(_)))
        );
        a.heard_teacher_example(id).unwrap();
        let presentations: Vec<_> = a
            .library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::StimulusPresented(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(presentations.len(), 1);
        assert_eq!(presentations[0].revision, Some(id));
        assert_eq!(presentations[0].session, None);
        assert_eq!(presentations[0].russian, "Привет.");
    }

    #[test]
    fn teacher_examples_reject_changed_source_and_separate_new_responses() {
        let mut a = state();
        let mut answer = response();
        let original = a.ensure_teacher_example(&answer, 0).unwrap();
        answer.content.examples[0].russian = "Пока.".into();
        assert!(a.ensure_teacher_example(&answer, 0).is_err());
        assert_eq!(
            a.library.revision(original).unwrap().passage.russian,
            "Привет."
        );
        answer.provenance.response_id = "another-response".into();
        assert_ne!(a.ensure_teacher_example(&answer, 0).unwrap(), original);
        assert!(example_reference(&answer, 99).is_err());
    }
}
