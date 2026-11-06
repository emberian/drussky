//! Actual opportunities to see/hear material. These facts do not assert attention or learning.
use crate::*;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresentationPurpose {
    Instruction,
    TaskInput,
    Answer,
    Reference,
    Audio,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StimulusPresentation {
    pub session: Option<Uuid>,
    pub revision: Option<Uuid>,
    pub russian: String,
    pub purpose: PresentationPurpose,
    pub decision: Option<Uuid>,
    pub utterance_family: Option<String>,
}
impl StimulusPresentation {
    pub fn validate(&self) -> Result<(), Error> {
        if self.session.is_some_and(|id| id.is_nil())
            || self.revision.is_some_and(|id| id.is_nil())
            || self.decision.is_some_and(|id| id.is_nil())
            || (self.decision.is_some() && self.session.is_none())
            || self.russian.trim().is_empty()
            || self.russian.len() > MAX_TEXT
            || self
                .utterance_family
                .as_ref()
                .is_some_and(|s| s.trim().is_empty() || s.len() > 128)
        {
            return Err(Error::Invalid("invalid presented stimulus".into()));
        }
        Ok(())
    }
}
pub(crate) fn validate_presentation_references(
    p: &StimulusPresentation,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    if p.revision
        .and_then(|id| events.get(&id))
        .is_some_and(|event| !matches!(event.fact, Fact::PassageRevised(_)))
    {
        return Err(Error::Invalid(
            "presented stimulus revision is not a revision".into(),
        ));
    }
    if let Some(event) = p.decision.and_then(|id| events.get(&id)) {
        match &event.fact {
            Fact::TeachingDecisionRecorded(d) if Some(d.session) == p.session => {}
            _ => {
                return Err(Error::Invalid(
                    "presentation refers to another teaching session".into(),
                ));
            }
        }
    }
    // `revision` identifies associated source, not proof that the displayed crop/variant or
    // recorded speaker exactly matched that source. The actual shown/intended text is retained.
    Ok(())
}
impl Library {
    pub fn presentations(&self) -> impl Iterator<Item = (&Event, &StimulusPresentation)> {
        self.events.values().filter_map(|event| match &event.fact {
            Fact::StimulusPresented(p) => Some((event, p)),
            _ => None,
        })
    }
    /// New means no prior observation in the available app log, never outside-app unfamiliarity.
    /// Definition/version is not exposure. A retrospective probe needs an actual first TaskInput
    /// presentation; legacy attempts without it have unknown pre-response history. Its own first
    /// input is not mistaken for prior restudy. This is novelty only within the available log.
    pub(crate) fn presentation_novelty(
        &self,
        revision: Uuid,
        before_ms: i64,
        excluding_attempt: Option<Uuid>,
    ) -> StimulusNovelty {
        let Some(target) = self.revision(revision) else {
            return StimulusNovelty::UnknownHistory;
        };
        let candidate = &target.passage.russian;
        if stimulus_identity(candidate).is_empty() {
            return StimulusNovelty::UnknownHistory;
        }
        let first_input = excluding_attempt
            .and_then(|id| self.attempt(id))
            .and_then(|attempt| {
                self.presentations()
                    .filter(|(e, p)| {
                        e.recorded_at_ms <= before_ms
                            && p.session == Some(attempt.session)
                            && p.purpose == PresentationPurpose::TaskInput
                            && stimulus_contains(&p.russian, candidate)
                    })
                    .min_by_key(|(e, _)| (e.recorded_at_ms, e.id))
            });
        let boundary = first_input.map_or(before_ms, |(event, _)| event.recorded_at_ms);
        let family = first_input.and_then(|(_, p)| p.utterance_family.as_deref());
        let previous_presentation = self
            .presentations()
            .filter(|(e, p)| {
                e.recorded_at_ms <= boundary
                    && first_input.is_none_or(|(first, _)| e.id != first.id)
                    && (stimulus_contains(&p.russian, candidate)
                        || family
                            .is_some_and(|family| p.utterance_family.as_deref() == Some(family)))
            })
            .map(|(e, _)| e)
            .min_by_key(|e| (e.recorded_at_ms, e.id));
        let previous_attempt = self
            .attempts()
            .filter(|(e, a)| {
                e.recorded_at_ms <= boundary
                    && Some(e.id) != excluding_attempt
                    && self
                        .revision(a.revision)
                        .is_some_and(|r| stimulus_contains(&r.passage.russian, candidate))
            })
            .map(|(e, _)| e)
            .min_by_key(|e| (e.recorded_at_ms, e.id));
        if let Some(event) = previous_presentation
            .into_iter()
            .chain(previous_attempt)
            .min_by_key(|e| (e.recorded_at_ms, e.id))
        {
            return StimulusNovelty::PreviouslyExposed { event: event.id };
        }
        if excluding_attempt.is_some() && first_input.is_none() {
            StimulusNovelty::UnknownHistory
        } else {
            StimulusNovelty::NoPriorRecordedExposure
        }
    }
    /// Observed instruction/answer presentation linked to this decision or completed practice
    /// anchors the revisit. A merely selected action, asset attachment, or failed play click does not.
    pub fn teaching_activity_time(&self, decision_id: Uuid, now_ms: i64) -> Option<i64> {
        let Fact::TeachingDecisionRecorded(_) = &self.events.get(&decision_id)?.fact else {
            return None;
        };
        self.presentations()
            .filter(|(e, p)| {
                e.recorded_at_ms <= now_ms
                    && p.decision == Some(decision_id)
                    && matches!(
                        p.purpose,
                        PresentationPurpose::Instruction | PresentationPurpose::Answer
                    )
            })
            .map(|(e, _)| e.recorded_at_ms)
            .chain(
                self.completed_practice_for_decision(decision_id, now_ms)
                    .map(|(e, _)| e.recorded_at_ms),
            )
            .max()
    }
}
