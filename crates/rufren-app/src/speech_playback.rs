//! Attach listening opportunities to the reference actually replayed.
use crate::{audio_studio::StudioRegionPlayback, model::AppState, repository::Result};
use rufren_core::*;
use uuid::Uuid;

impl AppState {
    /// The studio checks PCM identity/frame bounds and acknowledges playback.
    /// Here we check the immutable notebook binding. A learner's own recording
    /// cannot establish that the reference's words were heard.
    pub(crate) fn heard_region(&mut self, played: StudioRegionPlayback) -> Result<()> {
        if played.recording.role != "reference" {
            return Ok(());
        }
        let Ok(revision) = Uuid::parse_str(&played.recording.revision_id) else {
            return Ok(());
        };
        self.refresh()?;
        let bound = self.library.events.values().any(|event| match &event.fact {
            Fact::RecordingAttached(attachment) => {
                attachment.role == AudioRole::Reference
                    && attachment.target == AnnotationTarget::Revision(revision)
                    && attachment.asset_id == played.recording.asset_id
                    && attachment.asset.sha256 == played.recording.content_hash
            }
            _ => false,
        });
        let Some(reference) = self
            .library
            .revision(revision)
            .filter(|r| bound && r.passage.id.to_string() == played.recording.passage_id)
        else {
            return Ok(());
        };
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.revision == revision && s.phase == Phase::Retrieve)
        {
            return self.heard_reference();
        }
        // Conservatively mark this reference as encountered, even when only a
        // selected region played. This is not a transcript or whole-text recall.
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
    use crate::{audio_studio::StudioRecording, repository::Repository};
    use rufren_audio::RegionSpan;
    use std::rc::Rc;

    fn reference(state: &mut AppState, text: &str) -> StudioRegionPlayback {
        let passage = Uuid::new_v4();
        let revision = Event::new(
            state.device,
            1,
            Fact::PassageRevised(Revision {
                passage: Passage {
                    id: passage,
                    title: text.into(),
                    russian: text.into(),
                    translation: "Fixture meaning".into(),
                    source: "Public synthetic test".into(),
                    coach_reviewed: false,
                },
                parents: Default::default(),
            }),
        )
        .unwrap();
        let record = StudioRecording {
            asset_id: Uuid::new_v4().to_string(),
            content_hash: "a".repeat(64),
            role: "reference".into(),
            passage_id: passage.to_string(),
            revision_id: revision.id.to_string(),
            sample_rate: 24000,
            channels: 1,
            frames: 24000,
            ..Default::default()
        };
        let attachment = Event::new(
            state.device,
            2,
            Fact::RecordingAttached(AudioAttachment {
                asset_id: record.asset_id.clone(),
                asset: AudioAsset {
                    sha256: record.content_hash.clone(),
                    media_type: "audio/wav".into(),
                    sample_rate_hz: 24000,
                    channels: 1,
                    frames: 24000,
                },
                target: AnnotationTarget::Revision(revision.id),
                role: AudioRole::Reference,
                note: "Fixture".into(),
            }),
        )
        .unwrap();
        state.repo.append(&[revision, attachment]).unwrap();
        state.refresh().unwrap();
        StudioRegionPlayback {
            recording: record,
            slot: "A".into(),
            source_pcm_hash: "b".repeat(64),
            source_kind: "original_wav".into(),
            sample_rate: 24000,
            span: RegionSpan {
                start_frame: 0,
                end_frame: 12000,
            },
            playback_rate: 1.0,
        }
    }
    #[test]
    fn region_replay_uses_the_actual_reference_and_never_the_other_active_task() {
        let mut state =
            AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap();
        let current = reference(&mut state, "Привет.");
        let foreign = reference(&mut state, "Пока.");
        let current_id = Uuid::parse_str(&current.recording.revision_id).unwrap();
        let foreign_id = Uuid::parse_str(&foreign.recording.revision_id).unwrap();
        state.start(current_id, Skill::Speaking).unwrap();
        state.phase(Phase::Retrieve).unwrap();
        state.heard_region(foreign).unwrap();
        assert_eq!(state.plan.as_ref().unwrap().reference_plays, 0);
        let seen: Vec<_> = state
            .library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::StimulusPresented(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].revision, Some(foreign_id));
        assert_eq!(seen[0].session, None);
        state.heard_region(current).unwrap();
        assert_eq!(state.plan.as_ref().unwrap().reference_plays, 1);
        assert!(state.session.as_ref().unwrap().cues.contains(&Cue::Replay));
    }
    #[test]
    fn learner_audio_and_mismatched_attachment_hash_are_not_reference_evidence() {
        let mut state =
            AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap();
        let mut played = reference(&mut state, "Привет.");
        played.recording.role = "attempt".into();
        state.heard_region(played.clone()).unwrap();
        played.recording.role = "reference".into();
        played.recording.content_hash = "c".repeat(64);
        state.heard_region(played).unwrap();
        assert!(
            !state
                .library
                .events
                .values()
                .any(|e| matches!(e.fact, Fact::StimulusPresented(_)))
        );
    }
}
