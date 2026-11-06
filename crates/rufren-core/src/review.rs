//! An inspectable spacing comparator, not a fitted memory model or competence estimate.
use crate::*;

pub const MINUTE_MS: i64 = 60_000;
pub const DAY_MS: i64 = 24 * 60 * MINUTE_MS;
pub const SPACED_POLICY_ID: &str = "bounded-spaced-self-report-v1";
const STEPS: [i64; 8] = [
    10 * MINUTE_MS,
    DAY_MS,
    3 * DAY_MS,
    7 * DAY_MS,
    14 * DAY_MS,
    30 * DAY_MS,
    60 * DAY_MS,
    120 * DAY_MS,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ReviewKey {
    pub revision: Uuid,
    pub skill: Skill,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewEvidence {
    NotReported,
    UnassistedSelfReport,
    CoachConfirmedUnassisted,
    Assisted,
    Again,
    CoachCorrection,
    ContestedJudgment,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewState {
    pub key: ReviewKey,
    pub policy: String,
    pub attempts: usize,
    pub unassisted_self_reports: usize,
    pub coach_confirmed_unassisted: usize,
    pub assisted_attempts: usize,
    pub ignored_future_attempts: usize,
    pub last_attempt_ms: Option<i64>,
    pub last_evidence: Option<ReviewEvidence>,
    pub interval_ms: i64,
    pub due_at_ms: i64,
    pub successful_spaced_reviews: usize,
}
impl ReviewState {
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.due_at_ms <= now_ms
    }
    pub fn is_new(&self) -> bool {
        self.attempts == 0
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewEntry {
    pub state: ReviewState,
    pub due: bool,
}

/// Fixed intervals bounded between ten minutes and 120 days. No retention probability is claimed.
/// Times order practice observations only; they never choose content or assessment winners.
/// Future-dated attempts are excluded until their time arrives and reported for inspection.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpacedReview;
impl SpacedReview {
    pub fn id(&self) -> &'static str {
        SPACED_POLICY_ID
    }
    pub fn provenance(&self) -> &'static str {
        "Engineering baseline: fixed bounded intervals, separate exact revision and skill; self-reports are not correctness. Motivated by spacing/retrieval evidence E1–E2 and O1; these intervals and adaptation rules have not been validated for Rufren."
    }
    pub fn project(&self, library: &Library, key: ReviewKey, now_ms: i64) -> ReviewState {
        let mut state = ReviewState {
            key,
            policy: self.id().into(),
            attempts: 0,
            unassisted_self_reports: 0,
            coach_confirmed_unassisted: 0,
            assisted_attempts: 0,
            ignored_future_attempts: 0,
            last_attempt_ms: None,
            last_evidence: None,
            interval_ms: 0,
            due_at_ms: now_ms,
            successful_spaced_reviews: 0,
        };
        let mut observations: Vec<_> = library
            .attempts()
            .filter(|(_, a)| a.revision == key.revision && a.skill == key.skill)
            .collect();
        observations.sort_by_key(|(e, _)| (e.recorded_at_ms, e.id));
        let mut level = 0usize;
        let mut scheduled_from = None;
        for (event, attempt) in observations {
            if event.recorded_at_ms > now_ms {
                state.ignored_future_attempts += 1;
                continue;
            }
            let evidence = self.evidence(library, event.id, attempt);
            let first = state.interval_ms == 0;
            state.attempts += 1;
            state.last_attempt_ms = Some(event.recorded_at_ms);
            state.last_evidence = Some(evidence);
            let success = matches!(
                evidence,
                ReviewEvidence::UnassistedSelfReport | ReviewEvidence::CoachConfirmedUnassisted
            );
            if attempt.self_rating == SelfRating::Recalled && attempt.cues.is_empty() {
                state.unassisted_self_reports += 1;
            }
            if evidence == ReviewEvidence::CoachConfirmedUnassisted {
                state.coach_confirmed_unassisted += 1;
            }
            if !attempt.cues.is_empty() || attempt.self_rating == SelfRating::WithHelp {
                state.assisted_attempts += 1;
            }
            if evidence == ReviewEvidence::NotReported {
                continue;
            }
            if success {
                if first {
                    level = 1;
                    state.interval_ms = STEPS[level];
                    state.due_at_ms = event.recorded_at_ms.saturating_add(state.interval_ms);
                    scheduled_from = Some(event.recorded_at_ms);
                } else if event.recorded_at_ms >= state.due_at_ms
                    && scheduled_from.is_none_or(|time| event.recorded_at_ms > time)
                {
                    // Only retrieval at/after the scheduled delay advances the ladder.
                    level = (level + 1).min(STEPS.len() - 1);
                    state.successful_spaced_reviews += 1;
                    state.interval_ms = STEPS[level];
                    state.due_at_ms = event.recorded_at_ms.saturating_add(state.interval_ms);
                    scheduled_from = Some(event.recorded_at_ms);
                }
                // Early repeated success does not postpone an already scheduled review.
            } else {
                level = 0;
                state.interval_ms = if evidence == ReviewEvidence::Assisted {
                    30 * MINUTE_MS
                } else {
                    STEPS[0]
                };
                let retry = event.recorded_at_ms.saturating_add(state.interval_ms);
                // Repeated immediate attempts cannot push a retry endlessly into the future.
                state.due_at_ms = if first || event.recorded_at_ms >= state.due_at_ms {
                    scheduled_from = Some(event.recorded_at_ms);
                    retry
                } else {
                    state.due_at_ms.min(retry)
                };
            }
        }
        state
    }
    pub fn evidence(
        &self,
        library: &Library,
        attempt_id: Uuid,
        attempt: &Attempt,
    ) -> ReviewEvidence {
        if attempt.self_rating == SelfRating::NotReported {
            return ReviewEvidence::NotReported;
        }
        let judgments = library.judgments(attempt_id, JudgmentDimension::Recall);
        let trusted: Vec<_> = judgments
            .iter()
            .filter(|(_, j)| j.confidence_percent >= 70)
            .map(|(_, j)| j.verdict)
            .collect();
        let correct = trusted.contains(&Verdict::Correct);
        let adverse = trusted
            .iter()
            .any(|v| matches!(v, Verdict::Incorrect | Verdict::PartlyCorrect));
        if correct && adverse {
            return ReviewEvidence::ContestedJudgment;
        }
        if adverse {
            return ReviewEvidence::CoachCorrection;
        }
        if attempt.self_rating == SelfRating::Again {
            return ReviewEvidence::Again;
        }
        if !attempt.cues.is_empty() || attempt.self_rating == SelfRating::WithHelp {
            return ReviewEvidence::Assisted;
        }
        if correct {
            ReviewEvidence::CoachConfirmedUnassisted
        } else {
            ReviewEvidence::UnassistedSelfReport
        }
    }
    /// Unique active revisions only. Conflicted passages need explicit resolution before selection.
    /// Skill order breaks ties so a new library does not accidentally begin with UUID ordering.
    pub fn queue(&self, library: &Library, now_ms: i64, skills: &[Skill]) -> Vec<ReviewEntry> {
        let mut entries = Vec::new();
        for heads in library.heads().values().filter(|heads| heads.len() == 1) {
            // Teacher tasks retain their exact question and skill in their own revisit queue.
            // A generic passage review would silently substitute a different task.
            if library
                .events
                .get(&heads[0])
                .is_some_and(|e| is_teacher_generated_reference(e) || is_sound_focus_reference(e))
            {
                continue;
            }
            for skill in skills.iter().copied().collect::<BTreeSet<_>>() {
                let state = self.project(
                    library,
                    ReviewKey {
                        revision: heads[0],
                        skill,
                    },
                    now_ms,
                );
                entries.push(ReviewEntry {
                    due: state.is_due(now_ms),
                    state,
                });
            }
        }
        entries.sort_by_key(|e| {
            (
                !e.due,
                e.state.due_at_ms,
                e.state.attempts,
                skills
                    .iter()
                    .position(|s| *s == e.state.key.skill)
                    .unwrap_or(usize::MAX),
                e.state.key.revision,
            )
        });
        entries
    }
    pub fn select(&self, library: &Library, now_ms: i64, skills: &[Skill]) -> Option<ReviewEntry> {
        self.queue(library, now_ms, skills)
            .into_iter()
            .find(|entry| entry.due)
    }
}
