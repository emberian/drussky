//! Local exact-step progress; authored responses use the existing immutable evidence log.
use crate::{
    model::{AppState, Page},
    repository::{Result, now_ms},
};
use rufren_content::{FoundationAction, FoundationStep};
use rufren_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;
const KEY: &str = "foundation_journey_v1";
// Deterministic authored targets retain their original wire version across app upgrades.
const FOUNDATION_CONTENT_EVENT_VERSION: u32 = 8;
pub(crate) fn foundation_tiles_text(step: &FoundationStep, tiles: &[String]) -> String {
    let words = matches!(&step.action,FoundationAction::Blend{tiles,..} if tiles.iter().any(|s|s.chars().count()>1));
    tiles.join(if words { " " } else { "" })
}
pub(crate) fn foundation_missing_intros(
    s: &FoundationState,
    step: &FoundationStep,
) -> Vec<FoundationStep> {
    let earlier: Vec<_> = rufren_content::foundations()
        .episodes
        .into_iter()
        .flat_map(|e| e.steps)
        .take_while(|candidate| candidate.id != step.id)
        .collect();
    let mut graphemes = step.requires_graphemes.clone();
    let mut meanings = step.requires_meanings.clone();
    let mut wanted = BTreeSet::new();
    loop {
        let before = wanted.len();
        for candidate in &earlier {
            if !s
                .encounters
                .contains(&format!("{}:encountered", candidate.id))
                && (candidate
                    .introduces_graphemes
                    .iter()
                    .any(|g| graphemes.contains(g))
                    || candidate
                        .introduces_meanings
                        .iter()
                        .any(|m| meanings.contains(m)))
            {
                wanted.insert(candidate.id.clone());
                for g in &candidate.requires_graphemes {
                    if !graphemes.contains(g) {
                        graphemes.push(g.clone())
                    }
                }
                for m in &candidate.requires_meanings {
                    if !meanings.contains(m) {
                        meanings.push(m.clone())
                    }
                }
            }
        }
        if before == wanted.len() {
            break;
        }
    }
    earlier
        .into_iter()
        .filter(|candidate| wanted.contains(&candidate.id))
        .collect()
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct FoundationState {
    pub version: u32,
    pub revision: String,
    pub episode: usize,
    pub episode_id: String,
    pub step_id: String,
    pub step_revision: String,
    pub step: usize,
    pub session: Uuid,
    pub draft: Vec<String>,
    pub cues: Vec<Cue>,
    pub revealed: bool,
    pub feedback: Option<String>,
    pub accepted: bool,
    #[serde(default)]
    pub played: Vec<String>,
    pub encounters: Vec<String>,
    pub attempts: Vec<Uuid>,
    pub pending: Option<FoundationPending>,
    pub finished: bool,
    #[serde(default)]
    pub recording: Option<AudioRecordMetadata>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct FoundationPending {
    events: Vec<Event>,
    feedback: String,
    accepted: bool,
}
impl Default for FoundationState {
    fn default() -> Self {
        Self {
            version: 1,
            revision: rufren_content::foundations().revision,
            episode: 0,
            episode_id: "foundation-1".into(),
            step_id: "f1-a".into(),
            step_revision: "1".into(),
            step: 0,
            session: Uuid::new_v4(),
            draft: vec![],
            cues: vec![],
            revealed: false,
            feedback: None,
            accepted: false,
            played: vec![],
            encounters: vec![],
            attempts: vec![],
            pending: None,
            finished: false,
            recording: None,
        }
    }
}
impl AppState {
    fn foundation_revision(&self, s: &FoundationState, step: &FoundationStep) -> Result<Event> {
        let pack = rufren_content::foundations();
        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("rufren-foundation:{}:{}", s.revision, step.id).as_bytes(),
        );
        let russian = step
            .audio_ids
            .first()
            .and_then(|id| pack.assets.iter().find(|a| &a.id == id))
            .map(|a| a.spoken.clone())
            .or_else(|| {
                step.display
                    .clone()
                    .filter(|v| v.chars().any(|c| ('А'..='я').contains(&c)))
            })
            .ok_or("This foundation task has no authored Russian target.")?;
        Ok(Event {
            version: FOUNDATION_CONTENT_EVENT_VERSION,
            id,
            device: Uuid::new_v5(&Uuid::NAMESPACE_URL, b"rufren-foundation-author-v1"),
            recorded_at_ms: 0,
            fact: Fact::PassageRevised(Revision {
                passage: Passage {
                    id,
                    title: step.instruction.clone(),
                    russian,
                    translation: step.instruction.clone(),
                    source: format!(
                        "rufren-foundation-v1:{}:{}@{}",
                        s.revision, step.id, step.revision
                    ),
                    coach_reviewed: false,
                },
                parents: BTreeSet::new(),
            }),
        })
    }
    pub(crate) fn foundation_state(&self) -> Result<FoundationState> {
        let s: FoundationState = self.repo.get(KEY)?.unwrap_or_default();
        if s.version != 1 || s.revision != rufren_content::foundations().revision {
            return Err(
                "This saved foundation uses a different edition. Your progress is preserved."
                    .into(),
            );
        }
        let pack = rufren_content::foundations();
        let Some(episode) = pack.episodes.get(s.episode) else {
            return Err(
                "Your saved foundation episode is unavailable; its progress is preserved.".into(),
            );
        };
        let Some(step) = episode.steps.get(s.step) else {
            return Err(
                "Your saved foundation step is unavailable; its progress is preserved.".into(),
            );
        };
        if episode.id != s.episode_id || step.id != s.step_id || step.revision != s.step_revision {
            return Err("Your saved foundation task belongs to a different edition; its progress is preserved.".into());
        }
        Ok(s)
    }
    pub(crate) fn foundation_save(&self, s: &FoundationState) -> Result<()> {
        self.repo.set(KEY, s)
    }
    pub(crate) fn start_foundations(&mut self) -> Result<()> {
        let s = self.foundation_state()?;
        self.foundation_save(&s)?;
        self.navigate(Page::Foundations)
    }
    pub(crate) fn foundation_has_started(&self) -> bool {
        self.repo
            .get::<FoundationState>(KEY)
            .ok()
            .flatten()
            .is_some()
    }
    pub(crate) fn foundation_edit(&self, draft: Vec<String>) -> Result<()> {
        let mut s = self.foundation_state()?;
        if s.pending.is_some() || s.feedback.is_some() {
            return Err("Save or retry the current response first.".into());
        }
        s.draft = draft;
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_help(&self) -> Result<()> {
        let mut s = self.foundation_state()?;
        s.revealed = true;
        if !s.cues.contains(&Cue::Coach) {
            s.cues.push(Cue::Coach)
        }
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_replayed(&self, asset: &str) -> Result<()> {
        let mut s = self.foundation_state()?;
        if !s.cues.contains(&Cue::Replay) {
            s.cues.push(Cue::Replay)
        }
        if !s.played.iter().any(|a| a == asset) {
            s.played.push(asset.into())
        }
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_retry(&self) -> Result<()> {
        let mut s = self.foundation_state()?;
        if s.pending.is_some() {
            return Err("Save the previous response before retrying.".into());
        }
        s.feedback = None;
        s.accepted = false;
        if !s.cues.contains(&Cue::Coach) {
            s.cues.push(Cue::Coach)
        }
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_submit(&mut self, step: &FoundationStep) -> Result<()> {
        let mut s = self.foundation_state()?;
        let pack = rufren_content::foundations();
        let actual = &pack.episodes[s.episode].steps[s.step];
        if actual != step {
            return Err("Reopen the current foundation step.".into());
        }
        if s.pending.is_none() {
            if !foundation_missing_intros(&s, step).is_empty()
                && !s.cues.contains(&Cue::ReferenceMaterial)
            {
                s.cues.push(Cue::ReferenceMaterial)
            }
            if let FoundationAction::Blend { answer, .. } = &step.action {
                let cue = if step
                    .display
                    .as_deref()
                    .is_some_and(|display| display == foundation_tiles_text(step, answer))
                {
                    Cue::OriginalText
                } else {
                    Cue::ReferenceMaterial
                };
                if !s.cues.contains(&cue) {
                    s.cues.push(cue)
                }
            }
            if matches!(step.action, FoundationAction::Choose { .. })
                && !step.audio_ids.is_empty()
                && !step.audio_ids.iter().any(|id| s.played.contains(id))
            {
                return Err("Listen to the prepared sound before choosing. If sound is unavailable, use Skip for now.".into());
            }
            if s.feedback.is_some() {
                return Ok(());
            }
            let (accepted, feedback, reference, task, skill) = match &step.action {
                FoundationAction::Choose {
                    options,
                    correct_id,
                    success,
                } => {
                    let answer = s.draft.first().ok_or("Choose an answer first.")?;
                    let option = options
                        .iter()
                        .find(|o| &o.id == answer)
                        .ok_or("Choose one of the displayed answers.")?;
                    let ok = answer == correct_id;
                    (
                        ok,
                        if ok {
                            success.clone()
                        } else {
                            option.feedback.clone()
                        },
                        options
                            .iter()
                            .find(|o| &o.id == correct_id)
                            .unwrap()
                            .text
                            .clone(),
                        if step.requires_meanings.is_empty() {
                            TaskKind::SoundDiscrimination
                        } else {
                            TaskKind::MeaningRecall
                        },
                        Skill::Listening,
                    )
                }
                FoundationAction::Blend {
                    answer,
                    feedback,
                    success,
                    ..
                } => {
                    let ok = &s.draft == answer;
                    let pos = s
                        .draft
                        .iter()
                        .zip(answer)
                        .position(|(a, b)| a != b)
                        .unwrap_or(s.draft.len().min(answer.len().saturating_sub(1)));
                    (
                        ok,
                        if ok {
                            success.clone()
                        } else if s.draft.len() > answer.len()
                            && s.draft.iter().zip(answer).all(|(a, b)| a == b)
                        {
                            format!(
                                "The model ends after {} tiles. Remove the extra tile(s) with Undo, then try again.",
                                answer.len()
                            )
                        } else {
                            feedback[pos].clone()
                        },
                        foundation_tiles_text(step, answer),
                        if answer.iter().any(|a| a.chars().count() > 1) {
                            TaskKind::Composition
                        } else {
                            TaskKind::Spelling
                        },
                        Skill::Writing,
                    )
                }
                _ => return Err("This teaching card does not collect a scored answer.".into()),
            };
            let now = now_ms();
            let revision = Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("rufren-foundation:{}:{}", s.revision, step.id).as_bytes(),
            );
            let attempt = Uuid::new_v4();
            let make = |id, fact| Event {
                version: EVENT_VERSION,
                id,
                device: self.device,
                recorded_at_ms: now,
                fact,
            };
            let response = match &step.action {
                FoundationAction::Choose { options, .. } => options
                    .iter()
                    .find(|o| s.draft.first() == Some(&o.id))
                    .unwrap()
                    .text
                    .clone(),
                _ => foundation_tiles_text(step, &s.draft),
            };
            let mut events = vec![];
            events.push(self.foundation_revision(&s, step)?);
            events.push(make(
                attempt,
                Fact::AttemptRecorded(Attempt {
                    session: s.session,
                    revision,
                    skill,
                    response,
                    cues: s.cues.clone(),
                    self_rating: SelfRating::NotReported,
                    policy: "foundation-supported-v1".into(),
                }),
            ));
            events.push(make(
                Uuid::new_v5(&attempt, b"context"),
                Fact::AttemptContextRecorded(AttemptContext {
                    attempt,
                    content_pack: Some("foundations".into()),
                    content_revision: Some(s.revision.clone()),
                    exercise_id: Some(format!("{}@{}", step.id, step.revision)),
                    prompt: Some(serde_json::to_string(step).map_err(|e| e.to_string())?),
                    reference_answer: Some(reference),
                    task,
                    target_features: step
                        .requires_graphemes
                        .iter()
                        .map(|g| format!("cyrillic:{g}"))
                        .collect(),
                    novel_context: false,
                    delayed_probe: false,
                    practice_duration_ms: None,
                    effort: None,
                    grammar_response: None,
                    supersedes: BTreeSet::new(),
                }),
            ));
            events.push(make(Uuid::new_v5(&attempt,b"authored-match"),Fact::CoachJudgmentRecorded(CoachJudgment{attempt,dimension:if skill==Skill::Writing{JudgmentDimension::Spelling}else{JudgmentDimension::Comprehension},verdict:if accepted{Verdict::Correct}else{Verdict::Incorrect},confidence_percent:100,provenance:CoachProvenance{coach:"Prepared foundation answer key".into(),method:"Exact authored option or tile match".into(),note:"Task response only; supported practice is not mastery or pronunciation evidence.".into()},supersedes:BTreeSet::new()})));
            s.pending = Some(FoundationPending {
                events,
                feedback,
                accepted,
            });
            self.foundation_save(&s)?;
        }
        let pending = s.pending.clone().unwrap();
        self.repo.append(&pending.events)?;
        self.refresh()?;
        for e in &pending.events {
            if matches!(e.fact, Fact::AttemptRecorded(_)) && !s.attempts.contains(&e.id) {
                s.attempts.push(e.id)
            }
        }
        s.feedback = Some(pending.feedback);
        s.accepted = pending.accepted;
        s.pending = None;
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_save_recording(
        &mut self,
        metadata: AudioRecordMetadata,
    ) -> Result<()> {
        metadata.validate().map_err(|e| e.to_string())?;
        let mut s = self.foundation_state()?;
        let pack = rufren_content::foundations();
        let step = &pack.episodes[s.episode].steps[s.step];
        if !matches!(step.action, FoundationAction::Speak { .. }) {
            return Err("This recording belongs to a different step.".into());
        }
        let revision = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("rufren-foundation:{}:{}", s.revision, step.id).as_bytes(),
        );
        if metadata.revision_id != revision.to_string() || metadata.role != "attempt" {
            return Err("The recording does not match this foundation step.".into());
        }
        let event = Event {
            version: EVENT_VERSION,
            id: Uuid::new_v5(&s.session, metadata.asset_id.as_bytes()),
            device: self.device,
            recorded_at_ms: metadata.created_at_ms,
            fact: Fact::RecordingAttached(AudioAttachment {
                asset_id: metadata.asset_id.clone(),
                asset: AudioAsset {
                    sha256: metadata.content_hash.clone(),
                    media_type: metadata.mime_type.clone(),
                    sample_rate_hz: metadata.sample_rate,
                    channels: metadata.channels,
                    frames: metadata.frames,
                },
                target: AnnotationTarget::Revision(revision),
                role: AudioRole::Attempt,
                note: "Optional foundation self-comparison take. No acoustic judgment.".into(),
            }),
        };
        self.repo
            .append(&[self.foundation_revision(&s, step)?, event])?;
        self.refresh()?;
        s.recording = Some(metadata);
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_read_try(&mut self, step: &FoundationStep) -> Result<()> {
        let mut s = self.foundation_state()?;
        if !matches!(step.action, FoundationAction::Read { .. }) {
            return Err("This is not a reading turn.".into());
        }
        if s.feedback.is_some() {
            return self.foundation_help();
        }
        if s.pending.is_none() {
            if !foundation_missing_intros(&s, step).is_empty()
                && !s.cues.contains(&Cue::ReferenceMaterial)
            {
                s.cues.push(Cue::ReferenceMaterial)
            }
            let pack = rufren_content::foundations();
            if &pack.episodes[s.episode].steps[s.step] != step {
                return Err("Reopen this reading turn.".into());
            }
            let attempt = Uuid::new_v4();
            let now = now_ms();
            let revision = Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("rufren-foundation:{}:{}", s.revision, step.id).as_bytes(),
            );
            let make = |id, fact| Event {
                version: EVENT_VERSION,
                id,
                device: self.device,
                recorded_at_ms: now,
                fact,
            };
            s.draft =
                vec!["I tried reading aloud (self-report; no pronunciation judgment).".into()];
            let events = vec![
                self.foundation_revision(&s, step)?,
                make(
                    attempt,
                    Fact::AttemptRecorded(Attempt {
                        session: s.session,
                        revision,
                        skill: Skill::Reading,
                        response: s.draft.join(""),
                        cues: s.cues.clone(),
                        self_rating: SelfRating::NotReported,
                        policy: "foundation-reading-self-comparison-v1".into(),
                    }),
                ),
                make(
                    Uuid::new_v5(&attempt, b"context"),
                    Fact::AttemptContextRecorded(AttemptContext {
                        attempt,
                        content_pack: Some("foundations".into()),
                        content_revision: Some(s.revision.clone()),
                        exercise_id: Some(format!("{}@{}", step.id, step.revision)),
                        prompt: Some(step.instruction.clone()),
                        reference_answer: step.display.clone(),
                        task: TaskKind::FormRecall,
                        target_features: step
                            .requires_graphemes
                            .iter()
                            .map(|g| format!("cyrillic:{g}"))
                            .collect(),
                        novel_context: false,
                        delayed_probe: false,
                        practice_duration_ms: None,
                        effort: None,
                        grammar_response: None,
                        supersedes: BTreeSet::new(),
                    }),
                ),
            ];
            s.pending=Some(FoundationPending{events,feedback:"Reading attempt noted. Listen and compare for yourself; the app has not judged your pronunciation.".into(),accepted:true});
            self.foundation_save(&s)?;
        }
        self.foundation_submit(step)?;
        self.foundation_help()
    }
    pub(crate) fn foundation_repeat_episode(&self) -> Result<()> {
        let mut s = self.foundation_state()?;
        if s.pending.is_some() {
            return Err("Save the current answer first.".into());
        }
        s.step = 0;
        s.finished = false;
        s.session = Uuid::new_v4();
        let pack = rufren_content::foundations();
        s.step_id = pack.episodes[s.episode].steps[0].id.clone();
        s.step_revision = pack.episodes[s.episode].steps[0].revision.clone();
        s.draft.clear();
        s.cues.clear();
        s.played.clear();
        s.revealed = false;
        s.feedback = None;
        s.accepted = false;
        s.recording = None;
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_next(&self, skip: bool) -> Result<()> {
        let mut s = self.foundation_state()?;
        if s.pending.is_some() {
            return Err("Save your response before leaving this step.".into());
        }
        let pack = rufren_content::foundations();
        let step = &pack.episodes[s.episode].steps[s.step];
        if !skip
            && matches!(step.action, FoundationAction::Present { .. })
            && !step.audio_ids.is_empty()
            && !step.audio_ids.iter().all(|a| s.played.contains(a))
        {
            return Err("Listen to the prepared model first, or explicitly Skip for now.".into());
        }
        if !skip && matches!(step.action, FoundationAction::Read { .. }) && !s.revealed {
            return Err("Try reading and reveal the model, or use Show me / Skip for now.".into());
        }
        if !skip
            && matches!(
                step.action,
                FoundationAction::Choose { .. } | FoundationAction::Blend { .. }
            )
            && !s.accepted
        {
            return Err("Try your answer, or use Skip for now.".into());
        }
        s.encounters.push(format!(
            "{}:{}",
            step.id,
            if skip { "skipped" } else { "encountered" }
        ));
        if s.step + 1 < pack.episodes[s.episode].steps.len() {
            s.step += 1
        } else if s.episode + 1 < pack.episodes.len() {
            s.episode += 1;
            s.step = 0
        } else {
            s.finished = true
        }
        s.episode_id = pack.episodes[s.episode].id.clone();
        s.step_id = pack.episodes[s.episode].steps[s.step].id.clone();
        s.step_revision = pack.episodes[s.episode].steps[s.step].revision.clone();
        s.session = Uuid::new_v4();
        s.draft.clear();
        s.recording = None;
        s.played.clear();
        s.cues.clear();
        s.revealed = false;
        s.feedback = None;
        s.accepted = false;
        self.foundation_save(&s)
    }
}
