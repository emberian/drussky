//! Immutable assessments and context. A coach identity is provenance, not authentication.
use crate::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    MeaningRecall,
    FormRecall,
    SoundDiscrimination,
    Pronunciation,
    Spelling,
    Composition,
    GrammarGeneralization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttemptContext {
    pub attempt: Uuid,
    #[serde(default)]
    pub content_pack: Option<String>,
    #[serde(default)]
    pub content_revision: Option<String>,
    #[serde(default)]
    pub exercise_id: Option<String>,
    /// Exact authored question and option IDs/text; independent of a device-local session draft.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Authored answer/example with its scope, not an automatic correctness judgment.
    #[serde(default)]
    pub reference_answer: Option<String>,
    pub task: TaskKind,
    /// Stable, curriculum-owned feature IDs. Exposure never implies mastery of these features.
    pub target_features: BTreeSet<String>,
    pub novel_context: bool,
    pub delayed_probe: bool,
    pub practice_duration_ms: Option<u64>,
    /// Learner report on a five-point scale; distinct from correctness.
    pub effort: Option<u8>,
    /// Optional response category for a deliberately diagnostic grammar task.
    pub grammar_response: Option<GrammarResponse>,
    pub supersedes: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentDimension {
    Recall,
    Comprehension,
    Grammar,
    Pronunciation,
    Fluency,
    Intelligibility,
    Spelling,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Correct,
    PartlyCorrect,
    Incorrect,
    Uncertain,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CoachProvenance {
    pub coach: String,
    pub method: String,
    pub note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CoachJudgment {
    pub attempt: Uuid,
    pub dimension: JudgmentDimension,
    pub verdict: Verdict,
    /// Expressed confidence, not an empirically calibrated probability.
    pub confidence_percent: u8,
    pub provenance: CoachProvenance,
    /// Explicit correction/resolution; concurrent heads remain visible.
    pub supersedes: BTreeSet<Uuid>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "target", content = "id", rename_all = "snake_case")]
pub enum AnnotationTarget {
    Revision(Uuid),
    Attempt(Uuid),
}
impl AnnotationTarget {
    pub fn id(self) -> Uuid {
        match self {
            Self::Revision(id) | Self::Attempt(id) => id,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    /// UTF-8 byte boundaries in the exact Russian revision or learner response.
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CoachAnnotation {
    pub target: AnnotationTarget,
    pub span: Option<TextSpan>,
    pub note: String,
    pub suggested_correction: Option<String>,
    pub provenance: CoachProvenance,
    pub supersedes: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioRole {
    Reference,
    Attempt,
}

/// Replicated metadata only. Attachment neither guarantees local bytes nor demonstrates learning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AudioAttachment {
    pub asset_id: String,
    pub asset: AudioAsset,
    pub target: AnnotationTarget,
    pub role: AudioRole,
    pub note: String,
}

impl AudioAsset {
    pub fn validate(&self) -> Result<(), Error> {
        let subtype = self
            .media_type
            .split(';')
            .next()
            .unwrap_or("")
            .strip_prefix("audio/")
            .unwrap_or("");
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !self.media_type.starts_with("audio/")
            || subtype.is_empty()
            || !subtype
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&b))
            || self.media_type.len() > 128
            || !self
                .media_type
                .bytes()
                .all(|b| b.is_ascii_graphic() || b == b' ')
            || !(8_000..=384_000).contains(&self.sample_rate_hz)
            || !(1..=8).contains(&self.channels)
            || self.frames == 0
            || self.frames > u64::from(self.sample_rate_hz) * 24 * 60 * 60
        {
            return Err(Error::Invalid("invalid audio asset metadata".into()));
        }
        Ok(())
    }
}

pub(crate) fn learning_parents(fact: &Fact) -> BTreeSet<Uuid> {
    match fact {
        Fact::AttemptContextRecorded(x) => x.supersedes.clone(),
        Fact::CoachJudgmentRecorded(x) => x.supersedes.clone(),
        Fact::CoachAnnotationRecorded(x) => x.supersedes.clone(),
        Fact::AiAssessmentRecorded(x) => x.supersedes.clone(),
        Fact::TranscriptConfirmed(x) => x.supersedes.clone(),
        _ => BTreeSet::new(),
    }
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn provenance(p: &CoachProvenance) -> bool {
    !p.coach.trim().is_empty()
        && p.coach.len() <= 256
        && !p.method.trim().is_empty()
        && p.method.len() <= 512
        && p.note.len() <= MAX_TEXT
}
pub(crate) fn validate_learning_fact(event: &Event) -> Result<(), Error> {
    let parents = learning_parents(&event.fact);
    if parents.len() > 64 || parents.contains(&event.id) || parents.iter().any(Uuid::is_nil) {
        return Err(invalid("invalid assessment supersession"));
    }
    let valid = match &event.fact {
        Fact::AttemptContextRecorded(x) => {
            !x.attempt.is_nil()
                && [&x.content_pack, &x.content_revision, &x.exercise_id]
                    .iter()
                    .all(|s| {
                        s.as_ref()
                            .is_none_or(|s| !s.trim().is_empty() && s.len() <= 128)
                    })
                && [&x.prompt, &x.reference_answer].iter().all(|s| {
                    s.as_ref()
                        .is_none_or(|s| !s.trim().is_empty() && s.len() <= 8192)
                })
                && x.target_features.len() <= 64
                && x.target_features
                    .iter()
                    .all(|s| !s.trim().is_empty() && s.len() <= 128)
                && x.effort.is_none_or(|e| (1..=5).contains(&e))
                && x.practice_duration_ms
                    .is_none_or(|d| d <= 24 * 60 * 60 * 1000)
        }
        Fact::CoachJudgmentRecorded(x) => {
            !x.attempt.is_nil() && x.confidence_percent <= 100 && provenance(&x.provenance)
        }
        Fact::CoachAnnotationRecorded(x) => {
            !x.target.id().is_nil()
                && !x.note.trim().is_empty()
                && x.note.len() <= MAX_TEXT
                && x.suggested_correction
                    .as_ref()
                    .is_none_or(|s| s.len() <= MAX_TEXT)
                && x.span.is_none_or(|s| s.start < s.end && s.end <= MAX_TEXT)
                && provenance(&x.provenance)
        }
        Fact::TeachingDecisionRecorded(x) => {
            x.validate()?;
            true
        }
        Fact::AiAssessmentRecorded(x) => {
            x.validate()?;
            true
        }
        Fact::StimulusPresented(x) => {
            x.validate()?;
            true
        }
        Fact::SpeechTranscribed(x) => {
            x.validate()?;
            true
        }
        Fact::TranscriptConfirmed(x) => {
            x.validate()?;
            true
        }
        Fact::RecordingAttached(x) => {
            x.asset.validate()?;
            !x.target.id().is_nil()
                && !x.asset_id.trim().is_empty()
                && x.asset_id.len() <= 256
                && x.note.len() <= MAX_TEXT
                && !x.asset_id.chars().any(char::is_control)
                && (!matches!(x.target, AnnotationTarget::Attempt(_))
                    || x.role == AudioRole::Attempt)
        }
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(invalid("invalid learning context or coach fact"))
    }
}

pub(crate) fn validate_learning_references(
    event: &Event,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    if let Fact::StimulusPresented(presentation) = &event.fact {
        validate_presentation_references(presentation, events)?;
    }
    validate_speech_references(&event.fact, events)?;
    if let Fact::AiAssessmentRecorded(assessment) = &event.fact {
        validate_ai_references(assessment, events)?;
    }
    let attempt = match &event.fact {
        Fact::AttemptContextRecorded(x) => Some(x.attempt),
        Fact::CoachJudgmentRecorded(x) => Some(x.attempt),
        _ => None,
    };
    if let Some(id) = attempt {
        if events
            .get(&id)
            .is_some_and(|e| !matches!(e.fact, Fact::AttemptRecorded(_)))
        {
            return Err(invalid("assessment must reference an attempt"));
        }
    }
    if let Fact::CoachAnnotationRecorded(x) = &event.fact {
        if let Some(target) = events.get(&x.target.id()) {
            let text = match (&x.target, &target.fact) {
                (AnnotationTarget::Revision(_), Fact::PassageRevised(r)) => &r.passage.russian,
                (AnnotationTarget::Attempt(_), Fact::AttemptRecorded(a)) => &a.response,
                _ => return Err(invalid("annotation target type mismatch")),
            };
            if x.span.is_some_and(|s| {
                s.end > text.len()
                    || !text.is_char_boundary(s.start)
                    || !text.is_char_boundary(s.end)
            }) {
                return Err(invalid("annotation span must match exact UTF-8 target"));
            }
        }
    }
    if let Fact::RecordingAttached(x) = &event.fact {
        if let Some(target) = events.get(&x.target.id()) {
            if !matches!(
                (&x.target, &target.fact),
                (AnnotationTarget::Revision(_), Fact::PassageRevised(_))
                    | (AnnotationTarget::Attempt(_), Fact::AttemptRecorded(_))
            ) {
                return Err(invalid("recording target type mismatch"));
            }
        }
    }
    for parent in learning_parents(&event.fact) {
        if let Some(p) = events.get(&parent) {
            let same = match (&event.fact, &p.fact) {
                (Fact::AttemptContextRecorded(a), Fact::AttemptContextRecorded(b)) => {
                    a.attempt == b.attempt
                }
                (Fact::CoachJudgmentRecorded(a), Fact::CoachJudgmentRecorded(b)) => {
                    a.attempt == b.attempt && a.dimension == b.dimension
                }
                (Fact::CoachAnnotationRecorded(a), Fact::CoachAnnotationRecorded(b)) => {
                    a.target == b.target
                }
                (Fact::AiAssessmentRecorded(a), Fact::AiAssessmentRecorded(b)) => {
                    a.attempt == b.attempt && a.dimension == b.dimension
                }
                (Fact::TranscriptConfirmed(a), Fact::TranscriptConfirmed(b)) => {
                    a.transcription == b.transcription
                }
                _ => false,
            };
            if !same {
                return Err(invalid("assessment supersedes an unrelated fact"));
            }
        }
    }
    if let Fact::TeachingDecisionRecorded(d) = &event.fact {
        for candidate in &d.candidates {
            if events
                .get(&candidate.key.revision)
                .is_some_and(|e| !matches!(e.fact, Fact::PassageRevised(_)))
            {
                return Err(invalid("teaching candidate must reference a revision"));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_audio_identities(events: &BTreeMap<Uuid, Event>) -> Result<(), Error> {
    let mut ids = BTreeMap::new();
    for event in events.values() {
        if let Fact::RecordingAttached(x) = &event.fact {
            if ids
                .insert(&x.asset_id, &x.asset)
                .is_some_and(|old| old != &x.asset)
            {
                return Err(invalid("audio identity has conflicting metadata"));
            }
        }
    }
    Ok(())
}

impl Library {
    pub fn recordings(&self, target: AnnotationTarget) -> Vec<(&Event, &AudioAttachment)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::RecordingAttached(a) if a.target == target => Some((e, a)),
                _ => None,
            })
            .collect()
    }
    pub fn attempt(&self, id: Uuid) -> Option<&Attempt> {
        match &self.events.get(&id)?.fact {
            Fact::AttemptRecorded(a) => Some(a),
            _ => None,
        }
    }
    /// All active judgments, including disagreements. No clock-based winner.
    pub fn judgments(
        &self,
        attempt: Uuid,
        dimension: JudgmentDimension,
    ) -> Vec<(&Event, &CoachJudgment)> {
        let superseded: BTreeSet<_> = self
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::CoachJudgmentRecorded(j) => Some(j.supersedes.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::CoachJudgmentRecorded(j)
                    if j.attempt == attempt
                        && j.dimension == dimension
                        && !superseded.contains(&e.id) =>
                {
                    Some((e, j))
                }
                _ => None,
            })
            .collect()
    }
    pub fn contexts(&self, attempt: Uuid) -> Vec<(&Event, &AttemptContext)> {
        let superseded: BTreeSet<_> = self
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::AttemptContextRecorded(c) => Some(c.supersedes.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::AttemptContextRecorded(c)
                    if c.attempt == attempt && !superseded.contains(&e.id) =>
                {
                    Some((e, c))
                }
                _ => None,
            })
            .collect()
    }
    pub fn annotations(&self, target: AnnotationTarget) -> Vec<(&Event, &CoachAnnotation)> {
        let superseded: BTreeSet<_> = self
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::CoachAnnotationRecorded(c) => Some(c.supersedes.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::CoachAnnotationRecorded(c)
                    if c.target == target && !superseded.contains(&e.id) =>
                {
                    Some((e, c))
                }
                _ => None,
            })
            .collect()
    }
    pub fn decisions(&self, session: Uuid) -> Vec<(&Event, &TeachingDecision)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::TeachingDecisionRecorded(d) if d.session == session => Some((e, d)),
                _ => None,
            })
            .collect()
    }
}

