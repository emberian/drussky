//! Learner timing choices and prospective start records. These are not memory observations.
use crate::*;
use sha2::{Digest, Sha256};

pub type ReviewTarget = PracticeBatchKey;
pub const REVIEW_TIMING_POLICY_ID: &str = "learner-review-timing-v1";
pub const MAX_REVIEW_EVIDENCE_IDS: usize = 512;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewCadence {
    MoreOften,
    #[default]
    Standard,
    MoreSpace,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewCadenceChange {
    pub parents: BTreeSet<Uuid>,
    pub cadence: ReviewCadence,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReviewTimingAction {
    Pause,
    Resume,
    AvailableAt { at_ms: i64 },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewTimingChange {
    pub target: ReviewTarget,
    pub parents: BTreeSet<Uuid>,
    pub action: ReviewTimingAction,
}
/// Written before retrieval begins; later support or preference changes never rewrite it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewStartDecision {
    pub target: ReviewTarget,
    pub session: Uuid,
    pub policy: String,
    pub as_of_ms: i64,
    pub started_at_ms: i64,
    pub cadence: ReviewCadence,
    pub cadence_heads: BTreeSet<Uuid>,
    pub cadence_fallback_reason: Option<String>,
    pub baseline_due_at_ms: i64,
    pub effective_due_at_ms: Option<i64>,
    pub timing_paused: bool,
    pub timing_pending: bool,
    pub timing_heads: BTreeSet<Uuid>,
    pub reason: String,
    /// First <=512 sorted source IDs; omitted count is total minus sample length.
    /// The full-set digest identifies the issued timing inputs; this bounded audit is not
    /// a complete experimental-model replay snapshot.
    pub evidence_ids: BTreeSet<Uuid>,
    pub evidence_total_count: usize,
    pub evidence_ids_sha256: String,
}
/// SHA-256 of sorted unique UUIDs concatenated as raw 16-byte UUID bytes.
pub fn review_evidence_ids_sha256(ids: &BTreeSet<Uuid>) -> String {
    let mut hash = Sha256::new();
    for id in ids {
        hash.update(id.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewCadenceState {
    pub cadence: ReviewCadence,
    pub heads: BTreeSet<Uuid>,
    pub conflicted: bool,
    pub pending: bool,
    pub fallback_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewTimingState {
    pub target: ReviewTarget,
    pub baseline_due_at_ms: i64,
    pub effective_due_at_ms: Option<i64>,
    pub heads: BTreeSet<Uuid>,
    pub paused: bool,
    pub pending: bool,
    pub conflicted: bool,
}
impl ReviewTimingState {
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.effective_due_at_ms.is_some_and(|t| t <= now_ms)
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ReviewCadencePolicy;
impl ReviewCadencePolicy {
    pub fn project(&self, library: &Library) -> ReviewCadenceState {
        let changes: BTreeMap<_, _> = library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::ReviewCadenceChanged(c) => Some((e.id, c)),
                _ => None,
            })
            .collect();
        let parents: BTreeSet<_> = changes
            .values()
            .flat_map(|c| c.parents.iter().copied())
            .collect();
        let heads: BTreeSet<_> = changes
            .keys()
            .filter(|id| !parents.contains(*id))
            .copied()
            .collect();
        let pending = parents.iter().any(|p| !changes.contains_key(p));
        let first = heads
            .iter()
            .next()
            .map(|id| changes[id].cadence)
            .unwrap_or_default();
        let different = heads.iter().any(|id| changes[id].cadence != first);
        ReviewCadenceState {
            cadence: if pending || different {
                ReviewCadence::Standard
            } else {
                first
            },
            conflicted: heads.len() > 1,
            pending,
            fallback_reason: if pending {
                Some("incomplete preferences; standard fallback".into())
            } else if different {
                Some("conflicting preferences; standard fallback".into())
            } else {
                None
            },
            heads,
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ReviewTimingPolicy;
impl ReviewTimingPolicy {
    /// Completion IDs must be verified by the caller against exact task proofs. We additionally
    /// require a saved matching attempt. Opening or abandoning a draft consumes nothing.
    /// Event wall clocks never select intent winners; all known causal heads participate.
    pub fn project(
        &self,
        library: &Library,
        target: ReviewTarget,
        baseline_due_at_ms: i64,
        _now_ms: i64,
        completed_sessions: &BTreeSet<Uuid>,
    ) -> ReviewTimingState {
        let changes: BTreeMap<_, _> = library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::ReviewTimingChanged(c) if c.target == target => Some((e.id, c)),
                _ => None,
            })
            .collect();
        let parents: BTreeSet<_> = changes
            .values()
            .flat_map(|c| c.parents.iter().copied())
            .collect();
        let heads: BTreeSet<_> = changes
            .keys()
            .filter(|id| !parents.contains(*id))
            .copied()
            .collect();
        let pending = parents.iter().any(|p| !changes.contains_key(p));
        let consumed: BTreeSet<_> = library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::ReviewStarted(s)
                    if s.target == target
                        && completed_sessions.contains(&s.session)
                        && library.attempts().any(|(e, a)| {
                            a.session == s.session
                                && e.recorded_at_ms >= s.started_at_ms
                                && attempt_matches_target(library, a, target)
                        }) =>
                {
                    Some(s.timing_heads.iter().copied())
                }
                _ => None,
            })
            .flatten()
            .collect();
        let paused = heads
            .iter()
            .any(|id| matches!(changes[id].action, ReviewTimingAction::Pause));
        let override_due = heads
            .iter()
            .filter(|id| !consumed.contains(*id))
            .filter_map(|id| match changes[id].action {
                ReviewTimingAction::AvailableAt { at_ms } => Some(at_ms),
                _ => None,
            })
            .max();
        ReviewTimingState {
            target,
            baseline_due_at_ms,
            effective_due_at_ms: if paused || pending {
                None
            } else {
                Some(override_due.unwrap_or(baseline_due_at_ms))
            },
            conflicted: heads.len() > 1,
            heads,
            paused,
            pending,
        }
    }
}
fn attempt_matches_target(library: &Library, a: &Attempt, target: ReviewTarget) -> bool {
    match target {
        ReviewTarget::Passage { revision, skill } => a.revision == revision && a.skill == skill,
        ReviewTarget::Teacher {
            document_id,
            practice_index,
        } => library
            .teacher_document(document_id)
            .and_then(|d| teacher_practice_reference_fact(&d.response, practice_index).ok())
            .is_some_and(|(_, revision, _)| a.revision == revision),
    }
}
impl Library {
    pub fn review_start_for_session(
        &self,
        session: Uuid,
        target: ReviewTarget,
    ) -> Option<&ReviewStartDecision> {
        self.events.values().find_map(|e| match &e.fact {
            Fact::ReviewStarted(s) if s.session == session && s.target == target => Some(s),
            _ => None,
        })
    }
}
pub(crate) fn review_timing_parents(fact: &Fact) -> BTreeSet<Uuid> {
    match fact {
        Fact::ReviewTimingChanged(c) => c.parents.clone(),
        Fact::ReviewCadenceChanged(c) => c.parents.clone(),
        _ => BTreeSet::new(),
    }
}
pub(crate) fn validate_review_timing_fact(event: &Event) -> Result<(), Error> {
    let invalid = || Error::Invalid("invalid review timing fact".into());
    let ids = |v: &BTreeSet<Uuid>, cap| {
        v.len() <= cap && !v.contains(&event.id) && !v.iter().any(Uuid::is_nil)
    };
    if !ids(&review_timing_parents(&event.fact), 64) {
        return Err(invalid());
    }
    match &event.fact {
        Fact::ReviewCadenceChanged(_) => {}
        Fact::ReviewTimingChanged(c) => {
            c.target.validate()?;
        }
        Fact::ReviewStarted(s) => {
            s.target.validate()?;
            if s.session.is_nil()
                || s.policy.trim().is_empty()
                || s.policy.len() > 128
                || s.as_of_ms > s.started_at_ms
                || s.reason.trim().is_empty()
                || s.reason.len() > 2048
                || s.cadence_fallback_reason
                    .as_ref()
                    .is_some_and(|x| x.trim().is_empty() || x.len() > 256)
                || !ids(&s.cadence_heads, 64)
                || !ids(&s.timing_heads, 64)
                || !ids(&s.evidence_ids, MAX_REVIEW_EVIDENCE_IDS)
                || s.evidence_ids.len() != s.evidence_total_count.min(MAX_REVIEW_EVIDENCE_IDS)
                || s.evidence_ids_sha256.len() != 64
                || !s
                    .evidence_ids_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || (s.evidence_total_count <= MAX_REVIEW_EVIDENCE_IDS
                    && s.evidence_ids_sha256 != review_evidence_ids_sha256(&s.evidence_ids))
                || s.effective_due_at_ms.is_none() != (s.timing_paused || s.timing_pending)
            {
                return Err(invalid());
            }
        }
        _ => {}
    }
    Ok(())
}
pub(crate) fn validate_review_timing_references(
    event: &Event,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    let invalid = || Error::Invalid("review timing reference mismatch".into());
    let target = match &event.fact {
        Fact::ReviewTimingChanged(c) => Some(c.target),
        Fact::ReviewStarted(s) => Some(s.target),
        _ => None,
    };
    if let Some(target) = target {
        match target {
            ReviewTarget::Passage { revision, .. } => {
                if let Some(e) = events.get(&revision) {
                    if !matches!(e.fact, Fact::PassageRevised(_))
                        || is_reserved_practice_reference(e)
                    {
                        return Err(invalid());
                    }
                }
            }
            ReviewTarget::Teacher {
                document_id,
                practice_index,
            } => {
                for e in events.values() {
                    if let Fact::TeacherDocumentRecorded(d) = &e.fact {
                        if d.document_id == document_id
                            && teacher_practice_reference_fact(&d.response, practice_index).is_err()
                        {
                            return Err(invalid());
                        }
                    }
                }
            }
        }
    }
    for id in review_timing_parents(&event.fact) {
        if let Some(parent) = events.get(&id) {
            let same = match (&event.fact, &parent.fact) {
                (Fact::ReviewCadenceChanged(_), Fact::ReviewCadenceChanged(_)) => true,
                (Fact::ReviewTimingChanged(a), Fact::ReviewTimingChanged(b)) => {
                    a.target == b.target
                }
                _ => false,
            };
            if !same {
                return Err(invalid());
            }
        }
    }
    if let Fact::ReviewStarted(s) = &event.fact {
        for id in &s.cadence_heads {
            if events
                .get(id)
                .is_some_and(|e| !matches!(e.fact, Fact::ReviewCadenceChanged(_)))
            {
                return Err(invalid());
            }
        }
        for id in &s.timing_heads {
            if events.get(id).is_some_and(
                |e| !matches!(&e.fact,Fact::ReviewTimingChanged(c) if c.target == s.target),
            ) {
                return Err(invalid());
            }
        }
        // Resolve the recorded heads themselves, never today's superseding preference heads.
        if s.cadence_heads.iter().all(|id| events.contains_key(id)) {
            let mut values = Vec::new();
            let mut pending = false;
            let mut stack: Vec<_> = s.cadence_heads.iter().copied().collect();
            let mut seen = BTreeSet::new();
            while let Some(id) = stack.pop() {
                if !seen.insert(id) {
                    continue;
                }
                match events.get(&id).map(|e| &e.fact) {
                    Some(Fact::ReviewCadenceChanged(c)) => {
                        if s.cadence_heads.contains(&id) {
                            values.push(c.cadence);
                        }
                        stack.extend(c.parents.iter().copied());
                    }
                    None => pending = true,
                    _ => return Err(invalid()),
                }
            }
            let first = values.first().copied().unwrap_or_default();
            let different = values.iter().any(|c| *c != first);
            let expected = if pending || different {
                ReviewCadence::Standard
            } else {
                first
            };
            let reason = if pending {
                Some("incomplete preferences; standard fallback")
            } else if different {
                Some("conflicting preferences; standard fallback")
            } else {
                None
            };
            // A missing ancestor may sync after this start. The issued incomplete-input
            // fallback remains Standard rather than being retrospectively invalidated.
            let issued_incomplete = s.cadence == ReviewCadence::Standard
                && s.cadence_fallback_reason.as_deref()
                    == Some("incomplete preferences; standard fallback");
            if !issued_incomplete
                && (s.cadence != expected || s.cadence_fallback_reason.as_deref() != reason)
            {
                return Err(invalid());
            }
        }
        for other in events.values() {
            if let Fact::ReviewStarted(o) = &other.fact {
                if other.id != event.id && o.session == s.session && o != s {
                    return Err(Error::Invalid(
                        "conflicting immutable review starts for one session".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}
