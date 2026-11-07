//! Sparse handwriting retrieval within the course, plus voluntary bounded returns.
//! A checkpoint overlays the exact course cursor; it never rewrites authored tasks.
use crate::{
    foundation::FoundationState,
    handwriting::{HandwritingActivity, WritingOriginalRef, WritingReturnIntent, WritingStage},
    model::{AppState, Page},
    repository::{Result, now_ms},
};
use rufren_content::handwriting_practice::{
    HANDWRITING_PRACTICE_PLAN_REVISION, checkpoint, checkpoints_after,
};
use rufren_content::handwriting_tasks::{
    FoundationHandwritingInputFocus, FoundationHandwritingSequence, handwriting_tasks,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

const DAY_MS: i64 = 86_400_000;
const VISITS: &str = "foundation.writing.visit:";
const SNOOZE: &str = "foundation.writing.snooze_until";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FoundationWritingCheckpointKind {
    Lesson,
    Return,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FoundationWritingItem {
    pub spec_id: String,
    pub spec_revision: String,
    pub task_id: String,
    pub task_revision: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FoundationWritingCheckpoint {
    pub version: u32,
    pub id: Uuid,
    pub plan_revision: String,
    pub anchor_session: Uuid,
    pub anchor_step: String,
    pub anchor_revision: String,
    pub kind: FoundationWritingCheckpointKind,
    pub items: Vec<FoundationWritingItem>,
    pub index: usize,
    pub started_at_ms: i64,
}
impl FoundationWritingCheckpoint {
    pub(crate) fn item_session(&self) -> Uuid {
        Uuid::new_v5(&self.id, format!("item:{}", self.index).as_bytes())
    }
    pub(crate) fn run_id(&self) -> Uuid {
        Uuid::new_v5(&self.item_session(), b"writing-checkpoint-v1")
    }
    pub(crate) fn item(&self) -> &FoundationWritingItem {
        &self.items[self.index]
    }
    pub(crate) fn validate(&self, course: &FoundationState) -> Result<()> {
        if self.version != 1
            || self.id.is_nil()
            || self.plan_revision != HANDWRITING_PRACTICE_PLAN_REVISION
            || self.anchor_session != course.session
            || self.anchor_step != course.step_id
            || self.anchor_revision != course.step_revision
            || self.items.is_empty()
            || self.items.len() > 2
            || self.index >= self.items.len()
            || course.review_resume.is_some()
        {
            return Err("Your saved listening-and-writing turn needs recovery. Its paper and course place are preserved.".into());
        }
        for item in &self.items {
            let spec = checkpoint(&item.spec_id)
                .ok_or("The saved writing checkpoint is unavailable. Your work is kept.")?;
            let task = handwriting_tasks()
                .into_iter()
                .find(|t| t.id == item.task_id)
                .ok_or("The saved writing model is unavailable. Your work is kept.")?;
            if spec.revision != item.spec_revision
                || spec.task_id != item.task_id
                || task.revision != item.task_revision
                || task.input_focus != FoundationHandwritingInputFocus::WholeTarget
                || (self.kind == FoundationWritingCheckpointKind::Lesson
                    && spec.after_step_id != self.anchor_step)
                || !task
                    .prerequisite_step_ids
                    .iter()
                    .all(|id| course.encounters.contains(&format!("{id}:encountered")))
            {
                return Err("The saved writing checkpoint no longer matches its taught source. Your work is kept.".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FoundationWritingCheckpointView {
    pub id: Uuid,
    pub item_session: Uuid,
    pub index: usize,
    pub count: usize,
    pub kind: FoundationWritingCheckpointKind,
    pub instruction: String,
    pub comparison_focus: String,
    pub next_label: String,
    pub task_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FoundationWritingReturn {
    pub task_id: String,
    pub label: String,
    pub due_at_ms: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FoundationWritingOriginal {
    round_id: Uuid,
    attempt_id: Option<Uuid>,
    sequence: FoundationHandwritingSequence,
    original: WritingOriginalRef,
    pre_submission_cues: Option<Vec<String>>,
}
/// A local receipt of one actual opportunity. Skipping has no correctness meaning.
/// Originals and assistance are bound here before the course cursor can advance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FoundationWritingVisit {
    version: u32,
    item_session: Uuid,
    checkpoint_id: Uuid,
    kind: FoundationWritingCheckpointKind,
    item: FoundationWritingItem,
    anchor_session: Uuid,
    anchor_step: String,
    anchor_revision: String,
    offered_at_ms: i64,
    finished_at_ms: i64,
    skipped: bool,
    #[serde(default)]
    help_requested: bool,
    run_id: Option<Uuid>,
    originals: Vec<FoundationWritingOriginal>,
}

fn eligible_items(course: &FoundationState) -> Vec<FoundationWritingItem> {
    if course.writing_checkpoint.is_some() || course.review_resume.is_some() || course.finished {
        return vec![];
    }
    let tasks = handwriting_tasks();
    checkpoints_after(&course.step_id)
        .into_iter()
        .filter_map(|spec| {
            let task = tasks.iter().find(|task| task.id == spec.task_id)?;
            (task.input_focus == FoundationHandwritingInputFocus::WholeTarget
                && task
                    .prerequisite_step_ids
                    .iter()
                    .all(|id| course.encounters.contains(&format!("{id}:encountered"))))
            .then(|| FoundationWritingItem {
                spec_id: spec.id.into(),
                spec_revision: spec.revision.into(),
                task_id: task.id.clone(),
                task_revision: task.revision.clone(),
            })
        })
        .collect()
}

impl AppState {
    pub(crate) fn foundation_writing_checkpoint(
        &self,
    ) -> Result<Option<FoundationWritingCheckpointView>> {
        let course = self.foundation_state()?;
        let Some(active) = &course.writing_checkpoint else {
            return Ok(None);
        };
        active.validate(&course)?;
        let item = active.item();
        let spec = checkpoint(&item.spec_id).ok_or("The writing checkpoint is unavailable.")?;
        Ok(Some(FoundationWritingCheckpointView {
            id: active.id,
            item_session: active.item_session(),
            index: active.index,
            count: active.items.len(),
            kind: active.kind,
            instruction: spec.instruction.into(),
            comparison_focus: spec.comparison_focus.into(),
            next_label: if active.index + 1 < active.items.len() {
                "Next listening turn"
            } else if active.kind == FoundationWritingCheckpointKind::Return {
                "Back to my course"
            } else {
                "Continue the lesson"
            }
            .into(),
            task_id: item.task_id.clone(),
        }))
    }

    pub(crate) fn foundation_has_writing_checkpoint_after_current(&self) -> bool {
        self.foundation_state()
            .is_ok_and(|course| !eligible_items(&course).is_empty())
    }

    /// Called only after the authored base response has passed its own next-step guard.
    /// Skipped prerequisites never become eligible just because their page was visited.
    pub(crate) fn foundation_offer_writing_checkpoint(
        &self,
        course: &FoundationState,
    ) -> Result<bool> {
        if course.writing_checkpoint.is_some() || course.review_resume.is_some() || course.finished
        {
            return Ok(false);
        }
        let items = eligible_items(course);
        if items.is_empty() {
            return Ok(false);
        }
        let mut active = course.clone();
        active.writing_checkpoint = Some(FoundationWritingCheckpoint {
            version: 1,
            id: Uuid::new_v4(),
            plan_revision: HANDWRITING_PRACTICE_PLAN_REVISION.into(),
            anchor_session: course.session,
            anchor_step: course.step_id.clone(),
            anchor_revision: course.step_revision.clone(),
            kind: FoundationWritingCheckpointKind::Lesson,
            items,
            index: 0,
            started_at_ms: now_ms(),
        });
        self.foundation_save(&active)?;
        Ok(true)
    }

    pub(crate) fn foundation_writing_transition_pending(&self, item_session: Uuid) -> Result<bool> {
        Ok(self
            .repo
            .get::<FoundationWritingVisit>(&format!("{VISITS}{item_session}"))?
            .is_some())
    }
    /// A durable visit precedes the course write. Resume retries that exact
    /// transition instead of opening a new response whose evidence could be lost.
    pub(crate) fn foundation_recover_writing_transition(&self) -> Result<bool> {
        let course = self.foundation_state()?;
        let Some(checkpoint) = &course.writing_checkpoint else {
            return Ok(false);
        };
        let Some(visit) = self
            .repo
            .get::<FoundationWritingVisit>(&format!("{VISITS}{}", checkpoint.item_session()))?
        else {
            return Ok(false);
        };
        self.foundation_finish_writing_checkpoint_at(
            checkpoint.item_session(),
            visit.skipped,
            visit.finished_at_ms,
        )?;
        Ok(true)
    }
    pub(crate) fn foundation_skip_writing_checkpoint(&self, item_session: Uuid) -> Result<()> {
        self.foundation_finish_writing_checkpoint_at(item_session, true, now_ms())
    }

    pub(crate) fn foundation_finish_writing_checkpoint(
        &self,
        run: &HandwritingActivity,
    ) -> Result<()> {
        let Some(WritingReturnIntent::FoundationCheckpoint { item_session, .. }) =
            &run.return_intent
        else {
            return Err("This writing turn does not belong to a course checkpoint.".into());
        };
        if run.stage != WritingStage::Complete {
            return Err("Compare this writing turn before continuing.".into());
        }
        let course = self.foundation_state()?;
        // A retry after durable advancement cannot advance the new task again.
        if course
            .writing_checkpoint
            .as_ref()
            .is_none_or(|c| c.item_session() != *item_session)
        {
            return Ok(());
        }
        self.foundation_finish_writing_checkpoint_at(*item_session, false, now_ms())
    }

    pub(crate) fn foundation_finish_writing_checkpoint_at(
        &self,
        expected: Uuid,
        skipped: bool,
        now: i64,
    ) -> Result<()> {
        let mut course = self.foundation_state()?;
        let mut checkpoint = course
            .writing_checkpoint
            .clone()
            .ok_or("That writing turn has already ended. Your work is saved.")?;
        checkpoint.validate(&course)?;
        if checkpoint.item_session() != expected {
            return Err("That writing turn has changed. Your current paper is kept.".into());
        }
        let run = self.writing_checkpoint_run(&checkpoint)?;
        if !skipped
            && run
                .as_ref()
                .is_none_or(|r| r.stage != WritingStage::Complete)
        {
            return Err("Finish comparing this writing turn, or skip it for now.".into());
        }
        let key = format!("{VISITS}{expected}");
        if let Some(saved) = self.repo.get::<FoundationWritingVisit>(&key)? {
            if saved.version != 1
                || saved.item_session != expected
                || saved.checkpoint_id != checkpoint.id
                || saved.item != *checkpoint.item()
                || saved.anchor_session != course.session
            {
                return Err(
                    "The saved writing return differs from this turn. Your original is preserved."
                        .into(),
                );
            }
        } else {
            let originals = run
                .as_ref()
                .map(|run| {
                    run.history
                        .iter()
                        .filter_map(|round| {
                            round
                                .original
                                .clone()
                                .map(|original| FoundationWritingOriginal {
                                    round_id: round.round_id,
                                    attempt_id: round.attempt_id,
                                    sequence: round.sequence,
                                    original,
                                    pre_submission_cues: round.pre_submission_cues.clone(),
                                })
                        })
                        .chain(
                            run.original
                                .clone()
                                .map(|original| FoundationWritingOriginal {
                                    round_id: run.round_id,
                                    attempt_id: Some(run.attempt_id),
                                    sequence: run.sequence,
                                    original,
                                    pre_submission_cues: run.pre_submission_cues.clone(),
                                }),
                        )
                        .collect()
                })
                .unwrap_or_default();
            let visit = FoundationWritingVisit {
                version: 1,
                item_session: expected,
                checkpoint_id: checkpoint.id,
                kind: checkpoint.kind,
                item: checkpoint.item().clone(),
                anchor_session: course.session,
                anchor_step: course.step_id.clone(),
                anchor_revision: course.step_revision.clone(),
                offered_at_ms: checkpoint.started_at_ms,
                finished_at_ms: now,
                skipped,
                help_requested: run
                    .as_ref()
                    .map(|r| self.writing_checkpoint_help_requested(r))
                    .transpose()?
                    .unwrap_or(false),
                run_id: run.as_ref().map(|r| r.id),
                originals,
            };
            self.repo.set(&key, &visit)?;
        }
        if checkpoint.index + 1 < checkpoint.items.len() {
            checkpoint.index += 1;
            course.writing_checkpoint = Some(checkpoint);
            return self.foundation_save(&course);
        }
        course.writing_checkpoint = None;
        if checkpoint.kind == FoundationWritingCheckpointKind::Return {
            // The original cursor, draft, help, audio and pending course work never moved.
            self.foundation_save(&course)
        } else {
            // One write clears the overlay AND advances. A crash cannot re-offer this batch.
            self.foundation_advance_after_writing(course)
        }
    }

    fn foundation_writing_visits(&self) -> Result<Vec<FoundationWritingVisit>> {
        self.repo
            .metadata_with_prefix(VISITS)?
            .into_iter()
            .map(|(key, value)| {
                let visit: FoundationWritingVisit =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                if visit.version != 1
                    || visit.item_session.is_nil()
                    || key != format!("{VISITS}{}", visit.item_session)
                {
                    return Err(
                        "A saved writing practice record needs recovery. Its original is kept."
                            .into(),
                    );
                }
                Ok(visit)
            })
            .collect()
    }

    fn foundation_writing_returns_checked(&self, now: i64) -> Result<Vec<FoundationWritingReturn>> {
        let course = self.foundation_state()?;
        if course.writing_checkpoint.is_some()
            || course.review_resume.is_some()
            || self
                .repo
                .get::<i64>(SNOOZE)?
                .is_some_and(|until| until > now)
        {
            return Ok(vec![]);
        }
        let visits = self.foundation_writing_visits()?;
        let recent_returns = visits
            .iter()
            .filter(|v| {
                v.kind == FoundationWritingCheckpointKind::Return
                    && v.finished_at_ms <= now
                    && v.finished_at_ms > now.saturating_sub(DAY_MS)
            })
            .count();
        let capacity = 2usize.saturating_sub(recent_returns);
        if capacity == 0 {
            return Ok(vec![]);
        }
        let mut by_task: BTreeMap<&str, Vec<&FoundationWritingVisit>> = BTreeMap::new();
        for visit in &visits {
            if visit.finished_at_ms <= now {
                by_task.entry(&visit.item.task_id).or_default().push(visit);
            }
        }
        let tasks = handwriting_tasks();
        let mut due = vec![];
        for (task_id, history) in by_task {
            // A saved original or an explicitly requested example makes a useful
            // return. An untouched offer/skip is neither an attempt nor practice.
            if !history
                .iter()
                .any(|v| !v.originals.is_empty() || v.help_requested)
            {
                continue;
            }
            let last = history.iter().max_by_key(|v| v.finished_at_ms).unwrap();
            let due_at_ms = last.finished_at_ms.saturating_add(DAY_MS);
            if due_at_ms > now {
                continue;
            }
            let Some(spec) = checkpoint(&last.item.spec_id) else {
                continue;
            };
            let Some(task) = tasks
                .iter()
                .find(|t| t.id == task_id && t.revision == last.item.task_revision)
            else {
                continue;
            };
            if spec.revision != last.item.spec_revision
                || spec.task_id != task.id
                || !task
                    .prerequisite_step_ids
                    .iter()
                    .all(|id| course.encounters.contains(&format!("{id}:encountered")))
            {
                continue;
            }
            due.push(FoundationWritingReturn {
                task_id: task_id.into(),
                label: "Listen and write a familiar item".into(),
                due_at_ms,
            });
        }
        due.sort_by(|a, b| (a.due_at_ms, &a.task_id).cmp(&(b.due_at_ms, &b.task_id)));
        due.truncate(capacity);
        Ok(due)
    }
    pub(crate) fn foundation_writing_returns(&self, now: i64) -> Vec<FoundationWritingReturn> {
        self.foundation_writing_returns_checked(now)
            .unwrap_or_default()
    }
    pub(crate) fn foundation_start_writing_return(&mut self, task_id: &str) -> Result<()> {
        self.foundation_start_writing_return_at(task_id, now_ms())
    }
    pub(crate) fn foundation_start_writing_return_at(
        &mut self,
        task_id: &str,
        now: i64,
    ) -> Result<()> {
        let mut course = self.foundation_state()?;
        if course.writing_checkpoint.is_some() {
            return self.navigate(Page::Foundations);
        }
        if course.review_resume.is_some() || course.pending.is_some() {
            return Err("Finish or pause the current recall before starting this return.".into());
        }
        if !self
            .foundation_writing_returns_checked(now)?
            .iter()
            .any(|r| r.task_id == task_id)
        {
            return Err("This writing return is not due. Your course is ready to continue.".into());
        }
        let item = self
            .foundation_writing_visits()?
            .into_iter()
            .filter(|v| v.item.task_id == task_id && v.finished_at_ms <= now)
            .max_by_key(|v| v.finished_at_ms)
            .ok_or("This writing return has no saved attempt.")?
            .item;
        course.writing_checkpoint = Some(FoundationWritingCheckpoint {
            version: 1,
            id: Uuid::new_v4(),
            plan_revision: HANDWRITING_PRACTICE_PLAN_REVISION.into(),
            anchor_session: course.session,
            anchor_step: course.step_id.clone(),
            anchor_revision: course.step_revision.clone(),
            kind: FoundationWritingCheckpointKind::Return,
            items: vec![item],
            index: 0,
            started_at_ms: now,
        });
        self.foundation_save(&course)?;
        self.navigate(Page::Foundations)
    }
    pub(crate) fn foundation_snooze_writing_returns(&self) -> Result<()> {
        self.repo.set(SNOOZE, &now_ms().saturating_add(DAY_MS))
    }
}
