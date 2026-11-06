//! Local exact-step progress; authored responses use the existing immutable evidence log.
use crate::{
    model::{AppState, Page},
    repository::{Result, now_ms},
};
use rufren_content::foundations::{
    FoundationRecallMode, foundation_step_pack_revision, foundations_legacy,
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
    let mut samples = step.requires_samples.clone();
    let mut conventions = step.requires_conventions.clone();
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
                        .any(|m| meanings.contains(m))
                    || candidate
                        .introduces_samples
                        .iter()
                        .any(|v| samples.contains(v))
                    || candidate
                        .introduces_conventions
                        .iter()
                        .any(|v| conventions.contains(v)))
            {
                wanted.insert(candidate.id.clone());
                for g in &candidate.requires_graphemes {
                    if !graphemes.contains(g) {
                        graphemes.push(g.clone())
                    }
                }
                for value in &candidate.requires_samples {
                    if !samples.contains(value) {
                        samples.push(value.clone())
                    }
                }
                for value in &candidate.requires_conventions {
                    if !conventions.contains(value) {
                        conventions.push(value.clone())
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
    #[serde(default)]
    pub review_resume: Option<Box<FoundationState>>,
    #[serde(default)]
    pub cannot_yet: bool,
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
            review_resume: None,
            cannot_yet: false,
        }
    }
}
impl AppState {
    pub(crate) fn foundation_revision(
        &self,
        _s: &FoundationState,
        step: &FoundationStep,
    ) -> Result<Event> {
        let pack = rufren_content::foundations();
        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!(
                "rufren-foundation:{}:{}",
                foundation_step_pack_revision(&step.id),
                step.id
            )
            .as_bytes(),
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
            .or_else(|| match &step.action {
                FoundationAction::Recall { answer, mode, .. }
                    if matches!(
                        mode,
                        FoundationRecallMode::Written | FoundationRecallMode::Spoken
                    ) =>
                {
                    Some(answer.clone())
                }
                _ => None,
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
                        foundation_step_pack_revision(&step.id),
                        step.id,
                        step.revision
                    ),
                    coach_reviewed: false,
                },
                parents: BTreeSet::new(),
            }),
        })
    }
    pub(crate) fn foundation_state(&self) -> Result<FoundationState> {
        let mut s: FoundationState = self.repo.get(KEY)?.unwrap_or_default();
        let pack = rufren_content::foundations();
        if s.version != 1 {
            return Err(
                "This saved foundation uses a different format. Your progress is preserved.".into(),
            );
        }
        if s.revision != pack.revision {
            let old = foundations_legacy();
            if s.revision != old.revision
                || pack.episodes.len() <= old.episodes.len()
                || pack.episodes[..old.episodes.len()] != old.episodes[..]
            {
                return Err(
                    "This saved foundation uses a different edition. Your progress is preserved."
                        .into(),
                );
            }
            let Some(old_episode) = old.episodes.get(s.episode) else {
                return Err(
                    "Your saved foundation episode is unavailable; its progress is preserved."
                        .into(),
                );
            };
            let Some(old_step) = old_episode.steps.get(s.step) else {
                return Err(
                    "Your saved foundation step is unavailable; its progress is preserved.".into(),
                );
            };
            if s.episode_id != old_episode.id
                || s.step_id != old_step.id
                || s.step_revision != old_step.revision
            {
                return Err("Your saved foundation task belongs to a different edition; its progress is preserved.".into());
            }
            // An unfinished packet is recovered unchanged before starting a later task.
            if s.finished {
                if s.pending.is_some()
                    || s.episode + 1 != old.episodes.len()
                    || s.step + 1 != old_episode.steps.len()
                {
                    return Err(
                        "Your saved course ending needs recovery. Its work is preserved.".into(),
                    );
                }
                s.episode = old.episodes.len();
                s.step = 0;
                s.episode_id = pack.episodes[s.episode].id.clone();
                s.step_id = pack.episodes[s.episode].steps[0].id.clone();
                s.step_revision = pack.episodes[s.episode].steps[0].revision.clone();
                s.session = Uuid::new_v4();
                s.finished = false;
                s.draft.clear();
                s.cues.clear();
                s.played.clear();
                s.feedback = None;
                s.revealed = false;
                s.accepted = false;
                s.recording = None;
            }
            s.revision = pack.revision.clone();
            self.foundation_save(&s)?;
        }
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
        if draft.iter().map(|v| v.len()).sum::<usize>() > 8_000 {
            return Err("Keep this answer short so it can be saved.".into());
        }
        s.draft = draft;
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_help(&mut self) -> Result<()> {
        let mut s = self.foundation_state()?;
        if s.pending.is_some() {
            return Err("Finish saving your response before showing the model.".into());
        }
        s.revealed = true;
        let pack = rufren_content::foundations();
        if let FoundationAction::Recall { mode, .. } =
            &pack.episodes[s.episode].steps[s.step].action
        {
            let cue = if matches!(
                mode,
                FoundationRecallMode::ListeningMeaning | FoundationRecallMode::ReadingMeaning
            ) {
                Cue::Translation
            } else {
                Cue::OriginalText
            };
            if !s.cues.contains(&cue) {
                s.cues.push(cue)
            }
        }
        if !s.cues.contains(&Cue::Coach) {
            s.cues.push(Cue::Coach)
        }
        self.foundation_save(&s)
    }
    pub(crate) fn foundation_replayed(&self, asset: &str) -> Result<()> {
        let mut s = self.foundation_state()?;
        let pack = rufren_content::foundations();
        let stimulus = matches!(
            &pack.episodes[s.episode].steps[s.step].action,
            FoundationAction::Recall {
                mode: FoundationRecallMode::ListeningMeaning,
                ..
            }
        ) && !s.played.iter().any(|a| a == asset);
        if !stimulus && !s.cues.contains(&Cue::Replay) {
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
        if s.cannot_yet && s.draft.join("") == "I cannot recall it yet." {
            s.draft.clear()
        }
        s.cannot_yet = false;
        s.feedback = None;
        s.accepted = false;
        if !s.cues.contains(&Cue::Coach) {
            s.cues.push(Cue::Coach)
        }
        self.foundation_save(&s)
    }
    fn foundation_reserve_recall(
        &self,
        s: &mut FoundationState,
        step: &FoundationStep,
    ) -> Result<()> {
        let FoundationAction::Recall {
            mode,
            answer,
            accepted,
            feedback,
            reveal,
            ..
        } = &step.action
        else {
            return Err("This is not a recall turn.".into());
        };
        if matches!(mode, FoundationRecallMode::ListeningMeaning)
            && !step.audio_ids.iter().any(|a| s.played.contains(a))
        {
            return Err("Listen first, then give the meaning. You can also skip.".into());
        }
        let response = s.draft.join("");
        if response.trim().is_empty() {
            return Err("Give your response, or use Skip for now.".into());
        }
        if !foundation_missing_intros(s, step).is_empty()
            && !s.cues.contains(&Cue::ReferenceMaterial)
        {
            s.cues.push(Cue::ReferenceMaterial)
        }
        let normal = |text: &str| {
            if matches!(mode, FoundationRecallMode::Written) {
                return text.trim().to_lowercase();
            }
            text.trim()
                .trim_end_matches(['.', '!', '?'])
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        let normalized = normal(&response);
        let known = feedback.iter().find(|f| normal(&f.response) == normalized);
        let correct =
            accepted.iter().any(|a| normal(a) == normalized) || normal(answer) == normalized;
        let judgment = if s.cannot_yet {
            None
        } else {
            match mode {
                FoundationRecallMode::Written => Some(if correct {
                    Verdict::Correct
                } else {
                    Verdict::Incorrect
                }),
                FoundationRecallMode::Spoken => None,
                _ => {
                    if correct {
                        Some(Verdict::Correct)
                    } else if known.is_some() {
                        Some(Verdict::Incorrect)
                    } else {
                        None
                    }
                }
            }
        };
        let explanation = if s.cannot_yet {
            "Here is the model. You can try it again with help whenever you like.".into()
        } else if let Some(known) = known {
            known.feedback.clone()
        } else if judgment == Some(Verdict::Incorrect) {
            let target: Vec<_> = answer.chars().collect();
            let given: Vec<_> = response.chars().collect();
            if given.len() > target.len() && given.iter().zip(&target).all(|(a, b)| a == b) {
                format!(
                    "The word ends after {} letters. Remove the extra letters, then compare with the model.",
                    target.len()
                )
            } else if let Some(index) = given
                .iter()
                .zip(&target)
                .position(|(a, b)| a.to_lowercase().to_string() != b.to_lowercase().to_string())
            {
                format!(
                    "At position {}, the model uses {}. Compare it with your saved answer.",
                    index + 1,
                    target[index]
                )
            } else {
                format!("Compare the ending with the model: {answer}.")
            }
        } else {
            match mode{FoundationRecallMode::Spoken=>"Your speaking attempt is saved. Listen and compare your own voice with the reference.".into(),FoundationRecallMode::Written=>"Your spelling matches this word. Listen to its sound again if you like.".into(),_=>"Compare your meaning with the example below. Your wording can be different and still express the same idea.".into()}
        };
        let now = now_ms();
        let attempt = Uuid::new_v4();
        let source = self.foundation_revision(s, step)?;
        let revision = source.id;
        let make = |id, fact| Event {
            version: EVENT_VERSION,
            id,
            device: self.device,
            recorded_at_ms: now,
            fact,
        };
        let task = match mode {
            FoundationRecallMode::Written => TaskKind::Spelling,
            FoundationRecallMode::Spoken => TaskKind::FormRecall,
            _ => TaskKind::MeaningRecall,
        };
        let skill = match mode {
            FoundationRecallMode::ListeningMeaning => Skill::Listening,
            FoundationRecallMode::ReadingMeaning => Skill::Reading,
            FoundationRecallMode::Written => Skill::Writing,
            FoundationRecallMode::Spoken => Skill::Speaking,
        };
        let mut events = vec![
            source,
            make(
                attempt,
                Fact::AttemptRecorded(Attempt {
                    session: s.session,
                    revision,
                    skill,
                    response,
                    cues: s.cues.clone(),
                    self_rating: SelfRating::NotReported,
                    policy: if s.cues.is_empty() {
                        "foundation-recall-v1"
                    } else {
                        "foundation-assisted-recall-v1"
                    }
                    .into(),
                }),
            ),
            make(
                Uuid::new_v5(&attempt, b"context"),
                Fact::AttemptContextRecorded(AttemptContext {
                    attempt,
                    content_pack: Some("foundations".into()),
                    content_revision: Some(foundation_step_pack_revision(&step.id)),
                    exercise_id: Some(format!("{}@{}", step.id, step.revision)),
                    prompt: Some(format!(
                        "{}{}",
                        serde_json::to_string(step).map_err(|e| e.to_string())?,
                        if s.cannot_yet {
                            "\nLearner chose: cannot recall yet."
                        } else {
                            ""
                        }
                    )),
                    reference_answer: Some(format!("{answer} — {reveal}")),
                    task,
                    target_features: step
                        .requires_graphemes
                        .iter()
                        .map(|g| format!("cyrillic:{g}"))
                        .chain(
                            step.requires_meanings
                                .iter()
                                .map(|m| format!("meaning:{m}")),
                        )
                        .collect(),
                    novel_context: false,
                    delayed_probe: s.review_resume.is_some(),
                    practice_duration_ms: None,
                    effort: None,
                    grammar_response: None,
                    supersedes: BTreeSet::new(),
                }),
            ),
        ];
        if let Some(verdict) = judgment {
            events.push(make(Uuid::new_v5(&attempt,b"authored-match"),Fact::CoachJudgmentRecorded(CoachJudgment{attempt,dimension:if skill==Skill::Writing{JudgmentDimension::Spelling}else{JudgmentDimension::Comprehension},verdict,confidence_percent:100,provenance:CoachProvenance{coach:"Prepared foundation answer key".into(),method:"Exact authored form, accepted meaning or known confusion".into(),note:"Unlisted English wording is left for self-comparison. No pronunciation judgment.".into()},supersedes:BTreeSet::new()})))
        }
        if let Some(recording) = &s.recording {
            events.push(make(
                Uuid::new_v5(&attempt, b"spoken-original"),
                Fact::RecordingAttached(AudioAttachment {
                    asset_id: recording.asset_id.clone(),
                    asset: AudioAsset {
                        sha256: recording.content_hash.clone(),
                        media_type: recording.mime_type.clone(),
                        sample_rate_hz: recording.sample_rate,
                        channels: recording.channels,
                        frames: recording.frames,
                    },
                    target: AnnotationTarget::Attempt(attempt),
                    role: AudioRole::Attempt,
                    note: "Original optional speaking take; no acoustic assessment.".into(),
                }),
            ))
        }
        s.pending = Some(FoundationPending {
            events,
            feedback: explanation,
            accepted: judgment == Some(Verdict::Correct),
        });
        self.foundation_save(s)
    }
    pub(crate) fn foundation_cannot_yet(&mut self, step: &FoundationStep) -> Result<()> {
        let mut s = self.foundation_state()?;
        if s.pending.is_some() {
            return self.foundation_submit(step);
        }
        if !matches!(step.action, FoundationAction::Recall { .. }) {
            return self.foundation_help();
        }
        s.cannot_yet = true;
        if s.draft.join("").trim().is_empty() {
            s.draft = vec!["I cannot recall it yet.".into()]
        };
        self.foundation_save(&s)?;
        self.foundation_submit(step)
    }
    pub(crate) fn foundation_submit(&mut self, step: &FoundationStep) -> Result<()> {
        let mut s = self.foundation_state()?;
        let pack = rufren_content::foundations();
        let actual = &pack.episodes[s.episode].steps[s.step];
        if actual != step {
            return Err("Reopen the current foundation step.".into());
        }
        if s.pending.is_none()
            && matches!(step.action, FoundationAction::Recall { .. })
            && s.feedback.is_some()
        {
            return Ok(());
        }
        if s.pending.is_none() && matches!(step.action, FoundationAction::Recall { .. }) {
            self.foundation_reserve_recall(&mut s, step)?;
        }
        if s.pending.is_none() {
            if !foundation_missing_intros(&s, step).is_empty()
                && !s.cues.contains(&Cue::ReferenceMaterial)
            {
                s.cues.push(Cue::ReferenceMaterial)
            }
            if matches!(step.action, FoundationAction::Copy { .. })
                && !s.cues.contains(&Cue::OriginalText)
            {
                s.cues.push(Cue::OriginalText)
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
                FoundationAction::Copy { answer, .. } => {
                    let ok = s.draft.join("").trim().to_lowercase() == answer.to_lowercase();
                    (
                        ok,
                        if ok {
                            "You entered the copied word. Next you can try it with the model hidden.".into()
                        } else {
                            format!(
                                "Compare your letters with the visible model {answer}. Use backspace to change the ending or a misplaced letter."
                            )
                        },
                        answer.clone(),
                        TaskKind::Spelling,
                        Skill::Writing,
                    )
                }
                _ => return Err("This teaching card does not collect a scored answer.".into()),
            };
            let now = now_ms();
            let revision = Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!(
                    "rufren-foundation:{}:{}",
                    foundation_step_pack_revision(&step.id),
                    step.id
                )
                .as_bytes(),
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
                    content_revision: Some(foundation_step_pack_revision(&step.id)),
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
        if matches!(step.action, FoundationAction::Recall { .. }) {
            s.revealed = true
        }
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
        if !matches!(
            step.action,
            FoundationAction::Speak { .. }
                | FoundationAction::Recall {
                    mode: FoundationRecallMode::Spoken,
                    ..
                }
        ) {
            return Err("This recording belongs to a different step.".into());
        }
        let revision = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!(
                "rufren-foundation:{}:{}",
                foundation_step_pack_revision(&step.id),
                step.id
            )
            .as_bytes(),
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
                format!(
                    "rufren-foundation:{}:{}",
                    foundation_step_pack_revision(&step.id),
                    step.id
                )
                .as_bytes(),
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
                        content_revision: Some(foundation_step_pack_revision(&step.id)),
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
        if s.review_resume.is_some() {
            return Err("Finish or skip this return before repeating an episode.".into());
        }
        s.step = 0;
        s.cannot_yet = false;
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
        if !skip && matches!(step.action, FoundationAction::Recall { .. }) && s.feedback.is_none() {
            return Err("Save your response, or use Skip for now.".into());
        }
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
                FoundationAction::Choose { .. }
                    | FoundationAction::Blend { .. }
                    | FoundationAction::Copy { .. }
            )
            && !s.accepted
        {
            return Err("Try your answer, or use Skip for now.".into());
        }
        if s.review_resume.is_some() {
            return self.foundation_finish_review(&s);
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
        s.cannot_yet = false;
        s.recording = None;
        s.played.clear();
        s.cues.clear();
        s.revealed = false;
        s.feedback = None;
        s.accepted = false;
        self.foundation_save(&s)
    }
}