/// Operational knowledge from immutable app observations; never a claim about outside experience.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StimulusNovelty {
    PreviouslyExposed {
        event: Uuid,
    },
    /// Legacy records do not describe every rendered/replayed stimulus.
    UnknownHistory,
    /// No prior exposure in the available, v3-observed app history; outside experience is unknown.
    NoPriorRecordedExposure,
}
/// Canonical token sequence for conservative exposure comparison. Orthographic decoration and
/// punctuation do not manufacture a new utterance; ё remains distinct from е.
pub fn stimulus_identity(russian: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for c in russian.chars() {
        if c == '\u{301}' {
            continue;
        }
        if c.is_alphanumeric() {
            if separator && !output.is_empty() {
                output.push(' ');
            }
            output.extend(c.to_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    output
}
/// A short candidate inside a longer observed dialogue was also encountered. Token boundaries
/// prevent 'он' inside 'корабль' (or another word fragment) from matching.
pub fn stimulus_contains(presented: &str, candidate: &str) -> bool {
    let presented = stimulus_identity(presented);
    let candidate = stimulus_identity(candidate);
    !candidate.is_empty() && format!(" {} ", presented).contains(&format!(" {} ", candidate))
}
impl Library {
    /// Completed practice is observable exposure; selection and asset attachment are not.
    /// Positive novelty requires the forthcoming presentation log, so absence remains unknown.
    pub fn stimulus_novelty(
        &self,
        revision: Uuid,
        before_ms: i64,
        excluding_attempt: Option<Uuid>,
    ) -> StimulusNovelty {
        self.presentation_novelty(revision, before_ms, excluding_attempt)
    }
    /// A unique completed attempt must match the recommendation's session, policy and exact key.
    /// This establishes completed practice, not proof that an explanation was actually displayed.
    pub fn completed_practice_for_decision(
        &self,
        decision_id: Uuid,
        now_ms: i64,
    ) -> Option<(&Event, &Attempt)> {
        let Fact::TeachingDecisionRecorded(decision) = &self.events.get(&decision_id)?.fact else {
            return None;
        };
        let mut completed = self
            .attempts()
            .filter(|(e, a)| a.session == decision.session && e.recorded_at_ms <= now_ms);
        let (event, attempt) = completed.next()?;
        if completed.next().is_some()
            || attempt.revision != decision.selected.revision
            || attempt.skill != decision.selected.skill
            || attempt.policy != decision.policy
        {
            return None;
        }
        Some((event, attempt))
    }
}
