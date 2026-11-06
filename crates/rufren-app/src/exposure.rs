//! Immutable opportunities to see or hear material, distinct from planning or mastery.
use crate::{
    model::AppState,
    repository::{Result, now_ms},
};
use rufren_core::*;
use uuid::Uuid;

impl AppState {
    pub(crate) fn note_presentation(
        &mut self,
        id: Uuid,
        presentation: StimulusPresentation,
    ) -> Result<()> {
        if !presentation
            .russian
            .chars()
            .any(|c| ('\u{0400}'..='\u{052f}').contains(&c))
        {
            return Ok(());
        }
        self.refresh()?;
        let fact = Fact::StimulusPresented(presentation);
        if let Some(existing) = self.library.events.get(&id) {
            return if existing.fact == fact {
                Ok(())
            } else {
                Err(
                    "This presentation conflicts with an existing record. Your history is kept."
                        .into(),
                )
            };
        }
        if let Fact::StimulusPresented(shown) = &fact {
            if let Some((answer, draft)) = self.active_teacher_revisit()? {
                let exercise = answer
                    .content
                    .practice
                    .get(draft.index)
                    .ok_or("The active revisit no longer identifies its original question")?;
                let targets: Vec<&str> = exercise
                    .reference_russian
                    .as_deref()
                    .into_iter()
                    .chain(exercise.expected_response.as_deref())
                    .collect();
                if presentation_reveals_target(shown, draft.id, &targets) {
                    // Visibility, not storage or rendering, reaches this seam.
                    // An invalid pointer or failed cue write must remain visible
                    // as an error rather than allowing an unsupported claim.
                    let cue = if shown.session == Some(draft.id)
                        && shown.purpose == PresentationPurpose::Audio
                    {
                        Cue::Replay
                    } else {
                        Cue::ReferenceMaterial
                    };
                    self.teacher_revisit_mark_active_support(cue)?;
                }
            }
        }
        let event = Event {
            version: EVENT_VERSION,
            id,
            device: self.device,
            recorded_at_ms: now_ms(),
            fact,
        };
        self.repo.append(&[event])?;
        self.refresh()
    }

    pub(crate) fn presentation_decision(&self) -> Option<Uuid> {
        let session = self.session.as_ref()?;
        self.library
            .decisions(session.id)
            .into_iter()
            .filter(|(_, decision)| {
                decision.selected.revision == session.revision
                    && decision.selected.skill == session.skill
            })
            .max_by_key(|(event, _)| (event.recorded_at_ms, event.id))
            .map(|(event, _)| event.id)
    }

    /// Called only by the player's successful playback acknowledgment.
    /// The text identifies the reference's intended content, not a phonetic verification.
    pub(crate) fn note_reference_presentation(&mut self) -> Result<()> {
        let Some(session) = self.session.as_ref() else {
            return Ok(());
        };
        let Some(revision) = self.library.revision(session.revision) else {
            return Ok(());
        };
        self.note_presentation(
            Uuid::new_v4(),
            StimulusPresentation {
                session: Some(session.id),
                revision: Some(session.revision),
                russian: revision.passage.russian.clone(),
                purpose: PresentationPurpose::Audio,
                decision: self.presentation_decision(),
                utterance_family: None,
            },
        )
    }
}

/// Required reading text and the first listening play use TaskInput, not help.
/// Own Audio facts identify additional replay/model support, including facts
/// which survived a failed local draft write after playback acknowledgment.
/// A different task/window showing the same exact target is intervening restudy.
fn presentation_reveals_target(
    shown: &StimulusPresentation,
    review_session: Uuid,
    targets: &[&str],
) -> bool {
    if shown.session == Some(review_session) && shown.purpose == PresentationPurpose::TaskInput {
        return false;
    }
    targets
        .iter()
        .any(|target| stimulus_contains(&shown.russian, target))
}

#[cfg(test)]
mod revisit_support_tests {
    use super::*;

    #[test]
    fn required_stimulus_is_not_help_but_revealed_or_foreign_target_is() {
        let review = Uuid::new_v4();
        let mut shown = StimulusPresentation {
            session: Some(review),
            revision: None,
            russian: "Я чита́ю кни́гу.".into(),
            purpose: PresentationPurpose::TaskInput,
            decision: None,
            utterance_family: None,
        };
        let target = ["Я читаю книгу."];
        assert!(!presentation_reveals_target(&shown, review, &target));
        shown.purpose = PresentationPurpose::Audio;
        assert!(
            presentation_reveals_target(&shown, review, &target),
            "own Audio is replay/model support; first required play is TaskInput"
        );
        for purpose in [
            PresentationPurpose::Answer,
            PresentationPurpose::Reference,
            PresentationPurpose::Instruction,
        ] {
            shown.purpose = purpose;
            assert!(presentation_reveals_target(&shown, review, &target));
        }
        shown.session = Some(Uuid::new_v4());
        shown.purpose = PresentationPurpose::TaskInput;
        assert!(presentation_reveals_target(&shown, review, &target));
        shown.purpose = PresentationPurpose::Audio;
        assert!(presentation_reveals_target(&shown, review, &target));
        shown.session = None;
        assert!(presentation_reveals_target(&shown, review, &target));
        shown.russian = "Она читает книгу.".into();
        assert!(!presentation_reveals_target(&shown, review, &target));
    }
}
