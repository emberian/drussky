//! Immutable opportunities to see or hear material, distinct from planning or mastery.
use crate::{
    model::AppState,
    repository::{now_ms, Result},
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
