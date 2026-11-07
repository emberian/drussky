//! Raw local ink for an authored hand-printing task. No OCR, phonetic score, or mastery inference.
//! Key this component by scope_id. Parent navigation MUST await flush(scope_id) before unmounting.
use crate::{
    model::{AppState, Page},
    repository::Result,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MODEL: &str = "hand-printed-six-v2";
const ALPHABET_MODEL: &str = "hand-printed-alphabet-v1";
fn legacy_model() -> String {
    MODEL.into()
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InkMode {
    Trace,
    Copy,
    Hidden,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InkAudioRole {
    TaskInput,
    Model,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct HandwritingTask {
    #[serde(default = "legacy_model")]
    pub model_version: String,
    pub id: String,
    pub revision: String,
    pub target: String,
    pub instruction: String,
    pub meaning: Option<String>,
    pub reference_audio_id: Option<String>,
    pub audio_role: InkAudioRole,
    pub formation_hint: String,
    pub paper: InkPaper,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InkPaper {
    pub width: u32,
    pub height: u32,
    pub model_size: u32,
    pub baseline: u32,
    pub layout: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InkPoint {
    pub x: f64,
    pub y: f64,
    pub time_ms: f64,
    pub pressure: f64,
    pub tilt_x: f64,
    pub tilt_y: f64,
    pub twist: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InkGeometry {
    pub css_width: f64,
    pub css_height: f64,
    pub device_pixel_ratio: f64,
    pub viewport_scale: f64,
    pub units_per_css_pixel: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InkStroke {
    pub pointer_id: i64,
    pub pointer_type: String,
    #[serde(default)]
    pub capture_geometry: Option<InkGeometry>,
    pub active: bool,
    pub interrupted: bool,
    pub cancel_reason: Option<String>,
    pub points: Vec<InkPoint>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InkAid {
    pub kind: String,
    pub value: String,
    pub at_ms: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HandwritingSubmission {
    pub version: u32,
    pub model_version: String,
    pub view_box: [u32; 2],
    pub scope_id: String,
    pub task_id: String,
    pub task_revision: String,
    pub target: String,
    pub audio_id: Option<String>,
    pub audio_role: InkAudioRole,
    pub attempt_id: Uuid,
    pub created_at_ms: i64,
    pub mode: InkMode,
    pub input_mode: String,
    pub meaning_visible: bool,
    pub sound_visible: bool,
    pub sealed: bool,
    pub strokes: Vec<InkStroke>,
    pub aids: Vec<InkAid>,
    #[serde(default)]
    pub saved_at_ms: Option<i64>,
    #[serde(default)]
    pub prior_attempt_id: Option<Uuid>,
    #[serde(default)]
    pub pending_snapshot: Option<Box<HandwritingSubmission>>,
}
impl HandwritingSubmission {
    fn validate(&self, scope: &str, task: &HandwritingTask, original: bool) -> Result<()> {
        if self.version != 1
            || self.model_version != task.model_version
            || self.view_box != [task.paper.width, task.paper.height]
            || self.scope_id != scope
            || self.task_id != task.id
            || self.task_revision != task.revision
            || self.target != task.target
            || self.audio_id != task.reference_audio_id
            || self.audio_role != task.audio_role
            || self.attempt_id.is_nil()
            || !matches!(self.input_mode.as_str(), "pen" | "finger" | "mouse")
            || self.strokes.len() > 1000
            || self.aids.len() > 4096
        {
            return Err("This ink belongs to a different task or drawing version. Its original is preserved.".into());
        }
        let mut count = 0;
        for stroke in &self.strokes {
            count += stroke.points.len();
            if !matches!(stroke.pointer_type.as_str(), "pen" | "touch" | "mouse")
                || stroke.points.is_empty()
                || stroke.points.len() > 100_000
            {
                return Err("A saved ink stroke cannot be read. Its original is preserved.".into());
            }
            if let Some(g) = &stroke.capture_geometry {
                if ![
                    g.css_width,
                    g.css_height,
                    g.device_pixel_ratio,
                    g.viewport_scale,
                    g.units_per_css_pixel,
                    g.offset_x,
                    g.offset_y,
                ]
                .iter()
                .all(|v| v.is_finite())
                    || g.css_width <= 0.0
                    || g.css_height <= 0.0
                    || g.units_per_css_pixel <= 0.0
                {
                    return Err("The drawing's original coordinate scale is unavailable. Its ink is preserved.".into());
                }
            }
            for p in &stroke.points {
                if ![
                    p.x, p.y, p.time_ms, p.pressure, p.tilt_x, p.tilt_y, p.twist, p.width, p.height,
                ]
                .iter()
                .all(|v| v.is_finite())
                    || p.x.abs() > 10000.0
                    || p.y.abs() > 10000.0
                    || !(0.0..=1.0).contains(&p.pressure)
                {
                    return Err(
                        "A saved ink point cannot be read. Its original is preserved.".into(),
                    );
                }
            }
        }
        if count > 200_000 {
            return Err("This drawing is too large to save safely. Keep the pad open.".into());
        }
        if original
            && (!self.sealed
                || self.saved_at_ms.is_none()
                || self.strokes.is_empty()
                || self.strokes.iter().any(|s| s.active)
                || self.pending_snapshot.is_some())
        {
            return Err("The original drawing has not finished saving.".into());
        }
        if let Some(pending) = &self.pending_snapshot {
            pending.validate(scope, task, true)?;
            if pending.attempt_id != self.attempt_id || pending.strokes != self.strokes {
                return Err(
                    "The pending original no longer matches this ink. Its original is preserved."
                        .into(),
                );
            }
        }
        Ok(())
    }
}
fn draft_key(scope: &str) -> String {
    format!("handwriting.draft:{scope}")
}
fn original_key(id: Uuid) -> String {
    format!("handwriting.original:{id}")
}
impl AppState {
    pub(crate) fn handwriting_draft(
        &self,
        scope: &str,
        task: &HandwritingTask,
    ) -> Result<Option<HandwritingSubmission>> {
        let draft: Option<HandwritingSubmission> = self.repo.get(&draft_key(scope))?;
        if let Some(d) = &draft {
            d.validate(scope, task, false)?
        }
        Ok(draft)
    }
    pub(crate) fn handwriting_store_draft(
        &self,
        scope: &str,
        task: &HandwritingTask,
        draft: &HandwritingSubmission,
    ) -> Result<()> {
        draft.validate(scope, task, false)?;
        self.repo.set(&draft_key(scope), draft)
    }
    pub(crate) fn handwriting_store_original(
        &self,
        scope: &str,
        task: &HandwritingTask,
        original: &HandwritingSubmission,
    ) -> Result<()> {
        original.validate(scope, task, true)?;
        let key = original_key(original.attempt_id);
        if let Some(saved) = self.repo.get::<HandwritingSubmission>(&key)? {
            if &saved != original {
                return Err("An earlier original uses this identity. It is preserved.".into());
            }
        } else {
            self.repo.set(&key, original)?
        }
        let index_key = format!("handwriting.originals:{scope}");
        let mut ids: Vec<Uuid> = self.repo.get(&index_key)?.unwrap_or_default();
        if !ids.contains(&original.attempt_id) {
            ids.push(original.attempt_id);
            self.repo.set(&index_key, &ids)?
        }
        Ok(())
    }
}

use rufren_content::handwriting_tasks::{
    FoundationHandwritingInputFocus, FoundationHandwritingLayout,
    FoundationHandwritingSequence as WritingSequence, FoundationHandwritingTask,
    handwriting_task_for_step, handwriting_tasks,
};
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WritingStage {
    Writing,
    Compare,
    Say,
    Complete,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SayDisposition {
    #[default]
    NotReported,
    SelfReportedSaid,
    Skipped,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WritingOriginalRef {
    pub id: Uuid,
    pub scope_id: String,
    pub task_id: String,
    pub task_revision: String,
    pub format: String,
    pub sha256: String,
    pub local_key: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WritingRound {
    pub round_id: Uuid,
    pub scope_id: String,
    pub sequence: WritingSequence,
    pub original: Option<WritingOriginalRef>,
    pub say: SayDisposition,
    // Older archives did not distinguish Compare from Complete. Keep that unknown.
    #[serde(default)]
    pub stage: Option<WritingStage>,
    #[serde(default)]
    pub attempt_id: Option<Uuid>,
    #[serde(default)]
    pub audio_completed: Option<bool>,
    #[serde(default)]
    pub pre_submission_cues: Option<Vec<String>>,
}
/// A read-only projection of an exact round; opening it never replaces the live round.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WritingRoundReview {
    pub run_id: Uuid,
    pub round_id: Uuid,
    pub scope_id: String,
    pub sequence: WritingSequence,
    pub task: HandwritingTask,
    pub stage: Option<WritingStage>,
    pub say: SayDisposition,
    pub original: Option<WritingOriginalRef>,
    pub originals: Vec<WritingOriginalReview>,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WritingOriginalReview {
    pub reference: WritingOriginalRef,
    pub payload: serde_json::Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum WritingReturnIntent {
    Foundations {
        session: Uuid,
        step: String,
        revision: String,
    },
    FoundationCheckpoint {
        checkpoint: Uuid,
        item_session: Uuid,
        anchor_session: Uuid,
        step: String,
        revision: String,
    },
    Catalog,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HandwritingActivity {
    pub version: u32,
    pub id: Uuid,
    pub course_session: Uuid,
    pub course_step: String,
    pub course_step_revision: String,
    pub task_id: String,
    pub task_revision: String,
    pub sequence: WritingSequence,
    pub stage: WritingStage,
    pub round_id: Uuid,
    pub attempt_id: Uuid,
    pub scope_id: String,
    pub original: Option<WritingOriginalRef>,
    pub audio_completed: bool,
    pub model_revealed: bool,
    #[serde(default)]
    pub pre_submission_cues: Option<Vec<String>>,
    #[serde(default)]
    pub entry_previewed: bool,
    #[serde(default)]
    pub return_intent: Option<WritingReturnIntent>,
    pub say: SayDisposition,
    pub history: Vec<WritingRound>,
    #[serde(default)]
    pub native_instance: Option<Uuid>,
}
fn paper_for(task: &FoundationHandwritingTask) -> InkPaper {
    let weight: f64 = task
        .target
        .chars()
        .map(|c| {
            if c == ' ' {
                0.45
            } else if c == '.' {
                0.28
            } else {
                0.84
            }
        })
        .sum();
    match task.layout {
        FoundationHandwritingLayout::LetterBox => InkPaper {
            width: 400,
            height: 300,
            model_size: 190,
            baseline: 277,
            layout: "letter_box".into(),
        },
        FoundationHandwritingLayout::WordLine => InkPaper {
            width: 400.max((weight * 125.0 + 80.0).ceil() as u32),
            height: if task.source_pack_revision == "foundation-2026-11-06.3" {
                300
            } else {
                260
            },
            model_size: 125,
            baseline: 225,
            layout: "word_line".into(),
        },
        FoundationHandwritingLayout::PhraseLine => InkPaper {
            width: 640.max((weight * 120.0 + 80.0).ceil() as u32),
            height: if task.source_pack_revision == "foundation-2026-11-06.3" {
                300
            } else {
                260
            },
            model_size: 120,
            baseline: 225,
            layout: "phrase_line".into(),
        },
    }
}
impl HandwritingActivity {
    pub(crate) fn scope(&self) -> String {
        self.scope_id.clone()
    }
    pub(crate) fn definition(&self) -> Result<FoundationHandwritingTask> {
        let task = handwriting_tasks()
            .into_iter()
            .find(|t| t.id == self.task_id && t.revision == self.task_revision)
            .ok_or(
                "The authored writing activity changed. Your original drawings are preserved.",
            )?;
        Ok(task)
    }
    pub(crate) fn task(&self) -> HandwritingTask {
        let t = self
            .definition()
            .expect("validated authored writing activity");
        let copy = self.sequence == WritingSequence::SeeCopyHearSay;
        let instruction = if copy {
            "Copy the example.".into()
        } else if t.input_focus == FoundationHandwritingInputFocus::InitialSegmentOfSyllable {
            "Listen, then write the capital letter for the first sound.".into()
        } else {
            match t.layout {
                FoundationHandwritingLayout::LetterBox => {
                    "Listen, then write the capital letter.".into()
                }
                FoundationHandwritingLayout::WordLine => {
                    "Listen, then write the word in lowercase.".into()
                }
                FoundationHandwritingLayout::PhraseLine => {
                    "Listen, then write the message, including spaces and punctuation.".into()
                }
            }
        };
        HandwritingTask {
            model_version: if t.source_pack_revision == "foundation-2026-11-06.3" {
                ALPHABET_MODEL
            } else {
                MODEL
            }
            .into(),
            id: format!("{}-{}", t.id, if copy { "copy" } else { "sound" }),
            revision: t.revision.clone(),
            target: t.target.clone(),
            instruction,
            meaning: t.meaning.clone(),
            reference_audio_id: Some(t.audio_id.clone()),
            audio_role: if copy {
                InkAudioRole::Model
            } else {
                InkAudioRole::TaskInput
            },
            formation_hint: format!("{} {}", t.formation_hint, t.script_note),
            paper: paper_for(&t),
        }
    }
    fn validate(&self) -> Result<()> {
        let task = self.definition()?;
        if self.version != 2
            || self.id.is_nil()
            || self.course_session.is_nil()
            || self.round_id.is_nil()
            || self.attempt_id.is_nil()
            || self.scope_id.is_empty()
            || !task.sequences.contains(&self.sequence)
        {
            return Err("This writing run no longer matches its authored task. Your drawings are preserved.".into());
        }
        let mut round_ids = std::collections::BTreeSet::from([self.round_id]);
        for round in &self.history {
            if round.round_id.is_nil()
                || !round_ids.insert(round.round_id)
                || round.scope_id.is_empty()
                || !task.sequences.contains(&round.sequence)
                || round.attempt_id.is_some_and(|id| id.is_nil())
            {
                return Err(
                    "A saved writing round has a different identity. Its original is preserved."
                        .into(),
                );
            }
        }
        for (scope, sequence, original) in self
            .history
            .iter()
            .map(|r| (r.scope_id.as_str(), r.sequence, r.original.as_ref()))
            .chain(std::iter::once((
                self.scope_id.as_str(),
                self.sequence,
                self.original.as_ref(),
            )))
        {
            if let Some(original) = original {
                let suffix = if sequence == WritingSequence::SeeCopyHearSay {
                    "copy"
                } else {
                    "sound"
                };
                if original.id.is_nil()
                    || original.scope_id != scope
                    || original.task_id != format!("{}-{suffix}", self.task_id)
                    || original.task_revision != self.task_revision
                {
                    return Err("A saved original belongs to another writing round. Its identity is preserved.".into());
                }
            }
        }
        Ok(())
    }
}
fn activity_key(id: Uuid) -> String {
    format!("handwriting.activity:{id}")
}
fn course_key(course: Uuid, task: &str, sequence: WritingSequence) -> String {
    format!(
        "handwriting.course:{course}:{task}:{}",
        if sequence == WritingSequence::SeeCopyHearSay {
            "copy"
        } else {
            "sound"
        }
    )
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl AppState {
    pub(crate) fn writing_load(&self, id: Uuid) -> Result<HandwritingActivity> {
        let run = self
            .repo
            .get::<HandwritingActivity>(&activity_key(id))?
            .ok_or("The saved writing run is unavailable. Its original drawings are preserved.")?;
        run.validate()?;
        if run.id != id {
            return Err(
                "The saved writing run has a different identity. Its original is preserved.".into(),
            );
        }
        Ok(run)
    }
    fn writing_prefer_run(&self, run: &HandwritingActivity) -> Result<()> {
        self.repo.set(
            &course_key(run.course_session, &run.task_id, run.sequence),
            &run.id,
        )
    }
    pub(crate) fn handwriting_for_course(
        &self,
        course: Uuid,
    ) -> Result<Option<HandwritingActivity>> {
        let state = self.foundation_state()?;
        let Some(task) = handwriting_task_for_step(&state.step_id) else {
            return Ok(None);
        };
        if let Ok(active) = self.active_handwriting() {
            if active.course_session == course && active.task_id == task.id {
                return Ok(Some(active));
            }
        }
        // Legacy course/sequence pointers name a run, not its changing current round.
        // Discover canonical activities so a stale or interrupted index cannot hide ink.
        Ok(self
            .handwriting_history()?
            .into_iter()
            .rev()
            .find(|run| run.course_session == course && run.task_id == task.id))
    }
    /// A checkpoint has its own deterministic run, so catalog visits cannot steal
    /// its partial drawing and a crash between pointer writes cannot duplicate it.
    pub(crate) fn writing_checkpoint_run(
        &self,
        checkpoint: &crate::foundation_writing::FoundationWritingCheckpoint,
    ) -> Result<Option<HandwritingActivity>> {
        let id = checkpoint.run_id();
        let Some(run) = self.repo.get::<HandwritingActivity>(&activity_key(id))? else {
            return Ok(None);
        };
        run.validate()?;
        if run.id != id
            || run.course_session != checkpoint.item_session()
            || run.course_step != checkpoint.anchor_step
            || run.course_step_revision != checkpoint.anchor_revision
            || run.task_id != checkpoint.item().task_id
            || run.task_revision != checkpoint.item().task_revision
        {
            return Err(
                "The saved checkpoint paper has another identity. Your drawings are kept.".into(),
            );
        }
        Ok(Some(run))
    }
    fn prepare_foundation_writing_checkpoint(
        &self,
        checkpoint: &crate::foundation_writing::FoundationWritingCheckpoint,
    ) -> Result<Option<HandwritingActivity>> {
        let course = self.foundation_state()?;
        checkpoint.validate(&course)?;
        let task = handwriting_tasks()
            .into_iter()
            .find(|t| {
                t.id == checkpoint.item().task_id && t.revision == checkpoint.item().task_revision
            })
            .ok_or("The checkpoint's writing model is unavailable.")?;
        let mut run = if let Some(run) = self.writing_checkpoint_run(checkpoint)? {
            run
        } else {
            let id = checkpoint.run_id();
            let round_id = Uuid::new_v5(&id, b"initial-round");
            HandwritingActivity {
                version: 2,
                id,
                course_session: checkpoint.item_session(),
                course_step: checkpoint.anchor_step.clone(),
                course_step_revision: checkpoint.anchor_revision.clone(),
                task_id: task.id,
                task_revision: task.revision,
                sequence: WritingSequence::HearWriteCompareSay,
                stage: WritingStage::Writing,
                round_id,
                attempt_id: Uuid::new_v5(&round_id, b"initial-attempt"),
                scope_id: format!("{id}:{round_id}"),
                original: None,
                audio_completed: false,
                model_revealed: false,
                pre_submission_cues: Some(vec![]),
                entry_previewed: false,
                return_intent: None,
                say: SayDisposition::NotReported,
                history: vec![],
                native_instance: None,
            }
        };
        if run.stage == WritingStage::Writing
            && run.original.is_none()
            && self
                .repo
                .get::<bool>(&format!(
                    "foundation.writing.reference:{}",
                    checkpoint.item_session()
                ))?
                .unwrap_or(false)
        {
            if let Some(cues) = &mut run.pre_submission_cues {
                if !cues.iter().any(|cue| cue == "reference_material") {
                    cues.push("reference_material".into());
                }
            }
        }
        run.return_intent = Some(WritingReturnIntent::FoundationCheckpoint {
            checkpoint: checkpoint.id,
            item_session: checkpoint.item_session(),
            anchor_session: checkpoint.anchor_session,
            step: checkpoint.anchor_step.clone(),
            revision: checkpoint.anchor_revision.clone(),
        });
        self.repo.set(&activity_key(run.id), &run)?;
        self.writing_prefer_run(&run)?;
        self.repo.set("handwriting.active", &run.id)?;
        self.repo.set("handwriting.surface", &"run")?;
        Ok(Some(run))
    }
    pub(crate) fn active_handwriting(&self) -> Result<HandwritingActivity> {
        let id = self
            .repo
            .get::<Uuid>("handwriting.active")?
            .ok_or("Choose a writing activity from a taught word or letter.")?;
        self.writing_load(id)
    }
    pub(crate) fn handwriting_task_available(
        &self,
        task: &FoundationHandwritingTask,
    ) -> Result<bool> {
        let course = self.foundation_state()?;
        let source = task.foundation_step_id == course.step_id
            && rufren_content::foundations().episodes[course.episode].steps[course.step]
                .audio_ids
                .iter()
                .all(|id| course.played.contains(id));
        Ok(source
            || course
                .encounters
                .contains(&format!("{}:encountered", task.foundation_step_id)))
    }
    pub(crate) fn open_handwriting(&mut self) -> Result<()> {
        if self.page == Page::Foundations && self.prepare_foundation_writing()?.is_some() {
            return Ok(());
        }
        let course = self.foundation_state()?;
        let task = handwriting_task_for_step(&course.step_id)
            .ok_or("This encounter has no authored writing activity yet.")?;
        if let Some(run) = self.handwriting_for_course(course.session)? {
            if run.stage != WritingStage::Complete {
                self.repo.set("handwriting.active", &run.id)?;
                return self.resume_handwriting();
            }
        }
        self.open_writing_task(&task.id, task.sequence)
    }
    pub(crate) fn foundation_writing(&self) -> Result<Option<HandwritingActivity>> {
        let course = self.foundation_state()?;
        if let Some(checkpoint) = &course.writing_checkpoint {
            return Ok(self
                .writing_checkpoint_run(checkpoint)?
                .filter(|run| self.writing_returns_to_foundations(run).unwrap_or(false)));
        }
        if course.review_resume.is_some() {
            return Ok(None);
        }
        if !matches!(
            rufren_content::foundations().episodes[course.episode].steps[course.step].action,
            rufren_content::FoundationAction::Present { .. }
        ) {
            return Ok(None);
        }
        let Some(task) = handwriting_task_for_step(&course.step_id) else {
            return Ok(None);
        };
        Ok(self.handwriting_for_course(course.session)?.filter(|run| {
            run.task_revision == task.revision
                && self.writing_returns_to_foundations(run).unwrap_or(false)
                && run.sequence == WritingSequence::SeeCopyHearSay
        }))
    }
    /// Prepare only the authored current teaching model for copying, before it is heard.
    /// This binds navigation intent without changing the run's original provenance.
    pub(crate) fn prepare_foundation_writing(&mut self) -> Result<Option<HandwritingActivity>> {
        if self.foundation_recover_writing_transition()? {
            return Ok(None);
        }
        let course = self.foundation_state()?;
        if let Some(checkpoint) = &course.writing_checkpoint {
            return self.prepare_foundation_writing_checkpoint(checkpoint);
        }
        if course.review_resume.is_some() {
            return Ok(None);
        }
        let step = &rufren_content::foundations().episodes[course.episode].steps[course.step];
        if !matches!(
            step.action,
            rufren_content::FoundationAction::Present { .. }
        ) {
            return Ok(None);
        }
        let Some(task) = handwriting_task_for_step(&course.step_id) else {
            return Ok(None);
        };
        self.open_writing_task_inner(&task.id, WritingSequence::SeeCopyHearSay, true)?;
        self.foundation_writing()
    }
    fn writing_returns_to_foundations(&self, run: &HandwritingActivity) -> Result<bool> {
        let course = self.foundation_state()?;
        if let Some(WritingReturnIntent::FoundationCheckpoint {
            checkpoint,
            item_session,
            anchor_session,
            step,
            revision,
        }) = &run.return_intent
        {
            return Ok(course.writing_checkpoint.as_ref().is_some_and(|c| {
                c.id == *checkpoint
                    && c.item_session() == *item_session
                    && c.run_id() == run.id
                    && course.session == *anchor_session
                    && course.step_id == *step
                    && course.step_revision == *revision
                    && c.item().task_id == run.task_id
                    && c.item().task_revision == run.task_revision
            }));
        }
        let task = run.definition()?;
        if task.foundation_step_id != course.step_id
            || task.foundation_step_revision != course.step_revision
        {
            return Ok(false);
        }
        Ok(match &run.return_intent {
            Some(WritingReturnIntent::Foundations {
                session,
                step,
                revision,
            }) => {
                *session == course.session
                    && *step == course.step_id
                    && *revision == course.step_revision
            }
            Some(
                WritingReturnIntent::Catalog | WritingReturnIntent::FoundationCheckpoint { .. },
            ) => false,
            None => {
                !run.entry_previewed
                    && course.session == run.course_session
                    && course.step_id == run.course_step
                    && course.step_revision == run.course_step_revision
            }
        })
    }
    pub(crate) fn open_writing_task(
        &mut self,
        task_id: &str,
        sequence: WritingSequence,
    ) -> Result<()> {
        self.open_writing_task_inner(task_id, sequence, false)
    }
    fn open_writing_task_inner(
        &mut self,
        task_id: &str,
        sequence: WritingSequence,
        inline: bool,
    ) -> Result<()> {
        let task = handwriting_tasks()
            .into_iter()
            .find(|t| t.id == task_id)
            .ok_or("That writing activity is unavailable.")?;
        if !inline && !self.handwriting_task_available(&task)? {
            return Err("Meet this word or letter with its lesson model first. Your current course place is kept.".into());
        }
        if !task.sequences.contains(&sequence) {
            return Err("This writing route is not authored for the target.".into());
        }
        self.writing_reference_use()?;
        let course = self.foundation_state()?;
        let key = course_key(course.session, &task.id, sequence);
        let runs = self.handwriting_history()?;
        let indexed = self
            .repo
            .get::<Uuid>(&key)?
            .map(|id| self.writing_load(id))
            .transpose()?;
        if indexed.as_ref().is_some_and(|run| {
            run.course_session != course.session
                || run.task_id != task.id
                || run.task_revision != task.revision
        }) {
            return Err(
                "The saved writing route names another course or task. Your drawing is kept."
                    .into(),
            );
        }
        let matching = |run: &HandwritingActivity| {
            run.course_session == course.session
                && run.task_id == task.id
                && run.task_revision == task.revision
                && run.sequence == sequence
        };
        // An explicit active/resumed run wins over the preferred route pointer.
        // Canonical order is only recovery fallback, never a recency claim.
        let active = self.active_handwriting().ok();
        let existing = active
            .as_ref()
            .filter(|run| matching(run) && run.stage != WritingStage::Complete)
            .or_else(|| indexed.as_ref().filter(|run| matching(run)))
            .or_else(|| {
                runs.iter()
                    .find(|run| matching(run) && run.stage != WritingStage::Complete)
            })
            .or_else(|| runs.iter().filter(|run| matching(run)).last())
            // A legacy pointer can name a completed run whose current round
            // changed route. Keep its earlier fresh-round reentry behavior.
            .or_else(|| {
                indexed
                    .as_ref()
                    .filter(|run| run.stage == WritingStage::Complete)
            })
            .cloned();
        let run = if let Some(run) = existing {
            run
        } else {
            let id = Uuid::new_v4();
            let round = Uuid::new_v4();
            // A copy exercise continues the model heard in this exact lesson
            // encounter. Dictation still needs its own fresh task input, and
            // catalog practice and subsequent rounds start independently.
            let heard_course_model = sequence == WritingSequence::SeeCopyHearSay
                && self.page == Page::Foundations
                && course.step_id == task.foundation_step_id
                && course.step_revision == task.foundation_step_revision
                && course.played.contains(&task.audio_id);
            let run = HandwritingActivity {
                version: 2,
                id,
                course_session: course.session,
                course_step: course.step_id.clone(),
                course_step_revision: course.step_revision.clone(),
                task_id: task.id.clone(),
                task_revision: task.revision.clone(),
                sequence,
                stage: WritingStage::Writing,
                round_id: round,
                attempt_id: Uuid::new_v4(),
                scope_id: format!("{id}:{round}"),
                original: None,
                audio_completed: heard_course_model,
                model_revealed: sequence == WritingSequence::SeeCopyHearSay,
                pre_submission_cues: Some(if sequence == WritingSequence::SeeCopyHearSay {
                    vec!["visual_model".into()]
                } else {
                    vec![]
                }),
                entry_previewed: self.page == Page::Handwriting
                    && self.repo.get::<String>("handwriting.surface")?.as_deref()
                        == Some("catalog"),
                return_intent: None,
                say: SayDisposition::NotReported,
                history: vec![],
                native_instance: None,
            };
            // The activity is the canonical history record. A crash after this
            // write and before either pointer is recoverable by enumeration.
            self.repo.set(&activity_key(id), &run)?;
            run
        };
        let mut run = run;
        run.return_intent = Some(if inline || self.page == Page::Foundations {
            WritingReturnIntent::Foundations {
                session: course.session,
                step: course.step_id.clone(),
                revision: course.step_revision.clone(),
            }
        } else {
            WritingReturnIntent::Catalog
        });
        if run.stage == WritingStage::Writing
            && run.original.is_none()
            && sequence == WritingSequence::SeeCopyHearSay
            && self.page == Page::Foundations
            && course.step_id == task.foundation_step_id
            && course.step_revision == task.foundation_step_revision
            && course.played.contains(&task.audio_id)
        {
            run.audio_completed = true;
        }
        self.repo.set(&activity_key(run.id), &run)?;
        self.repo.set(&key, &run.id)?;
        self.repo.set("handwriting.active", &run.id)?;
        if inline {
            self.repo.set("handwriting.surface", &"run")?;
            return Ok(());
        }
        if run.stage == WritingStage::Complete {
            self.writing_new_round(run.id, run.round_id, sequence)?
        }
        self.repo.set("handwriting.surface", &"run")?;
        self.navigate(Page::Handwriting)
    }
    pub(crate) fn writing_audio_completed(&self, id: Uuid, round: Uuid, asset: &str) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if run.id != id || run.round_id != round || run.definition()?.audio_id != asset {
            return Err("The spoken example belongs to another writing attempt.".into());
        }
        run.audio_completed = true;
        self.repo.set(&activity_key(id), &run)?;
        if self.writing_returns_to_foundations(&run)?
            && !matches!(
                run.return_intent,
                Some(WritingReturnIntent::FoundationCheckpoint { .. })
            )
        {
            self.foundation_replayed(asset)?;
        }
        Ok(())
    }
    pub(crate) fn writing_model_revealed(&self, id: Uuid, round: Uuid) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if run.id != id || run.round_id != round {
            return Err("The writing attempt changed.".into());
        }
        run.model_revealed = true;
        if run.stage == WritingStage::Writing && run.original.is_none() {
            if let Some(cues) = &mut run.pre_submission_cues {
                if !cues.iter().any(|cue| cue == "visual_model") {
                    cues.push("visual_model".into());
                }
            }
        }
        self.repo.set(&activity_key(id), &run)
    }
    pub(crate) fn writing_check_original(
        &self,
        id: Uuid,
        round: Uuid,
        original: &WritingOriginalRef,
    ) -> Result<()> {
        let run = self.active_handwriting()?;
        if let Some(WritingReturnIntent::FoundationCheckpoint { item_session, .. }) =
            &run.return_intent
        {
            if self.foundation_writing_transition_pending(*item_session)? {
                return Err("This writing turn is already saved. Continue the lesson to open its next paper.".into());
            }
        }
        if run.id != id
            || run.round_id != round
            || original.scope_id != run.scope()
            || original.task_id != run.task().id
            || original.task_revision != run.task_revision
            || !matches!(run.stage, WritingStage::Writing | WritingStage::Compare)
        {
            return Err(
                "The drawing belongs to another writing attempt. Its original is preserved.".into(),
            );
        }
        if run.sequence == WritingSequence::HearWriteCompareSay && !run.audio_completed {
            return Err(
                "Listen to the complete spoken prompt before comparing this drawing.".into(),
            );
        }
        Ok(())
    }
    pub(crate) fn writing_accept_original(
        &self,
        id: Uuid,
        round: Uuid,
        original: WritingOriginalRef,
    ) -> Result<()> {
        self.writing_check_original(id, round, &original)?;
        let mut run = self.active_handwriting()?;
        if run.id != id
            || run.round_id != round
            || original.scope_id != run.scope()
            || original.task_id != run.task().id
            || original.task_revision != run.task_revision
            || !matches!(run.stage, WritingStage::Writing | WritingStage::Compare)
        {
            return Err(
                "The drawing belongs to a different writing attempt. Its original is preserved."
                    .into(),
            );
        }
        if run.sequence == WritingSequence::HearWriteCompareSay && !run.audio_completed {
            return Err(
                "Listen to the complete spoken prompt before comparing this drawing.".into(),
            );
        }
        // Seal assistance from the actual original before feedback reveals the answer.
        // Legacy native archives do not establish an unassisted attempt.
        if run.original.is_none() {
            let artifact = self
                .repo
                .get::<serde_json::Value>(&original.local_key)?
                .ok_or("The saved drawing is unavailable. Its reference is kept.")?;
            let payload = artifact.get("payload").unwrap_or(&artifact);
            if original.format == "web-pointer-ink-v2" {
                if let Some(cues) = &mut run.pre_submission_cues {
                    if let Some(aids) = payload.get("aids").and_then(|v| v.as_array()) {
                        for aid in aids {
                            let kind = aid.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                            if matches!(kind, "visual_model" | "meaning" | "stroke_model_replay")
                                && !cues.iter().any(|cue| cue == kind)
                            {
                                cues.push(kind.into());
                            }
                        }
                    } else {
                        run.pre_submission_cues = None;
                    }
                }
            } else {
                let help = payload.get("model_help");
                if help
                    .and_then(|h| h.get("history_from_start"))
                    .and_then(|v| v.as_bool())
                    != Some(true)
                {
                    run.pre_submission_cues = None;
                } else if help
                    .and_then(|h| h.get("visible_at_start"))
                    .and_then(|v| v.as_bool())
                    == Some(true)
                    || help
                        .and_then(|h| h.get("revealed_at_ms"))
                        .and_then(|v| v.as_array())
                        .is_some_and(|v| !v.is_empty())
                {
                    if let Some(cues) = &mut run.pre_submission_cues {
                        if !cues.iter().any(|cue| cue == "visual_model") {
                            cues.push("visual_model".into());
                        }
                    }
                }
            }
        }
        run.original = Some(original);
        run.stage = WritingStage::Compare;
        self.repo.set(&activity_key(id), &run)
    }
    pub(crate) fn writing_store_artifact(
        &self,
        id: Uuid,
        scope: &str,
        task_id: &str,
        revision: &str,
        format: &str,
        payload: serde_json::Value,
    ) -> Result<WritingOriginalRef> {
        let bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
        let sha256 = hash(&bytes);
        let local_key = format!("handwriting.artifact:{id}");
        let stored = serde_json::json!({"version":1,"id":id,"scope_id":scope,"task_id":task_id,"task_revision":revision,"format":format,"sha256":sha256,"payload":payload});
        if let Some(existing) = self.repo.get::<serde_json::Value>(&local_key)? {
            if existing != stored {
                return Err(
                    "A different original uses this drawing identity. It is preserved.".into(),
                );
            }
        } else {
            self.repo.set(&local_key, &stored)?
        }
        Ok(WritingOriginalRef {
            id,
            scope_id: scope.into(),
            task_id: task_id.into(),
            task_revision: revision.into(),
            format: format.into(),
            sha256,
            local_key,
        })
    }
    pub(crate) fn writing_accept_web(
        &self,
        id: Uuid,
        round: Uuid,
        original: &HandwritingSubmission,
    ) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if run.id != id || run.round_id != round {
            return Err("The writing attempt changed. Its original is preserved.".into());
        }
        if original.attempt_id != run.attempt_id {
            return Err(
                "The saved ink belongs to a different attempt. Its original is preserved.".into(),
            );
        }
        original.validate(&run.scope(), &run.task(), true)?;
        let reference = self.writing_store_artifact(
            original.attempt_id,
            &run.scope(),
            &run.task().id,
            &run.task_revision,
            "web-pointer-ink-v2",
            serde_json::to_value(original).map_err(|e| e.to_string())?,
        )?;
        if original.aids.iter().any(|a| {
            matches!(
                a.kind.as_str(),
                "sound_task_input_completed" | "sound_model_completed"
            ) && Some(&a.value) == original.audio_id.as_ref()
        }) {
            run.audio_completed = true;
            self.repo.set(&activity_key(id), &run)?
        }
        self.writing_accept_original(id, round, reference)
    }
    pub(crate) fn writing_go_say(&self, id: Uuid, round: Uuid) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if run.id != id
            || run.round_id != round
            || run.stage != WritingStage::Compare
            || run.original.is_none()
        {
            return Err("Compare the saved drawing before continuing to speaking.".into());
        }
        run.stage = WritingStage::Say;
        self.repo.set(&activity_key(id), &run)
    }
    pub(crate) fn writing_say_done(
        &self,
        id: Uuid,
        round: Uuid,
        disposition: SayDisposition,
    ) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if run.id != id
            || run.round_id != round
            || run.stage != WritingStage::Say
            || disposition == SayDisposition::NotReported
        {
            return Err("This speaking turn is not ready to finish.".into());
        }
        if disposition == SayDisposition::SelfReportedSaid && !run.audio_completed {
            return Err(
                "Listen to the spoken model, then try saying it. You can also skip speaking."
                    .into(),
            );
        }
        run.say = disposition;
        run.stage = WritingStage::Complete;
        self.repo.set(&activity_key(id), &run)
    }
    pub(crate) fn writing_new_round(
        &self,
        id: Uuid,
        round: Uuid,
        sequence: WritingSequence,
    ) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if let Some(WritingReturnIntent::FoundationCheckpoint { item_session, .. }) =
            &run.return_intent
        {
            if self.foundation_writing_transition_pending(*item_session)? {
                return Err(
                    "This turn is already saved. Continue it before starting another.".into(),
                );
            }
        }
        if run.id != id
            || run.round_id != round
            || !matches!(
                run.stage,
                WritingStage::Compare | WritingStage::Say | WritingStage::Complete
            )
        {
            return Err("Finish comparing this drawing before starting a fresh attempt.".into());
        }
        if !run.definition()?.sequences.contains(&sequence) {
            return Err("That route is not authored for this target.".into());
        }
        run.history.push(WritingRound {
            round_id: run.round_id,
            scope_id: run.scope(),
            sequence: run.sequence,
            original: run.original.clone(),
            say: run.say,
            stage: Some(run.stage),
            attempt_id: Some(run.attempt_id),
            audio_completed: Some(run.audio_completed),
            pre_submission_cues: run.pre_submission_cues.clone(),
        });
        run.round_id = Uuid::new_v4();
        run.attempt_id = Uuid::new_v4();
        run.scope_id = format!("{}:{}", run.id, run.round_id);
        run.sequence = sequence;
        run.stage = WritingStage::Writing;
        run.original = None;
        run.audio_completed = false;
        run.model_revealed = sequence == WritingSequence::SeeCopyHearSay;
        run.pre_submission_cues = Some(if sequence == WritingSequence::SeeCopyHearSay {
            vec!["visual_model".into(), "after_comparison".into()]
        } else {
            vec!["after_comparison".into()]
        });
        run.entry_previewed = true;
        run.say = SayDisposition::NotReported;
        run.native_instance = None;
        self.repo.set(&activity_key(id), &run)?;
        // Preference is a recoverable pointer; the complete canonical round is
        // durable before this write, and other unfinished runs stay in history.
        self.writing_prefer_run(&run)
    }

    /// Finish the exercise in place and return directly to its next useful task.
    /// Retrying navigation after a saved completion must not advance twice.
    pub(crate) fn writing_finish_and_continue(
        &mut self,
        id: Uuid,
        round: Uuid,
        disposition: SayDisposition,
    ) -> Result<()> {
        let run = self.active_handwriting()?;
        if run.id != id || run.round_id != round {
            return Err("This writing activity changed. Your work is saved.".into());
        }
        if run.stage == WritingStage::Compare {
            self.writing_go_say(id, round)?;
        }
        if run.stage != WritingStage::Complete {
            self.writing_say_done(id, round, disposition)?;
        }
        if matches!(
            run.return_intent,
            Some(WritingReturnIntent::FoundationCheckpoint { .. })
        ) {
            let completed = self.writing_load(id)?;
            self.foundation_finish_writing_checkpoint(&completed)?;
            return self.navigate(Page::Foundations);
        }
        if matches!(
            run.return_intent,
            Some(WritingReturnIntent::Foundations { .. })
        ) || (run.return_intent.is_none() && !run.entry_previewed)
        {
            let course = self.foundation_state()?;
            if self.writing_returns_to_foundations(&run)?
                && run.definition()?.foundation_step_id == course.step_id
                && run.definition()?.foundation_step_revision == course.step_revision
            {
                self.foundation_next(false)?;
            }
            self.navigate(Page::Foundations)
        } else {
            self.open_writing_catalog()
        }
    }
    pub(crate) fn handwriting_originals(
        &self,
        scope: &str,
        task: &HandwritingTask,
    ) -> Result<Vec<HandwritingSubmission>> {
        // The index is only an acceleration hint. A sealed original may have
        // reached disk immediately before an interrupted index write.
        let mut originals = Vec::new();
        for (key, value) in self
            .repo
            .metadata_with_scope("handwriting.original:", scope)?
        {
            let original: HandwritingSubmission =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            if original.scope_id != scope {
                continue;
            }
            original.validate(scope, task, true)?;
            if key != original_key(original.attempt_id) {
                return Err(
                    "A saved drawing has a different storage identity. Its original is preserved."
                        .into(),
                );
            }
            originals.push(original)
        }
        // Keep detecting a genuinely missing indexed original rather than
        // silently presenting a complete-looking partial history.
        let ids: Vec<Uuid> = self
            .repo
            .get(&format!("handwriting.originals:{scope}"))?
            .unwrap_or_default();
        if ids
            .iter()
            .any(|id| !originals.iter().any(|o| o.attempt_id == *id))
        {
            return Err("A saved drawing is unavailable. Its index is kept.".into());
        }
        Ok(originals)
    }
    /// Canonical local runs, including legacy runs whose active/course index
    /// was superseded or whose pointer write was interrupted. No migration writes
    /// are necessary: the installed v2 activity metadata is already the record.
    pub(crate) fn handwriting_history(&self) -> Result<Vec<HandwritingActivity>> {
        let mut runs = Vec::new();
        for (key, value) in self.repo.metadata_with_prefix("handwriting.activity:")? {
            let run: HandwritingActivity =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            run.validate()?;
            if key != activity_key(run.id) {
                return Err("A saved writing run has a different storage identity. Its original is preserved.".into());
            }
            runs.push(run);
        }
        Ok(runs)
    }
    /// Explicit history reentry resumes the exact run, including a completed run,
    /// without starting a fresh attempt or changing the course's saved cursor.
    pub(crate) fn open_handwriting_history(&mut self, id: Uuid) -> Result<()> {
        let mut run = self.writing_load(id)?;
        run.return_intent = Some(WritingReturnIntent::Catalog);
        self.repo.set(&activity_key(id), &run)?;
        self.writing_reference_use()?;
        self.writing_prefer_run(&run)?;
        self.repo.set("handwriting.active", &id)?;
        self.resume_handwriting()
    }
    pub(crate) fn handwriting_round_review(
        &self,
        id: Uuid,
        round_id: Uuid,
    ) -> Result<WritingRoundReview> {
        let run = self.writing_load(id)?;
        let (scope, sequence, stage, say, original) = if run.round_id == round_id {
            (
                run.scope(),
                run.sequence,
                Some(run.stage),
                run.say,
                run.original.clone(),
            )
        } else {
            let round = run
                .history
                .iter()
                .find(|r| r.round_id == round_id)
                .ok_or("That writing round is unavailable. Your current drawing is kept.")?;
            (
                round.scope_id.clone(),
                round.sequence,
                round.stage,
                round.say,
                round.original.clone(),
            )
        };
        // Use the archived round's route when reconstructing its authored task;
        // the current run may already have moved from copy to dictation.
        let mut projection = run.clone();
        projection.sequence = sequence;
        let task = projection.task();
        let mut originals = Vec::new();
        for (key, artifact) in self
            .repo
            .metadata_with_scope("handwriting.artifact:", &scope)?
        {
            if artifact["scope_id"].as_str() != Some(&scope) {
                continue;
            }
            let artifact_id: Uuid =
                serde_json::from_value(artifact["id"].clone()).map_err(|e| e.to_string())?;
            let reference = self.writing_artifact_ref(artifact_id)?;
            if key != reference.local_key
                || reference.scope_id != scope
                || reference.task_id != task.id
                || reference.task_revision != task.revision
            {
                return Err("A drawing's task identity differs from its saved round. Its original is preserved.".into());
            }
            originals.push(WritingOriginalReview {
                reference,
                payload: artifact["payload"].clone(),
            });
        }
        // Raw web originals are independently durable. Include the narrow
        // interruption window before their generic artifact was written.
        for ink in self.handwriting_originals(&scope, &task)? {
            let payload = serde_json::to_value(&ink).map_err(|e| e.to_string())?;
            if let Some(stored) = originals.iter().find(|o| o.reference.id == ink.attempt_id) {
                if stored.payload != payload {
                    return Err("Two saved originals use the same drawing identity with different ink. Both are preserved.".into());
                }
                continue;
            }
            let reference = WritingOriginalRef {
                id: ink.attempt_id,
                scope_id: scope.clone(),
                task_id: task.id.clone(),
                task_revision: task.revision.clone(),
                format: "web-pointer-ink-v2".into(),
                sha256: hash(&serde_json::to_vec(&payload).map_err(|e| e.to_string())?),
                local_key: original_key(ink.attempt_id),
            };
            originals.push(WritingOriginalReview { reference, payload });
        }
        if let Some(reference) = &original {
            if !originals.iter().any(|o| o.reference == *reference) {
                return Err("The round's saved original is unavailable or changed. Its reference is preserved.".into());
            }
        }
        Ok(WritingRoundReview {
            run_id: id,
            round_id,
            scope_id: scope,
            sequence,
            task,
            stage,
            say,
            original,
            originals,
        })
    }
    pub(crate) fn handwriting_course_complete(&self, course: Uuid) -> bool {
        self.handwriting_for_course(course)
            .ok()
            .flatten()
            .is_some_and(|run| run.stage == WritingStage::Complete)
    }
    pub(crate) fn handwriting_resume_available(&self) -> bool {
        self.active_handwriting().ok().is_some_and(|run| {
            run.stage != WritingStage::Complete
                && self
                    .foundation_state()
                    .ok()
                    .is_some_and(|s| s.session == run.course_session)
        })
    }
    pub(crate) fn resume_handwriting(&mut self) -> Result<()> {
        let run = self.active_handwriting()?;
        if self.writing_returns_to_foundations(&run)? {
            return self.navigate(Page::Foundations);
        }
        self.repo.set("handwriting.surface", &"run")?;
        self.navigate(Page::Handwriting)
    }
    pub(crate) fn handwriting_export(&self) -> Result<serde_json::Value> {
        self.handwriting_export_run(self.active_handwriting()?.id)
    }
    /// Export an exact saved run without changing the current activity. Review
    /// supplies the same recovered originals shown in history, including saves
    /// interrupted before their index or accepted-reference write.
    pub(crate) fn handwriting_export_run(&self, id: Uuid) -> Result<serde_json::Value> {
        let run = self.writing_load(id)?;
        let mut originals = Vec::new();
        for round_id in run
            .history
            .iter()
            .map(|r| r.round_id)
            .chain(std::iter::once(run.round_id))
        {
            let review = self.handwriting_round_review(id, round_id)?;
            for original in review.originals {
                let reference = original.reference;
                originals.push(serde_json::json!({
                    "version": 1,
                    "id": reference.id,
                    "scope_id": reference.scope_id,
                    "task_id": reference.task_id,
                    "task_revision": reference.task_revision,
                    "format": reference.format,
                    "sha256": reference.sha256,
                    "payload": original.payload,
                }));
            }
        }
        let mut native_latest = Vec::new();
        for scope in run
            .history
            .iter()
            .map(|r| r.scope_id.clone())
            .chain(std::iter::once(run.scope()))
        {
            if let Some(checkpoint) = self
                .repo
                .get::<serde_json::Value>(&format!("handwriting.native.latest:{scope}"))?
            {
                native_latest.push(checkpoint)
            }
        }
        Ok(
            serde_json::json!({"format":"drussky-writing-originals","version":1,"task":run.definition()?,"run":run,"originals":originals,"latest_native_drafts":native_latest}),
        )
    }
}
impl AppState {
    pub(crate) fn writing_artifact_ref(&self, id: Uuid) -> Result<WritingOriginalRef> {
        let key = format!("handwriting.artifact:{id}");
        let artifact = self
            .repo
            .get::<serde_json::Value>(&key)?
            .ok_or("The saved drawing is unavailable. Its reference is kept.")?;
        if artifact["id"] != serde_json::json!(id)
            || artifact["version"] != 1
            || artifact["sha256"].as_str()
                != Some(
                    hash(&serde_json::to_vec(&artifact["payload"]).map_err(|e| e.to_string())?)
                        .as_str(),
                )
        {
            return Err(
                "The drawing's saved bytes do not match its identity. Its original is preserved."
                    .into(),
            );
        }
        Ok(WritingOriginalRef {
            id,
            scope_id: artifact["scope_id"]
                .as_str()
                .ok_or("Missing drawing scope")?
                .into(),
            task_id: artifact["task_id"]
                .as_str()
                .ok_or("Missing drawing task")?
                .into(),
            task_revision: artifact["task_revision"]
                .as_str()
                .ok_or("Missing drawing revision")?
                .into(),
            format: artifact["format"]
                .as_str()
                .ok_or("Missing drawing format")?
                .into(),
            sha256: artifact["sha256"]
                .as_str()
                .ok_or("Missing drawing identity")?
                .into(),
            local_key: key,
        })
    }
    pub(crate) fn writing_native_instance(
        &self,
        id: Uuid,
        round: Uuid,
        instance: Option<Uuid>,
    ) -> Result<()> {
        let mut run = self.active_handwriting()?;
        if run.id != id || run.round_id != round {
            return Err("The writing attempt changed.".into());
        }
        run.native_instance = instance;
        self.repo.set(&activity_key(id), &run)
    }
    /// Explicit requests for the example are useful practice opportunities even
    /// when the learner could not yet put ink on the page. They are not attempts.
    pub(crate) fn writing_checkpoint_help_requested(
        &self,
        run: &HandwritingActivity,
    ) -> Result<bool> {
        if run.sequence == WritingSequence::HearWriteCompareSay
            && run.stage == WritingStage::Writing
            && run.model_revealed
        {
            return Ok(true);
        }
        if let Some(draft) = self.handwriting_draft(&run.scope(), &run.task())? {
            if draft.aids.iter().any(|aid| {
                matches!(
                    aid.kind.as_str(),
                    "visual_model" | "meaning" | "stroke_model_replay"
                )
            }) {
                return Ok(true);
            }
        }
        if let Some(checkpoint) = self
            .repo
            .get::<serde_json::Value>(&format!("handwriting.native.latest:{}", run.scope()))?
        {
            if checkpoint
                .get("model_help")
                .and_then(|h| h.get("revealed_at_ms"))
                .and_then(|v| v.as_array())
                .is_some_and(|times| !times.is_empty())
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn writing_checkpoint_reference_access(&self) -> Result<()> {
        let course = self.foundation_state()?;
        if let Some(checkpoint) = &course.writing_checkpoint {
            self.repo.set(
                &format!("foundation.writing.reference:{}", checkpoint.item_session()),
                &true,
            )?;
            if let Some(mut run) = self.writing_checkpoint_run(checkpoint)? {
                if run.stage == WritingStage::Writing && run.original.is_none() {
                    if let Some(cues) = &mut run.pre_submission_cues {
                        if !cues.iter().any(|cue| cue == "reference_material") {
                            cues.push("reference_material".into());
                        }
                    }
                    self.repo.set(&activity_key(run.id), &run)?;
                }
            }
        }
        Ok(())
    }
    fn writing_reference_use(&self) -> Result<()> {
        self.writing_checkpoint_reference_access()?;
        let mut course = self.foundation_state()?;
        if course.writing_checkpoint.is_some() {
            return Ok(());
        }
        let pack = rufren_content::foundations();
        if matches!(
            pack.episodes[course.episode].steps[course.step].action,
            rufren_content::FoundationAction::Recall { .. }
        ) && course.feedback.is_none()
            && !course.cues.contains(&rufren_core::Cue::ReferenceMaterial)
        {
            course.cues.push(rufren_core::Cue::ReferenceMaterial);
            self.foundation_save(&course)?
        }
        Ok(())
    }
    pub(crate) fn writing_navigation_needs_flush(&self) -> bool {
        if self.page == Page::Foundations {
            return self.foundation_writing().ok().flatten().is_some_and(|run| {
                self.active_handwriting()
                    .ok()
                    .is_some_and(|active| active.id == run.id)
            });
        }
        self.page == Page::Handwriting
            && !matches!(
                self.repo
                    .get::<String>("handwriting.surface")
                    .ok()
                    .flatten()
                    .as_deref(),
                Some("catalog" | "history")
            )
            && self.active_handwriting().is_ok()
    }
    pub(crate) fn open_writing_catalog(&mut self) -> Result<()> {
        self.writing_reference_use()?;
        self.repo.set("handwriting.surface", &"catalog")?;
        self.navigate(Page::Handwriting)
    }
    pub(crate) fn open_writing_history_list(&mut self) -> Result<()> {
        self.writing_reference_use()?;
        self.repo.set("handwriting.surface", &"history")?;
        self.navigate(Page::Handwriting)
    }
}
