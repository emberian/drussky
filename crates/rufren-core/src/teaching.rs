//! Small interpretable experimental teaching model. All numerical predictions are hypotheses.
use crate::*;

pub const CONTRAST_POLICY_ID: &str = "experimental-case-contrast-v2";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrammarResponse {
    CaseSensitive,
    FirstNounActor,
    Other,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeachingAction {
    ExplainAnchor,
    ContrastExamples,
    Retrieve,
    DelayedNovelProbe,
    AskCoach,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyCandidate {
    pub key: ReviewKey,
    pub action: TeachingAction,
    /// Relative policy utility, not a probability or a correctness score.
    pub score: i32,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LearningPrediction {
    pub feature: String,
    pub skill: Skill,
    pub minimum_delay_ms: i64,
    pub requires_novel_context: bool,
    pub judged_dimension: JudgmentDimension,
    /// Uncalibrated model forecast in basis points, persisted before observing outcomes.
    pub predicted_success_bps: u16,
    pub comparator_success_bps: u16,
    pub comparator_policy: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TeachingDecision {
    pub session: Uuid,
    pub policy: String,
    pub selected: ReviewKey,
    pub action: TeachingAction,
    pub candidates: Vec<PolicyCandidate>,
    pub assumptions: Vec<String>,
    pub predictions: Vec<LearningPrediction>,
    pub override_reason: Option<String>,
}
impl TeachingDecision {
    pub fn validate(&self) -> Result<(), Error> {
        let text = |s: &str| !s.trim().is_empty() && s.len() <= MAX_TEXT;
        if self.session.is_nil()
            || self.selected.revision.is_nil()
            || self.policy.is_empty()
            || self.policy.len() > 128
            || self.candidates.is_empty()
            || self.candidates.len() > 64
            || !self
                .candidates
                .iter()
                .any(|c| c.key == self.selected && c.action == self.action)
            || self
                .candidates
                .iter()
                .any(|c| c.key.revision.is_nil() || !text(&c.reason))
            || self.assumptions.is_empty()
            || self.assumptions.len() > 32
            || self.assumptions.iter().any(|s| !text(s))
            || self.predictions.len() > 32
            || self.predictions.iter().any(|p| {
                !text(&p.feature)
                    || p.feature.len() > 128
                    || !(MINUTE_MS..=365 * DAY_MS).contains(&p.minimum_delay_ms)
                    || p.predicted_success_bps > 10_000
                    || p.comparator_success_bps > 10_000
                    || !text(&p.comparator_policy)
                    || p.comparator_policy.len() > 128
            })
            || self.override_reason.as_ref().is_some_and(|s| !text(s))
        {
            return Err(Error::Invalid(
                "invalid teaching decision or prediction".into(),
            ));
        }
        Ok(())
    }
    pub fn override_with(
        &mut self,
        key: ReviewKey,
        action: TeachingAction,
        reason: String,
    ) -> Result<(), Error> {
        let mut updated = self.clone();
        updated.selected = key;
        updated.action = action;
        updated.override_reason = Some(reason);
        updated.validate()?;
        *self = updated;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GrammarBeliefs {
    pub case_sensitive_bps: u16,
    pub first_noun_actor_bps: u16,
    pub unmodeled_bps: u16,
    pub observations: usize,
}
impl Default for GrammarBeliefs {
    fn default() -> Self {
        Self {
            case_sensitive_bps: 3334,
            first_noun_actor_bps: 3333,
            unmodeled_bps: 3333,
            observations: 0,
        }
    }
}
impl GrammarBeliefs {
    pub fn observe(&mut self, response: GrammarResponse) {
        // Deliberately coarse likelihood table; not a fitted Russian learner model.
        let likelihood = match response {
            GrammarResponse::CaseSensitive => [80, 10, 20],
            GrammarResponse::FirstNounActor => [10, 80, 20],
            GrammarResponse::Other => [10, 10, 60],
        };
        let weights = [
            u64::from(self.case_sensitive_bps) * likelihood[0],
            u64::from(self.first_noun_actor_bps) * likelihood[1],
            u64::from(self.unmodeled_bps) * likelihood[2],
        ];
        let total = weights.iter().sum::<u64>().max(1);
        // A nonzero floor keeps alternative explanations recoverable after contradictory data.
        let a = (weights[0] * 9700 / total + 100) as u16;
        let b = (weights[1] * 9700 / total + 100) as u16;
        self.case_sensitive_bps = a;
        self.first_noun_actor_bps = b;
        self.unmodeled_bps = 10_000 - a - b;
        self.observations += 1;
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContrastDomain {
    pub feature: String,
    pub skill: Skill,
    pub anchor_revision: Uuid,
    pub contrast_revision: Uuid,
    pub novel_probe_revision: Uuid,
}
/// Item-specific competing predictions. This is an inspectable authoring hypothesis, not a
/// correctness label inferred from a generic self-report or target tag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GrammarDiagnostic {
    pub revision: Uuid,
    pub skill: Skill,
    pub feature: String,
    pub prompt: String,
    pub case_sensitive_response: String,
    pub first_noun_actor_response: String,
    pub source: String,
}
impl GrammarDiagnostic {
    pub fn validate(&self) -> Result<(), Error> {
        let bounded = |s: &str, n| !s.trim().is_empty() && s.len() <= n;
        if self.revision.is_nil()
            || !bounded(&self.feature, 128)
            || !bounded(&self.prompt, 8192)
            || !bounded(&self.case_sensitive_response, 2048)
            || !bounded(&self.first_noun_actor_response, 2048)
            || self.case_sensitive_response.trim() == self.first_noun_actor_response.trim()
            || !bounded(&self.source, 512)
        {
            return Err(Error::Invalid(
                "diagnostic item must name distinct competing responses to an exact question"
                    .into(),
            ));
        }
        Ok(())
    }
}
fn contrast_policy(id: &str) -> bool {
    id == CONTRAST_POLICY_ID || id == "experimental-case-contrast-v1"
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ContrastPolicy;
impl ContrastPolicy {
    /// Legacy response labels do not supply item-specific distinguishing predictions.
    /// Call beliefs_with_diagnostics when an explicit diagnostic contract is available.
    pub fn beliefs(
        &self,
        _library: &Library,
        _feature: &str,
        _skill: Skill,
        _now_ms: i64,
    ) -> GrammarBeliefs {
        GrammarBeliefs::default()
    }
    pub fn beliefs_with_diagnostics(
        &self,
        library: &Library,
        feature: &str,
        skill: Skill,
        now_ms: i64,
        diagnostics: &[GrammarDiagnostic],
    ) -> Result<GrammarBeliefs, Error> {
        if diagnostics.len() > 8 {
            return Err(Error::Invalid(
                "at most eight diagnostic contracts fit the durable decision budget".into(),
            ));
        }
        for item in diagnostics {
            item.validate()?;
        }
        // A completed practice following instruction starts a new learning epoch. It is not
        // evidence of a permanent learner trait, and an uncompleted recommendation does nothing.
        let cutoff = library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::TeachingDecisionRecorded(d)
                    if contrast_policy(&d.policy)
                        && matches!(
                            d.action,
                            TeachingAction::ExplainAnchor | TeachingAction::ContrastExamples
                        )
                        && d.predictions
                            .iter()
                            .any(|p| p.feature == feature && p.skill == skill) =>
                {
                    library.teaching_activity_time(e.id, now_ms)
                }
                _ => None,
            })
            .max();
        let mut attempts: Vec<_> = library
            .attempts()
            .filter(|(e, a)| {
                e.recorded_at_ms <= now_ms
                    && cutoff.is_none_or(|time| e.recorded_at_ms > time)
                    && a.skill == skill
                    && a.cues.is_empty()
                    && a.self_rating != SelfRating::WithHelp
            })
            .collect();
        attempts.sort_by_key(|(e, _)| (e.recorded_at_ms, e.id));
        let mut seen = BTreeSet::new();
        let mut beliefs = GrammarBeliefs::default();
        for (event, attempt) in attempts {
            let contexts = library.contexts(event.id);
            if contexts.len() != 1 {
                continue;
            }
            let context = contexts[0].1;
            if context.task != TaskKind::GrammarGeneralization
                || !context.target_features.contains(feature)
            {
                continue;
            }
            let Some(item) = diagnostics.iter().find(|d| {
                d.revision == attempt.revision
                    && d.skill == skill
                    && d.feature == feature
                    && context.prompt.as_deref() == Some(d.prompt.as_str())
            }) else {
                continue;
            };
            let Some(revision) = library.revision(attempt.revision) else {
                continue;
            };
            // Same utterance/revision edit cannot supply repeated independent diagnostic evidence.
            if !seen.insert(stimulus_identity(&revision.passage.russian)) {
                continue;
            }
            let response = if attempt.response == item.case_sensitive_response {
                GrammarResponse::CaseSensitive
            } else if attempt.response == item.first_noun_actor_response {
                GrammarResponse::FirstNounActor
            } else {
                GrammarResponse::Other
            };
            beliefs.observe(response);
        }
        Ok(beliefs)
    }
    pub fn plan(
        &self,
        library: &Library,
        domain: &ContrastDomain,
        now_ms: i64,
        session: Uuid,
    ) -> Result<TeachingDecision, Error> {
        self.plan_with_diagnostics(library, domain, now_ms, session, &[])
    }
    pub fn plan_with_diagnostics(
        &self,
        library: &Library,
        domain: &ContrastDomain,
        now_ms: i64,
        session: Uuid,
        diagnostics: &[GrammarDiagnostic],
    ) -> Result<TeachingDecision, Error> {
        if domain.feature.trim().is_empty()
            || domain.feature.len() > 128
            || domain.anchor_revision == domain.contrast_revision
            || domain.anchor_revision == domain.novel_probe_revision
            || domain.contrast_revision == domain.novel_probe_revision
        {
            return Err(Error::Invalid(
                "contrast domain needs a feature and three distinct revisions".into(),
            ));
        }
        let heads = library.heads();
        for id in [
            domain.anchor_revision,
            domain.contrast_revision,
            domain.novel_probe_revision,
        ] {
            if library.revision(id).is_none() || !heads.values().any(|h| h.len() == 1 && h[0] == id)
            {
                return Err(Error::Invalid("experimental contrast policy requires available unconflicted current revisions".into()));
            }
        }
        let beliefs = self.beliefs_with_diagnostics(
            library,
            &domain.feature,
            domain.skill,
            now_ms,
            diagnostics,
        )?;
        let completions: Vec<_> = library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::TeachingDecisionRecorded(d)
                    if contrast_policy(&d.policy)
                        && d.selected.skill == domain.skill
                        && [domain.anchor_revision, domain.contrast_revision]
                            .contains(&d.selected.revision) =>
                {
                    library
                        .teaching_activity_time(e.id, now_ms)
                        .map(|time| (time, d))
                }
                _ => None,
            })
            .collect();
        let last_learning = completions.iter().map(|(t, _)| *t).max();
        let anchor_done = completions
            .iter()
            .any(|(_, d)| d.selected.revision == domain.anchor_revision);
        let contrast_done = completions
            .iter()
            .any(|(_, d)| d.selected.revision == domain.contrast_revision);
        let last_probe = library
            .attempts()
            .filter(|(e, a)| {
                e.recorded_at_ms <= now_ms
                    && a.skill == domain.skill
                    && a.revision == domain.novel_probe_revision
            })
            .map(|(e, _)| e.recorded_at_ms)
            .max();
        let probe_due = last_learning.is_some_and(|time| {
            now_ms.saturating_sub(time) >= DAY_MS && last_probe.is_none_or(|probe| probe < time)
        });
        let key = |revision| ReviewKey {
            revision,
            skill: domain.skill,
        };
        let difficulty = beliefs.observations >= 2 && beliefs.unmodeled_bps >= 5000;
        let novelty = library.stimulus_novelty(domain.novel_probe_revision, now_ms, None);
        let probe_action = if novelty == StimulusNovelty::NoPriorRecordedExposure {
            TeachingAction::DelayedNovelProbe
        } else {
            TeachingAction::Retrieve
        };
        let candidates=vec![
            PolicyCandidate{key:key(domain.anchor_revision),action:TeachingAction::ExplainAnchor,
                score:if difficulty {160}else if !anchor_done {100}else{20},
                reason:if difficulty {"Ask the AI teacher to re-explain or change representation; the limited diagnostic model does not explain the answers."}else{"Establish a meaningful example with the AI teacher; human signoff is optional."}.into()},
            PolicyCandidate{key:key(domain.contrast_revision),action:TeachingAction::ContrastExamples,
                score:if anchor_done && !contrast_done {100}else{30+i32::from(beliefs.first_noun_actor_bps/100)},
                reason:"Use a deliberately separating contrast; an ordinary subject-first correct answer cannot distinguish the competing rules.".into()},
            PolicyCandidate{key:key(domain.anchor_revision),action:TeachingAction::Retrieve,
                score:if contrast_done {90}else{25+i32::from(beliefs.case_sensitive_bps/150)},
                reason:"Retrieve independently in a stable task; response evidence remains task- and item-specific.".into()},
            PolicyCandidate{key:key(domain.novel_probe_revision),action:probe_action,
                score:if probe_due {150}else{-100},
                reason:format!("Delayed probe anchored to completed practice, not recommendations. Novelty evidence: {:?}; use as retention practice when unseen status is unknown or previously exposed.",novelty)},
        ];
        let selected = candidates
            .iter()
            .enumerate()
            .max_by_key(|(i, c)| (c.score, std::cmp::Reverse(*i)))
            .unwrap()
            .1;
        let forecast = 5000 + beliefs.case_sensitive_bps / 4;
        let mut assumptions=vec![
            "Experimental comparison heuristic, not POMDP/lookahead, calibrated learner diagnosis, or an optimal curriculum. Fixed likelihoods and scores are engineering hypotheses.".into(),
            format!("Current learning-epoch weights: case={}, first-noun={}, unmodeled={}; {} distinct unassisted diagnostic items.",beliefs.case_sensitive_bps,beliefs.first_noun_actor_bps,beliefs.unmodeled_bps,beliefs.observations),
            "Observed instruction/answer presentation or completed matched practice anchors this policy. A recommendation, recording attachment, or unverified novelty flag is not exposure evidence.".into(),
            "Content quality/source is separate from human approval: AI teaching can proceed on draft material; examples and competing-rule predictions remain attributed authoring hypotheses.".into(),
            "Forecasts below are synthetic calibration targets, not measured probabilities or estimated learning gains. The comparator equals the forecast: no policy improvement is assumed.".into(),
        ];
        for item in diagnostics {
            assumptions.push(format!("Diagnostic source {}; revision {}; exact prompt {:?}; case predicts {:?}, first-noun predicts {:?}.",item.source,item.revision,item.prompt,item.case_sensitive_response,item.first_noun_actor_response));
        }
        let decision = TeachingDecision {
            session,
            policy: CONTRAST_POLICY_ID.into(),
            selected: selected.key,
            action: selected.action,
            candidates,
            assumptions,
            predictions: vec![LearningPrediction {
                feature: domain.feature.clone(),
                skill: domain.skill,
                minimum_delay_ms: DAY_MS,
                requires_novel_context: novelty == StimulusNovelty::NoPriorRecordedExposure,
                judged_dimension: JudgmentDimension::Grammar,
                predicted_success_bps: forecast,
                comparator_success_bps: forecast,
                comparator_policy: SPACED_POLICY_ID.into(),
            }],
            override_reason: None,
        };
        decision.validate()?;
        Ok(decision)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    Correct,
    Incorrect,
    PartlyCorrect,
    Uncertain,
    ConflictingJudgments,
    SelfReportOnly,
    Unjudged,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProbeObservation {
    pub attempt: Uuid,
    pub elapsed_ms: i64,
    pub outcome: ProbeOutcome,
    pub practice_duration_ms: Option<u64>,
    pub effort: Option<u8>,
}
impl Library {
    /// Eligible future observations for one persisted prediction. This is descriptive follow-up,
    /// not a causal estimate: transfer, other practice, and time trends can confound comparison.
    pub fn prediction_observations(
        &self,
        decision_id: Uuid,
        prediction_index: usize,
        now_ms: i64,
    ) -> Vec<ProbeObservation> {
        let Some(event) = self.events.get(&decision_id) else {
            return Vec::new();
        };
        let Fact::TeachingDecisionRecorded(decision) = &event.fact else {
            return Vec::new();
        };
        let Some(p) = decision.predictions.get(prediction_index) else {
            return Vec::new();
        };
        let Some(activity_time) = self.teaching_activity_time(decision_id, now_ms) else {
            return Vec::new();
        };
        let mut observations = Vec::new();
        for (attempt_event, a) in self.attempts() {
            let elapsed = attempt_event.recorded_at_ms.saturating_sub(activity_time);
            if attempt_event.recorded_at_ms > now_ms
                || elapsed < p.minimum_delay_ms
                || a.skill != p.skill
                || !a.cues.is_empty()
                || a.self_rating == SelfRating::WithHelp
            {
                continue;
            }
            let contexts = self.contexts(attempt_event.id);
            if contexts.len() != 1 {
                continue;
            }
            let c = contexts[0].1;
            if !c.delayed_probe
                || !c.target_features.contains(&p.feature)
                || (p.requires_novel_context
                    && self.stimulus_novelty(
                        a.revision,
                        attempt_event.recorded_at_ms,
                        Some(attempt_event.id),
                    ) != StimulusNovelty::NoPriorRecordedExposure)
            {
                continue;
            }
            let judgments = self.judgments(attempt_event.id, p.judged_dimension);
            let verdicts: Vec<_> = judgments
                .iter()
                .filter(|(_, j)| j.confidence_percent >= 70)
                .map(|(_, j)| j.verdict)
                .collect();
            let outcome = if verdicts.is_empty() {
                if a.self_rating == SelfRating::NotReported {
                    ProbeOutcome::Unjudged
                } else {
                    ProbeOutcome::SelfReportOnly
                }
            } else if verdicts.iter().any(|v| *v != verdicts[0]) {
                ProbeOutcome::ConflictingJudgments
            } else {
                match verdicts[0] {
                    Verdict::Correct => ProbeOutcome::Correct,
                    Verdict::Incorrect => ProbeOutcome::Incorrect,
                    Verdict::PartlyCorrect => ProbeOutcome::PartlyCorrect,
                    Verdict::Uncertain => ProbeOutcome::Uncertain,
                }
            };
            observations.push(ProbeObservation {
                attempt: attempt_event.id,
                elapsed_ms: elapsed,
                outcome,
                practice_duration_ms: c.practice_duration_ms,
                effort: c.effort,
            });
        }
        observations.sort_by_key(|o| (o.elapsed_ms, o.attempt));
        observations
    }
}

/// Shared seam for baseline and experimental planning; both return inspectable durable decisions.
#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub session: Uuid,
    pub now_ms: i64,
    pub skills: Vec<Skill>,
    pub contrast_domain: Option<ContrastDomain>,
}
pub trait TeachingPlanner {
    fn policy_id(&self) -> &'static str;
    fn decide(
        &self,
        library: &Library,
        context: &PolicyContext,
    ) -> Result<Option<TeachingDecision>, Error>;
}
impl TeachingPlanner for SpacedReview {
    fn policy_id(&self) -> &'static str {
        SPACED_POLICY_ID
    }
    fn decide(
        &self,
        library: &Library,
        context: &PolicyContext,
    ) -> Result<Option<TeachingDecision>, Error> {
        let queue = self.queue(library, context.now_ms, &context.skills);
        let Some(selected) = queue.iter().find(|entry| entry.due) else {
            return Ok(None);
        };
        let candidates = queue.iter().take(64).enumerate().map(|(i,e)| PolicyCandidate {
            key: e.state.key, action: TeachingAction::Retrieve, score: 64 - i as i32,
            reason: format!("{}; {} attempts; due at {} ms; exact skill and revision; {} self-reported independent recalls, {} coach-confirmed.",
                if e.state.is_new() { "New material" } else if e.due { "Due review" } else { "Scheduled later" },
                e.state.attempts,e.state.due_at_ms,e.state.unassisted_self_reports,e.state.coach_confirmed_unassisted),
        }).collect();
        let decision = TeachingDecision { session: context.session, policy: self.id().into(),
            selected: selected.state.key, action: TeachingAction::Retrieve, candidates,
            assumptions: vec![self.provenance().into(), "Local event times order practice; device clock errors may affect due dates. Future-dated attempts are ignored until their timestamp arrives.".into()],
            predictions: Vec::new(), override_reason: None };
        decision.validate()?;
        Ok(Some(decision))
    }
}
impl TeachingPlanner for ContrastPolicy {
    fn policy_id(&self) -> &'static str {
        CONTRAST_POLICY_ID
    }
    fn decide(
        &self,
        library: &Library,
        context: &PolicyContext,
    ) -> Result<Option<TeachingDecision>, Error> {
        let Some(domain) = &context.contrast_domain else {
            return Ok(None);
        };
        if !context.skills.contains(&domain.skill) {
            return Ok(None);
        }
        self.plan(library, domain, context.now_ms, context.session)
            .map(Some)
    }
}
