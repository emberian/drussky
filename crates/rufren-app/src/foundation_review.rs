//! Bounded returns to exact authored recall tasks. These intervals are task cadence, not mastery.
use crate::{
    foundation::FoundationState,
    model::{AppState, Page},
    repository::{Result, now_ms},
};
use rufren_content::foundations::{FoundationRecallMode, foundation_step_pack_revision};
use rufren_content::{FoundationAction, FoundationStep};
use rufren_core::*;
use uuid::Uuid;

const POLICY: &str = "foundation-exact-recall-cadence-v1";
#[derive(Clone)]
pub(crate) struct FoundationReviewEntry {
    pub step_id: String,
    pub label: String,
    pub skill: Skill,
    pub due_at_ms: i64,
}
fn recall_key(step: &FoundationStep) -> Option<ReviewKey> {
    let FoundationAction::Recall { mode, .. } = step.action else {
        return None;
    };
    let skill = match mode {
        FoundationRecallMode::ListeningMeaning => Skill::Listening,
        FoundationRecallMode::ReadingMeaning => Skill::Reading,
        FoundationRecallMode::Written => Skill::Writing,
        FoundationRecallMode::Spoken => Skill::Speaking,
    };
    Some(ReviewKey {
        revision: Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!(
                "rufren-foundation:{}:{}",
                foundation_step_pack_revision(&step.id),
                step.id
            )
            .as_bytes(),
        ),
        skill,
    })
}
fn exact_context(library: &Library, attempt: Uuid, step: &FoundationStep) -> bool {
    let contexts = library.contexts(attempt);
    !contexts.is_empty()
        && contexts.iter().all(|(_, c)| {
            c.content_pack.as_deref() == Some("foundations")
                && c.content_revision.as_deref()
                    == Some(foundation_step_pack_revision(&step.id).as_str())
                && c.exercise_id.as_deref()
                    == Some(format!("{}@{}", step.id, step.revision).as_str())
                && c.prompt.as_deref() == serde_json::to_string(step).ok().as_deref()
        })
}
fn criterion_evidence(
    library: &Library,
    step: &FoundationStep,
    id: Uuid,
    attempt: &Attempt,
) -> ReviewEvidence {
    if !exact_context(library, id, step) {
        return ReviewEvidence::NotReported;
    }
    if !attempt.cues.is_empty() {
        return ReviewEvidence::Assisted;
    }
    if matches!(
        step.action,
        FoundationAction::Recall {
            mode: FoundationRecallMode::Spoken,
            ..
        }
    ) {
        return ReviewEvidence::NotReported;
    }
    let dimension = if attempt.skill == Skill::Writing {
        JudgmentDimension::Spelling
    } else {
        JudgmentDimension::Comprehension
    };
    let judgments = library.judgments(id, dimension);
    let verdicts: Vec<_> = judgments
        .iter()
        .filter(|(_, j)| {
            j.confidence_percent == 100
                && j.provenance.coach == "Prepared foundation answer key"
                && j.provenance.method == "Exact authored form, accepted meaning or known confusion"
        })
        .map(|(_, j)| j.verdict)
        .collect();
    let correct = verdicts.contains(&Verdict::Correct);
    let adverse = verdicts
        .iter()
        .any(|v| matches!(v, Verdict::Incorrect | Verdict::PartlyCorrect));
    match (correct, adverse) {
        (true, true) => ReviewEvidence::ContestedJudgment,
        (_, true) => ReviewEvidence::CoachCorrection,
        (true, false) => ReviewEvidence::CoachConfirmedUnassisted,
        _ => ReviewEvidence::NotReported,
    }
}
impl AppState {
    fn foundation_review_projection(&self, step: &FoundationStep, now: i64) -> Option<ReviewState> {
        let key = recall_key(step)?;
        if !self.library.attempts().any(|(e, a)| {
            a.revision == key.revision
                && a.skill == key.skill
                && e.recorded_at_ms <= now
                && exact_context(&self.library, e.id, step)
        }) {
            return None;
        }
        let mut state =
            SpacedReview.project_with_evidence(&self.library, key, now, POLICY, |id, a| {
                criterion_evidence(&self.library, step, id, a)
            });
        // Unknown wording and ungraded speech are missing criterion outcomes. Offer another
        // encounter without promoting a success rung or assigning a memory failure.
        if state.interval_ms == 0 {
            state.interval_ms = DAY_MS;
            state.due_at_ms = state.last_attempt_ms?.saturating_add(DAY_MS);
        }
        Some(state)
    }
    pub(crate) fn foundation_review_queue(&self, now: i64) -> Vec<FoundationReviewEntry> {
        let mut entries = vec![];
        for step in rufren_content::foundations()
            .episodes
            .into_iter()
            .flat_map(|e| e.steps)
        {
            let Some(state) = self.foundation_review_projection(&step, now) else {
                continue;
            };
            let target = ReviewTarget::Passage {
                revision: state.key.revision,
                skill: state.key.skill,
            };
            let timing = self.review_timing(target, state.due_at_ms, now);
            if !timing.is_due(now) {
                continue;
            }
            entries.push(FoundationReviewEntry {
                step_id: step.id,
                label: match state.key.skill {
                    Skill::Listening => "Listen and remember",
                    Skill::Reading => "Read a familiar word",
                    Skill::Writing => "Recall a spelling",
                    Skill::Speaking => "Say a familiar word",
                }
                .into(),
                skill: state.key.skill,
                due_at_ms: timing.effective_due_at_ms.unwrap(),
            });
        }
        entries.sort_by_key(|e| (e.due_at_ms, e.step_id.clone()));
        entries.truncate(3);
        entries
    }
    pub(crate) fn foundation_start_review(&mut self, step_id: &str) -> Result<()> {
        self.foundation_start_review_at(step_id, now_ms())
    }
    pub(crate) fn foundation_start_review_at(&mut self, step_id: &str, now: i64) -> Result<()> {
        self.refresh()?;
        let course = self.foundation_state()?;
        if course.review_resume.is_some() || course.writing_checkpoint.is_some() {
            return self.navigate(Page::Foundations);
        }
        if course.pending.is_some() {
            return Err("Save your current course answer before starting a return.".into());
        }
        let pack = rufren_content::foundations();
        let (episode, step_index, step) = pack
            .episodes
            .iter()
            .enumerate()
            .find_map(|(ei, e)| {
                e.steps
                    .iter()
                    .enumerate()
                    .find(|(_, s)| s.id == step_id)
                    .map(|(si, s)| (ei, si, s))
            })
            .ok_or("This authored recall task is unavailable.")?;
        let state = self
            .foundation_review_projection(step, now)
            .ok_or("Try this recall in the course before returning to it.")?;
        let target = ReviewTarget::Passage {
            revision: state.key.revision,
            skill: state.key.skill,
        };
        if !self.review_timing(target, state.due_at_ms, now).is_due(now) {
            return Err("This return is not available yet. Continue your course instead.".into());
        }
        let mut active = course.clone();
        active.review_resume = Some(Box::new(course));
        active.episode = episode;
        active.episode_id = pack.episodes[episode].id.clone();
        active.step = step_index;
        active.step_id = step.id.clone();
        active.step_revision = step.revision.clone();
        active.session = Uuid::new_v4();
        active.draft.clear();
        active.cues.clear();
        active.played.clear();
        active.revealed = false;
        active.feedback = None;
        active.accepted = false;
        active.cannot_yet = false;
        active.recording = None;
        active.finished = false;
        active.pending = None;
        let evidence = self.library.events.keys().copied().collect();
        let decision = self.prepare_review_start(
            active.session,
            target,
            state.due_at_ms,
            now,
            now,
            POLICY,
            "Return to this exact authored recall after its assigned gap",
            evidence,
        )?;
        self.repo
            .append(&[self.foundation_revision(&active, step)?])?;
        self.record_review_start(&decision)?;
        self.foundation_save(&active)?;
        self.navigate(Page::Foundations)
    }
    pub(crate) fn foundation_finish_review(&self, active: &FoundationState) -> Result<()> {
        let course = active
            .review_resume
            .as_ref()
            .ok_or("This is not an active recall return.")?;
        self.foundation_save(course)
    }
}
