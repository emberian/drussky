//! AI teacher feedback is useful, durable model opinion with exact input bindings.
//! It is never implicitly a human assessment or evidence about unheard speech acoustics.
use crate::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiTextEvidence {
    WrittenText,
    /// A transcript supplies language content, not evidence of pronunciation or fluency.
    Transcript,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiProvenance {
    pub provider: String,
    pub model: String,
    pub response_id: String,
    pub prompt_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiCorrection {
    /// Empty original/replacement supports an insertion/deletion; reason remains required.
    pub original: String,
    pub replacement: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiAssessment {
    pub request_id: String,
    pub attempt: Uuid,
    pub revision: Uuid,
    pub skill: Skill,
    pub task: TaskKind,
    /// Literal immutable learner response, so stale requests cannot be attached to another answer.
    pub response: String,
    /// Exact immutable context fact, when available. Later context corrections do not rewrite it.
    pub context: Option<Uuid>,
    /// Explicit learner-confirmed transcript allows language feedback about spoken words while
    /// retaining the immutable Attempt.response recording label. None preserves written semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_transcript: Option<Uuid>,
    pub evidence: AiTextEvidence,
    pub dimension: JudgmentDimension,
    pub verdict: Verdict,
    pub suggested_response: Option<String>,
    pub corrections: Vec<AiCorrection>,
    pub explanation: String,
    pub next_step: String,
    pub provenance: AiProvenance,
    pub supersedes: BTreeSet<Uuid>,
}
impl AiAssessment {
    pub fn validate(&self) -> Result<(), Error> {
        let text = |value: &str, max: usize| !value.trim().is_empty() && value.len() <= max;
        if !text(&self.request_id, 128)
            || self.attempt.is_nil()
            || self.revision.is_nil()
            || self.context.is_some_and(|id| id.is_nil())
            || self.confirmed_transcript.is_some_and(|id| id.is_nil())
            || (self.confirmed_transcript.is_some() && self.evidence != AiTextEvidence::Transcript)
            || self.supersedes.len() > 64
            || self.supersedes.iter().any(Uuid::is_nil)
            || self.response.len() > MAX_TEXT
            || !matches!(
                self.dimension,
                JudgmentDimension::Recall
                    | JudgmentDimension::Comprehension
                    | JudgmentDimension::Grammar
                    | JudgmentDimension::Spelling
            )
            || self
                .suggested_response
                .as_ref()
                .is_some_and(|s| !text(s, MAX_TEXT))
            || self.corrections.len() > 16
            || self.corrections.iter().any(|c| {
                c.original.len() > MAX_TEXT
                    || c.replacement.len() > MAX_TEXT
                    || !text(&c.reason, 8_000)
                    || (c.original.is_empty() && c.replacement.is_empty())
                    || (!c.original.is_empty() && !self.response.contains(&c.original))
            })
            || !text(&self.explanation, MAX_TEXT)
            || !text(&self.next_step, 8_000)
            || !text(&self.provenance.provider, 128)
            || !text(&self.provenance.model, 256)
            || !text(&self.provenance.response_id, 512)
            || !text(&self.provenance.prompt_version, 128)
        {
            return Err(Error::Invalid(
                "invalid AI assessment or unsupported text-evidence dimension".into(),
            ));
        }
        // Keep worst-case aggregate below the relay's per-event size budget after JSON escaping.
        let bytes = self.response.len()
            + self.explanation.len()
            + self.next_step.len()
            + self.suggested_response.as_ref().map_or(0, String::len)
            + self
                .corrections
                .iter()
                .map(|c| c.original.len() + c.replacement.len() + c.reason.len())
                .sum::<usize>();
        if bytes > 96_000 {
            return Err(Error::Invalid(
                "AI assessment exceeds its text budget".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn validate_ai_references(
    assessment: &AiAssessment,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    if let Some(event) = events.get(&assessment.attempt) {
        match &event.fact {
            Fact::AttemptRecorded(a)
                if a.revision == assessment.revision
                    && a.skill == assessment.skill
                    && (assessment.confirmed_transcript.is_some()
                        || a.response == assessment.response) => {}
            _ => {
                return Err(Error::Invalid(
                    "AI assessment binding differs from the exact attempt".into(),
                ));
            }
        }
    }
    if let Some(confirmation) = assessment
        .confirmed_transcript
        .and_then(|id| events.get(&id))
    {
        let Fact::TranscriptConfirmed(c) = &confirmation.fact else {
            return Err(Error::Invalid(
                "AI transcript evidence is not a learner confirmation".into(),
            ));
        };
        if c.text != assessment.response {
            return Err(Error::Invalid(
                "AI response differs from the confirmed learner text".into(),
            ));
        }
        if let Some(transcription) = events.get(&c.transcription) {
            match &transcription.fact {
                Fact::SpeechTranscribed(t)
                    if t.attempt == assessment.attempt && t.revision == assessment.revision => {}
                _ => {
                    return Err(Error::Invalid(
                        "AI transcript evidence belongs to another attempt or revision".into(),
                    ));
                }
            }
        }
    }
    if events
        .get(&assessment.revision)
        .is_some_and(|event| !matches!(event.fact, Fact::PassageRevised(_)))
    {
        return Err(Error::Invalid(
            "AI assessment revision target is not a revision".into(),
        ));
    }
    if let Some(context) = assessment.context.and_then(|id| events.get(&id)) {
        match &context.fact {
            Fact::AttemptContextRecorded(c)
                if c.attempt == assessment.attempt && c.task == assessment.task => {}
            _ => {
                return Err(Error::Invalid(
                    "AI assessment context differs from the exact task".into(),
                ));
            }
        }
    }
    Ok(())
}
impl Library {
    /// Active AI opinions only. They do not appear in human judgments or independent recall counts.
    /// Concurrent provider responses remain visible until an explicit superseding assessment.
    pub fn ai_assessments(
        &self,
        attempt: Uuid,
        dimension: JudgmentDimension,
    ) -> Vec<(&Event, &AiAssessment)> {
        let superseded: BTreeSet<_> = self
            .events
            .values()
            .filter_map(|event| match &event.fact {
                Fact::AiAssessmentRecorded(a) => Some(a.supersedes.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        self.events
            .values()
            .filter_map(|event| match &event.fact {
                Fact::AiAssessmentRecorded(a)
                    if a.attempt == attempt
                        && a.dimension == dimension
                        && !superseded.contains(&event.id) =>
                {
                    Some((event, a))
                }
                _ => None,
            })
            .collect()
    }
}
