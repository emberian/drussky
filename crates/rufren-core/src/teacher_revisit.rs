//! Exact-task practice cadence, separate from generic passage review and proficiency estimates.
//! Inputs must already be matched by the application to the immutable document/question/audio.
use crate::*;
use rufren_tutor_contract::{ReviewStatus, TeacherRequest, TeacherResponse, TeacherSkill};

pub const TEACHER_REVISIT_POLICY_ID: &str = "teacher-delayed-revisit-v1";
const RETURN_STEPS: [i64; 7] = [
    DAY_MS,
    3 * DAY_MS,
    7 * DAY_MS,
    14 * DAY_MS,
    30 * DAY_MS,
    60 * DAY_MS,
    120 * DAY_MS,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeacherRevisitKey {
    pub document_id: Uuid,
    pub practice_index: usize,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeacherRevisitSeed {
    pub attempt_id: Uuid,
    pub context_id: Uuid,
    pub occurred_at_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeacherRevisitAiJudgment {
    pub fact_id: Uuid,
    pub recorded_at_ms: i64,
    pub verdict: Verdict,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeacherRevisitObservation {
    pub attempt_id: Uuid,
    pub context_id: Uuid,
    /// First actual retrieval input/start from the immutable task proof, not submission time.
    pub started_at_ms: i64,
    pub occurred_at_ms: i64,
    pub self_rating: SelfRating,
    pub cues: Vec<Cue>,
    /// Active, exact-task language-content opinions only; preserve disagreements.
    pub ai_judgments: Vec<TeacherRevisitAiJudgment>,
    /// Proven matching target exposure before this task began; required task input is excluded.
    pub prior_target_exposure_at_ms: Option<i64>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeacherRevisitEvidence {
    GuidedSeed,
    UnassistedSelfReport,
    AiCorrect,
    /// Learner Again or an adverse exact-task AI opinion; not an objective-error diagnosis.
    ClearError,
    AnswerHelp,
    ReplayOnly,
    Unrated,
    ContestedAi,
    /// A recent actual practice or prior target exposure prevents a full-delay claim.
    RecentRestudy,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeacherRevisitState {
    pub key: TeacherRevisitKey,
    pub policy: String,
    pub seed: TeacherRevisitSeed,
    pub due_at_ms: i64,
    pub interval_ms: i64,
    pub rung: usize,
    pub completed_returns: usize,
    pub self_report_advances: usize,
    pub ai_correct_advances: usize,
    pub clear_error_returns: usize,
    pub answer_help_returns: usize,
    pub replay_only_returns: usize,
    pub unrated_returns: usize,
    pub contested_returns: usize,
    pub recent_restudy_returns: usize,
    pub ignored_early_returns: usize,
    pub early_error_rechecks: usize,
    pub ignored_future_attempts: usize,
    pub ignored_future_judgments: usize,
    /// Stable representative of the last co-timed group, never a content/evidence winner.
    pub last_attempt: Option<Uuid>,
    pub last_evidence: TeacherRevisitEvidence,
}
impl TeacherRevisitState {
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.due_at_ms <= now_ms
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TeacherRevisitPolicy;
impl TeacherRevisitPolicy {
    pub fn id(&self) -> &'static str {
        TEACHER_REVISIT_POLICY_ID
    }
    pub fn provenance(&self) -> &'static str {
        "Engineering task-cadence baseline: exact completed guided practice seeds one day; delayed uncued self-recall or exact-task AI Correct can advance bounded fixed intervals. AI opinion, recall self-report, assistance and unrated completion remain distinct. Neither intervals nor model-correct scheduling are validated memory or mastery estimates."
    }
    /// Recompute from immutable occurrences as of `now_ms`. A late visible opinion belongs to
    /// its original attempt, never a new practice occurrence; due dates anchor to attempt time,
    /// not feedback arrival. New evidence can revise the projection, without changing facts.
    pub fn project(
        &self,
        key: TeacherRevisitKey,
        seed: TeacherRevisitSeed,
        observations: &[TeacherRevisitObservation],
        now_ms: i64,
    ) -> Result<TeacherRevisitState, Error> {
        self.project_with_cadence(key, seed, observations, now_ms, &BTreeMap::new())
    }
    /// Cadence is pinned at each actual start. Missing entries preserve legacy baseline timing.
    pub fn project_with_cadence(
        &self,
        key: TeacherRevisitKey,
        seed: TeacherRevisitSeed,
        observations: &[TeacherRevisitObservation],
        now_ms: i64,
        cadence: &BTreeMap<Uuid, ReviewCadence>,
    ) -> Result<TeacherRevisitState, Error> {
        let interval = |attempt, baseline| {
            cadence
                .get(&attempt)
                .copied()
                .unwrap_or_default()
                .scale_interval_ms(baseline)
        };
        let invalid = || Error::Invalid("invalid exact-task revisit observations".into());
        if key.document_id.is_nil()
            || key.practice_index >= 4
            || seed.attempt_id.is_nil()
            || seed.context_id.is_nil()
            || seed.occurred_at_ms > now_ms
        {
            return Err(invalid());
        }
        let mut unique = BTreeMap::<Uuid, &TeacherRevisitObservation>::new();
        let mut judgment_owners = BTreeMap::new();
        for o in observations {
            if o.attempt_id.is_nil()
                || o.context_id.is_nil()
                || o.attempt_id == seed.attempt_id
                || o.started_at_ms > o.occurred_at_ms
                || o.cues.len() > 32
                || o.ai_judgments.len() > 64
                || o.prior_target_exposure_at_ms
                    .is_some_and(|t| t > o.started_at_ms)
            {
                return Err(invalid());
            }
            if unique
                .insert(o.attempt_id, o)
                .is_some_and(|prior| prior != o)
            {
                return Err(invalid());
            }
            let mut opinions = BTreeMap::new();
            for j in &o.ai_judgments {
                if j.fact_id.is_nil()
                    || opinions.insert(j.fact_id, j).is_some_and(|p| p != j)
                    || judgment_owners
                        .insert(j.fact_id, o.attempt_id)
                        .is_some_and(|a| a != o.attempt_id)
                {
                    return Err(invalid());
                }
            }
        }
        let mut groups = BTreeMap::<i64, Vec<&TeacherRevisitObservation>>::new();
        for observation in unique.into_values() {
            groups
                .entry(observation.occurred_at_ms)
                .or_default()
                .push(observation);
        }
        let mut s = TeacherRevisitState {
            key,
            policy: self.id().into(),
            seed,
            due_at_ms: seed
                .occurred_at_ms
                .saturating_add(interval(seed.attempt_id, DAY_MS)),
            interval_ms: interval(seed.attempt_id, DAY_MS),
            rung: 0,
            completed_returns: 0,
            self_report_advances: 0,
            ai_correct_advances: 0,
            clear_error_returns: 0,
            answer_help_returns: 0,
            replay_only_returns: 0,
            unrated_returns: 0,
            contested_returns: 0,
            recent_restudy_returns: 0,
            ignored_early_returns: 0,
            early_error_rechecks: 0,
            ignored_future_attempts: 0,
            ignored_future_judgments: 0,
            last_attempt: None,
            last_evidence: TeacherRevisitEvidence::GuidedSeed,
        };
        let mut previous_practice = seed.occurred_at_ms;
        for (time, group) in groups {
            if time > now_ms {
                s.ignored_future_attempts += group.len();
                continue;
            }
            // Simultaneous starts cannot establish causal order or select a favorable profile.
            let group_interval = |baseline| {
                group
                    .iter()
                    .map(|o| interval(o.attempt_id, baseline))
                    .min()
                    .unwrap()
            };
            let mut classified = Vec::new();
            for o in &group {
                let mut seen = BTreeSet::new();
                let mut verdicts = BTreeSet::new();
                for j in &o.ai_judgments {
                    if !seen.insert(j.fact_id) {
                        continue;
                    }
                    if j.recorded_at_ms > now_ms {
                        s.ignored_future_judgments += 1;
                        continue;
                    }
                    verdicts.insert(match j.verdict {
                        Verdict::Correct => 0,
                        Verdict::PartlyCorrect => 1,
                        Verdict::Incorrect => 2,
                        Verdict::Uncertain => 3,
                    });
                }
                let correct = verdicts.contains(&0);
                let adverse = verdicts.contains(&1) || verdicts.contains(&2);
                let mut evidence = if correct && verdicts.len() > 1 {
                    TeacherRevisitEvidence::ContestedAi
                } else if o.self_rating == SelfRating::Again || adverse {
                    TeacherRevisitEvidence::ClearError
                } else if o.self_rating == SelfRating::WithHelp
                    || o.cues.iter().any(|c| *c != Cue::Replay)
                {
                    TeacherRevisitEvidence::AnswerHelp
                } else if !o.cues.is_empty() {
                    TeacherRevisitEvidence::ReplayOnly
                } else if correct {
                    TeacherRevisitEvidence::AiCorrect
                } else if o.self_rating == SelfRating::Recalled {
                    TeacherRevisitEvidence::UnassistedSelfReport
                } else {
                    TeacherRevisitEvidence::Unrated
                };
                let delay_anchor = previous_practice
                    .max(o.prior_target_exposure_at_ms.unwrap_or(previous_practice));
                if matches!(
                    evidence,
                    TeacherRevisitEvidence::AiCorrect
                        | TeacherRevisitEvidence::UnassistedSelfReport
                ) && o.started_at_ms.saturating_sub(delay_anchor) < s.interval_ms
                {
                    evidence = TeacherRevisitEvidence::RecentRestudy;
                }
                classified.push(evidence);
            }
            // Co-timed responses have no defensible UUID-based causal order. A mixed group
            // conservatively holds/resets; an all-success group advances at most one rung.
            let evidence = *classified
                .iter()
                .min_by_key(|e| evidence_priority(**e))
                .unwrap();
            // A form opened before the return was due does not become delayed retrieval by
            // sitting open until a later submission. Mixed co-timed starts are conservative.
            if group
                .iter()
                .any(|o| o.started_at_ms < s.due_at_ms || o.started_at_ms <= seed.occurred_at_ms)
            {
                s.ignored_early_returns += group.len();
                if time >= seed.occurred_at_ms
                    && matches!(
                        evidence,
                        TeacherRevisitEvidence::ClearError | TeacherRevisitEvidence::AnswerHelp
                    )
                {
                    s.rung = 0;
                    s.interval_ms = group_interval(DAY_MS);
                    s.due_at_ms = s.due_at_ms.min(time.saturating_add(s.interval_ms));
                    s.early_error_rechecks += 1;
                    s.last_attempt = Some(group[0].attempt_id);
                    s.last_evidence = evidence;
                }
                previous_practice = previous_practice.max(time);
                continue;
            }
            s.completed_returns += group.len();
            s.last_attempt = Some(group[0].attempt_id);
            s.last_evidence = evidence;
            for observed in classified {
                match observed {
                    TeacherRevisitEvidence::ClearError => s.clear_error_returns += 1,
                    TeacherRevisitEvidence::AnswerHelp => s.answer_help_returns += 1,
                    TeacherRevisitEvidence::ReplayOnly => s.replay_only_returns += 1,
                    TeacherRevisitEvidence::Unrated => s.unrated_returns += 1,
                    TeacherRevisitEvidence::ContestedAi => s.contested_returns += 1,
                    TeacherRevisitEvidence::RecentRestudy => s.recent_restudy_returns += 1,
                    _ => {}
                }
            }
            match evidence {
                TeacherRevisitEvidence::AiCorrect
                | TeacherRevisitEvidence::UnassistedSelfReport => {
                    s.rung = (s.rung + 1).min(RETURN_STEPS.len() - 1);
                    s.interval_ms = group_interval(RETURN_STEPS[s.rung]);
                    if evidence == TeacherRevisitEvidence::AiCorrect {
                        s.ai_correct_advances += 1;
                    } else {
                        s.self_report_advances += 1;
                    }
                }
                TeacherRevisitEvidence::ClearError | TeacherRevisitEvidence::AnswerHelp => {
                    s.rung = 0;
                    s.interval_ms = group_interval(DAY_MS);
                }
                _ => {}
            }
            let delay = if evidence == TeacherRevisitEvidence::ContestedAi {
                group_interval(DAY_MS).min(s.interval_ms)
            } else {
                s.interval_ms
            };
            s.due_at_ms = time.saturating_add(delay);
            previous_practice = time;
        }
        Ok(s)
    }
}

// Lower ranks dominate a co-timed group; timestamps never select a positive winner.
fn evidence_priority(e: TeacherRevisitEvidence) -> u8 {
    match e {
        TeacherRevisitEvidence::ClearError => 0,
        TeacherRevisitEvidence::AnswerHelp => 1,
        TeacherRevisitEvidence::ContestedAi => 2,
        TeacherRevisitEvidence::ReplayOnly => 3,
        TeacherRevisitEvidence::RecentRestudy => 4,
        TeacherRevisitEvidence::Unrated => 5,
        TeacherRevisitEvidence::AiCorrect => 6,
        TeacherRevisitEvidence::UnassistedSelfReport => 7,
        TeacherRevisitEvidence::GuidedSeed => 8,
    }
}

/// Conservative exclusion from generic passage reviews, including unattempted references whose
/// source document has not synced yet. The reserved generated-source prefix is intentionally
/// enough: malformed/imported lookalikes must not silently receive a substituted generic task.
pub fn is_teacher_generated_reference(event: &Event) -> bool {
    matches!(&event.fact,Fact::PassageRevised(r) if r.passage.source.starts_with("AI teacher exercise"))
}

/// Canonical generated reference; preserves the existing app's exact v2 fact bytes/identities.
/// The caller supplies the deterministic v2 envelope (source device, revision ID, zero time).
pub fn teacher_practice_reference_fact(
    answer: &TeacherResponse,
    index: usize,
) -> Result<(Uuid, Uuid, Fact), Error> {
    let invalid = || Error::Invalid("teacher exercise reference is unavailable or invalid".into());
    answer.request.validate().map_err(|_| invalid())?;
    answer
        .content
        .validate_for(&answer.request)
        .map_err(|_| invalid())?;
    if answer.provenance.review_status != ReviewStatus::AiGeneratedTeaching
        || answer.provenance.response_id.is_empty()
    {
        return Err(invalid());
    }
    let e = answer.content.practice.get(index).ok_or_else(invalid)?;
    let source = teacher_document_id(answer);
    let passage = Uuid::new_v5(&source, index.to_string().as_bytes());
    let revision = Uuid::new_v5(&passage, b"reference-v1");
    let title = match &answer.request {
        TeacherRequest::TeachLesson(r) => r.title.as_str(),
        _ => "A teacher follow-up",
    };
    Ok((
        source,
        revision,
        Fact::PassageRevised(Revision {
            passage: Passage {
                id: passage,
                title: format!("{title} · practice {}", index + 1),
                russian: e.reference_russian.clone().ok_or_else(invalid)?,
                translation: e.reference_english.clone().ok_or_else(invalid)?,
                source: format!(
                    "AI teacher exercise · {} · {} · {} · {}",
                    answer.provenance.provider,
                    answer.provenance.model,
                    answer.provenance.response_id,
                    index
                ),
                coach_reviewed: false,
            },
            parents: BTreeSet::new(),
        }),
    ))
}

/// Exact original question and plain reference used by the guided text/audio builders. Oral
/// attempts append their own validated recording evidence after this unchanged reference.
pub fn teacher_practice_prompt_reference(
    answer: &TeacherResponse,
    index: usize,
) -> Result<(String, String), Error> {
    teacher_practice_reference_fact(answer, index)?;
    let e = &answer.content.practice[index];
    let russian = e.reference_russian.as_deref().unwrap();
    let english = e.reference_english.as_deref().unwrap();
    let prompt = if e.skill == Some(TeacherSkill::Reading) {
        format!("{}\n\nRussian text:\n{russian}", e.prompt)
    } else {
        e.prompt.clone()
    };
    let reference = format!(
        "Example answer: {}\nReference Russian: {russian}\nReference meaning: {english}\nTeaching note: {}",
        e.expected_response
            .as_deref()
            .unwrap_or("An open response; accept valid alternatives."),
        e.teaching_note
    );
    Ok((prompt, reference))
}
