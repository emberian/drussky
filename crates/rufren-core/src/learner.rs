//! Descriptive evidence grouped by feature, task, and skill. No exposure-to-mastery inference.
use crate::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct FeatureKey {
    pub feature: String,
    pub skill: Skill,
    pub task: TaskKind,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JudgmentTally {
    pub correct: usize,
    pub partly_correct: usize,
    pub incorrect: usize,
    pub uncertain: usize,
    pub conflicting: usize,
    pub delayed_novel_correct: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeatureEvidence {
    pub key: FeatureKey,
    pub observations: usize,
    pub independent_self_reports: usize,
    pub assisted_attempts: usize,
    /// Reserved for explicitly feature-linked judgments; whole-response tags never populate it.
    pub judgments: BTreeMap<JudgmentDimension, JudgmentTally>,
    /// Whole-response judgments observed on attempts tagged with this feature. These are not
    /// judgments that the individual feature was required, correct, learned, or independently used.
    #[serde(default)]
    pub attempt_judgments: BTreeMap<JudgmentDimension, JudgmentTally>,
    pub reported_effort_sum: u64,
    pub reported_effort_count: usize,
    pub practice_duration_ms: u64,
    pub duration_observations: usize,
    pub last_observed_ms: i64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LearnerSnapshot {
    pub features: Vec<FeatureEvidence>,
    pub unresolved_context_attempts: usize,
    pub unannotated_attempts: usize,
    pub ignored_future_attempts: usize,
}
impl Library {
    /// Explicit target tagging identifies which evidence to inspect, not proof that each tagged
    /// feature was successfully retrieved. One attempt can appear under several features; totals
    /// across rows must not be summed as unique practice time. Low-confidence judgments remain
    /// in the fact log but do not enter these thresholded descriptive tallies.
    pub fn learner_snapshot(&self, now_ms: i64) -> LearnerSnapshot {
        let mut snapshot = LearnerSnapshot::default();
        let mut features = BTreeMap::<FeatureKey, FeatureEvidence>::new();
        for (event, attempt) in self.attempts() {
            if event.recorded_at_ms > now_ms {
                snapshot.ignored_future_attempts += 1;
                continue;
            }
            let contexts = self.contexts(event.id);
            if contexts.is_empty() {
                snapshot.unannotated_attempts += 1;
                continue;
            }
            if contexts.len() != 1 {
                snapshot.unresolved_context_attempts += 1;
                continue;
            }
            let context = contexts[0].1;
            let assisted = !attempt.cues.is_empty() || attempt.self_rating == SelfRating::WithHelp;
            for feature in &context.target_features {
                let key = FeatureKey {
                    feature: feature.clone(),
                    skill: attempt.skill,
                    task: context.task,
                };
                let evidence = features
                    .entry(key.clone())
                    .or_insert_with(|| FeatureEvidence {
                        key,
                        observations: 0,
                        independent_self_reports: 0,
                        assisted_attempts: 0,
                        judgments: BTreeMap::new(),
                        attempt_judgments: BTreeMap::new(),
                        reported_effort_sum: 0,
                        reported_effort_count: 0,
                        practice_duration_ms: 0,
                        duration_observations: 0,
                        last_observed_ms: event.recorded_at_ms,
                    });
                evidence.observations += 1;
                evidence.assisted_attempts += usize::from(assisted);
                evidence.independent_self_reports +=
                    usize::from(!assisted && attempt.self_rating == SelfRating::Recalled);
                evidence.last_observed_ms = evidence.last_observed_ms.max(event.recorded_at_ms);
                if let Some(effort) = context.effort {
                    evidence.reported_effort_sum += u64::from(effort);
                    evidence.reported_effort_count += 1;
                }
                if let Some(duration) = context.practice_duration_ms {
                    evidence.practice_duration_ms =
                        evidence.practice_duration_ms.saturating_add(duration);
                    evidence.duration_observations += 1;
                }
                for dimension in [
                    JudgmentDimension::Recall,
                    JudgmentDimension::Comprehension,
                    JudgmentDimension::Grammar,
                    JudgmentDimension::Pronunciation,
                    JudgmentDimension::Fluency,
                    JudgmentDimension::Intelligibility,
                    JudgmentDimension::Spelling,
                ] {
                    let judgments = self.judgments(event.id, dimension);
                    let verdicts: Vec<_> = judgments
                        .iter()
                        .filter(|(_, j)| j.confidence_percent >= 70)
                        .map(|(_, j)| j.verdict)
                        .collect();
                    if verdicts.is_empty() {
                        continue;
                    }
                    let tally = evidence.attempt_judgments.entry(dimension).or_default();
                    if verdicts.iter().any(|v| *v != verdicts[0]) {
                        tally.conflicting += 1;
                        continue;
                    }
                    match verdicts[0] {
                        Verdict::Correct => {
                            tally.correct += 1;
                            // A novelty boolean and whole-response correctness establish no
                            // feature-specific delayed generalization. Actual presentation and
                            // direct feature judgment contracts are required for that inference.
                        }
                        Verdict::PartlyCorrect => tally.partly_correct += 1,
                        Verdict::Incorrect => tally.incorrect += 1,
                        Verdict::Uncertain => tally.uncertain += 1,
                    }
                }
            }
        }
        snapshot.features = features.into_values().collect();
        snapshot
    }
}
