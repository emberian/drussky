//! Dedicated sound practice. Frozen trials, actual playback and saved responses remain distinct.
use crate::{
    model::{AppState, Page},
    repository::{Result, now_ms},
};
use rufren_core::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
const ACTIVE: &str = "sound_focus_active";
const SURFACE: &str = "practice_surface";
const PAUSED: &str = "sound_focus_paused_ids";
const POLICY: &str = "sound-focus-v1";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SoundFocusPhase {
    Examples,
    Listening,
    Speaking,
    Finished,
}
#[derive(Debug, Clone)]
pub(crate) struct SoundListeningTrial {
    pub trial_id: Uuid,
    pub token: Uuid,
    pub prepared_id: Uuid,
    pub target_member_id: String,
    pub option_member_ids: [String; 2],
    pub choice_member_id: Option<String>,
    pub saved_attempt: Option<Uuid>,
    pub can_answer: bool,
    pub play_count: usize,
    pub revealed: bool,
    pub conflicted: bool,
    pub incomplete: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct SoundSpeakingIntent {
    pub intent_id: Uuid,
    pub token: Uuid,
    pub prepared_id: Uuid,
    pub member_id: String,
    pub passage_id: Uuid,
    pub revision_id: Uuid,
    pub selected_asset_id: Option<String>,
    pub selected_attachment: Option<Uuid>,
    pub saved_attempt: Option<Uuid>,
    pub selected_interrupted: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct SoundFocusDraft {
    pub session_id: Uuid,
    pub contrast: SoundContrastSnapshot,
    pub references: [SoundReferenceBinding; 2],
    pub trials: Vec<SoundListeningTrial>,
    pub current_trial: usize,
    pub phase: SoundFocusPhase,
    pub labelled_examples_done: [bool; 2],
    pub speaking: Option<SoundSpeakingIntent>,
    pub historical: bool,
    pub history_incomplete: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectedTake {
    attachment: Uuid,
    metadata: AudioRecordMetadata,
    explicit_partial: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredFocus {
    version: u32,
    session_id: Uuid,
    contrast: SoundContrastSnapshot,
    references: [SoundReferenceBinding; 2],
    prepared: Vec<Event>,
    current_trial: usize,
    phase: SoundFocusPhase,
    speaking: Option<Event>,
    selected_take: Option<SelectedTake>,
    cues: BTreeMap<Uuid, Vec<Cue>>,
    historical: bool,
    #[serde(default)]
    response_packets: BTreeMap<Uuid, Vec<Event>>,
}
fn key(id: Uuid) -> String {
    format!("sound_focus_session:{id}")
}
fn fail() -> String {
    "This sound practice no longer matches its frozen trial or original recording. Your saved work is kept.".into()
}
fn snapshot(definition: &rufren_content::SoundContrastDefinition) -> Result<SoundContrastSnapshot> {
    serde_json::from_value(serde_json::to_value(definition).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn prepared(event: &Event) -> Result<&SoundTrialPrepared> {
    match &event.fact {
        Fact::SoundTrialPrepared(p) => Ok(p),
        _ => Err(fail()),
    }
}
fn metadata_asset(meta: &AudioRecordMetadata) -> AudioAsset {
    AudioAsset {
        sha256: meta.content_hash.clone(),
        media_type: meta.mime_type.clone(),
        sample_rate_hz: meta.sample_rate,
        channels: meta.channels,
        frames: meta.frames,
    }
}
fn check_metadata(reference: &SoundReferenceBinding, metadata: &AudioRecordMetadata) -> Result<()> {
    metadata.validate().map_err(|e| e.to_string())?;
    if metadata.asset_id != reference.asset_id
        || metadata.content_hash != reference.asset.sha256
        || metadata.role != "reference"
        || metadata.interrupted
        || metadata_asset(metadata) != reference.asset
        || metadata.revision_id != reference.revision.to_string()
    {
        return Err(fail());
    }
    Ok(())
}
impl StoredFocus {
    fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.session_id.is_nil()
            || (!self.historical && self.prepared.len() != 4)
            || (self.historical && self.prepared.len() > 4)
            || (self.historical && self.prepared.is_empty() && self.speaking.is_none())
            || self.current_trial > 3
        {
            return Err(fail());
        }
        for (position, event) in self.prepared.iter().enumerate() {
            event.validate().map_err(|e| e.to_string())?;
            let p = prepared(event)?;
            if p.block_id != self.session_id
                || (!self.historical && p.position != position)
                || p.mode != SoundTrialMode::Listening
                || p.contrast != self.contrast
                || p.references != self.references
            {
                return Err(fail());
            }
        }
        if let Some(e) = &self.speaking {
            e.validate().map_err(|e| e.to_string())?;
            let p = prepared(e)?;
            if p.block_id != self.session_id
                || p.mode != SoundTrialMode::Speaking
                || p.contrast != self.contrast
                || p.references != self.references
            {
                return Err(fail());
            }
        }
        if self.cues.values().any(|c| c.len() > 32) {
            return Err(fail());
        }
        for (trial, packet) in &self.response_packets {
            let prepared_event = self
                .prepared
                .iter()
                .chain(self.speaking.iter())
                .find(|e| prepared(e).is_ok_and(|p| p.trial_id == *trial))
                .ok_or_else(fail)?;
            let completions: Vec<_> = packet
                .iter()
                .filter_map(|e| match &e.fact {
                    Fact::SoundTrialCompleted(c) => Some(c),
                    _ => None,
                })
                .collect();
            if completions.len() != 1 {
                return Err(fail());
            }
            let c = completions[0];
            if c.prepared != prepared_event.id || packet.len() < 3 || packet.len() > 4 {
                return Err(fail());
            }
            let mut ids = BTreeSet::new();
            for e in packet {
                e.validate().map_err(|e| e.to_string())?;
                if !ids.insert(e.id) {
                    return Err(fail());
                }
                let valid = match &e.fact {
                    Fact::AttemptRecorded(a) => e.id == c.attempt && a.session == *trial,
                    Fact::AttemptContextRecorded(context) => {
                        e.id == c.context && context.attempt == c.attempt
                    }
                    Fact::SoundTrialCompleted(other) => other == c,
                    Fact::RecordingAttached(a) => {
                        matches!(a.target,AnnotationTarget::Attempt(id) if id==c.attempt)
                    }
                    _ => false,
                };
                if !valid {
                    return Err(fail());
                }
            }
            if !packet
                .iter()
                .any(|e| e.id == c.attempt && matches!(e.fact, Fact::AttemptRecorded(_)))
                || !packet
                    .iter()
                    .any(|e| e.id == c.context && matches!(e.fact, Fact::AttemptContextRecorded(_)))
            {
                return Err(fail());
            }
        }
        Ok(())
    }
}
impl AppState {
    fn sound_stored(&self) -> Result<Option<StoredFocus>> {
        let s = self.repo.get::<StoredFocus>(ACTIVE)?;
        if let Some(s) = &s {
            s.validate()?;
        }
        Ok(s)
    }
    fn sound_save(&self, s: &StoredFocus) -> Result<()> {
        s.validate()?;
        self.repo.set(ACTIVE, s)
    }
    fn sound_guard(&self, session: Uuid) -> Result<StoredFocus> {
        let s = self.sound_stored()?.ok_or("Open a sound practice first.")?;
        if s.session_id != session || s.historical {
            return Err(fail());
        }
        Ok(s)
    }
    fn sound_append(&mut self, events: &[Event]) -> Result<()> {
        self.refresh()?;
        let mut fresh = Vec::new();
        for event in events {
            if let Some(old) = self.library.events.get(&event.id) {
                if old != event {
                    return Err(fail());
                }
            } else {
                fresh.push(event.clone());
            }
        }
        if !fresh.is_empty() {
            self.repo.append(&fresh)?;
            self.refresh()?;
        }
        Ok(())
    }
    pub(crate) fn sound_focus_is_selected(&self) -> Result<bool> {
        Ok(self.repo.get::<String>(SURFACE)?.as_deref() == Some("sound_focus"))
    }
    pub(crate) fn sound_focus_unfinished(&self) -> Result<bool> {
        Ok(self
            .sound_stored()?
            .is_some_and(|s| !s.historical && s.phase != SoundFocusPhase::Finished))
    }
    pub(crate) fn select_ordinary_practice_surface(&self) -> Result<()> {
        self.repo.set(SURFACE, &"ordinary")
    }
    fn sound_archive_active(&self) -> Result<()> {
        if let Some(s) = self.sound_stored()? {
            // History is a read-only fact projection, never the editable paused workspace.
            if s.historical {
                return Ok(());
            }
            self.repo.set(&key(s.session_id), &s)?;
            let mut ids: BTreeSet<Uuid> = self.repo.get(PAUSED)?.unwrap_or_default();
            ids.insert(s.session_id);
            self.repo.set(PAUSED, &ids)?;
        }
        Ok(())
    }
    pub(crate) fn register_sound_focus_reference(
        &mut self,
        definition: &rufren_content::SoundContrastDefinition,
        member_id: &str,
        metadata: &AudioRecordMetadata,
    ) -> Result<SoundReferenceBinding> {
        let contrast = snapshot(definition)?;
        let member = contrast
            .members
            .iter()
            .find(|m| m.id == member_id)
            .ok_or_else(fail)?;
        let reference = sound_reference_event(&contrast, member_id).map_err(|e| e.to_string())?;
        let (passage, revision) =
            sound_reference_identity(&member.utterance.id, &member.utterance.version);
        metadata.validate().map_err(|e| e.to_string())?;
        if metadata.role != "reference"
            || metadata.interrupted
            || metadata.passage_id != passage.to_string()
            || metadata.revision_id != revision.to_string()
        {
            return Err(fail());
        }
        let source = metadata.device.get("provenance").ok_or_else(fail)?;
        let provenance_json = serde_json::to_string(source).map_err(|e| e.to_string())?;
        let asset = metadata_asset(metadata);
        let source_text = member.utterance.text.clone();
        let source_text_sha256 = dialogue_text_sha256(&source_text);
        let recording = Uuid::new_v5(
            &revision,
            format!("audio:{}:reference", metadata.asset_id).as_bytes(),
        );
        let attachment = Event {
            version: 6,
            id: recording,
            device: Uuid::new_v5(&revision, b"audio-attachment-writer"),
            recorded_at_ms: metadata.created_at_ms,
            fact: Fact::RecordingAttached(AudioAttachment {
                asset_id: metadata.asset_id.clone(),
                asset: asset.clone(),
                target: AnnotationTarget::Revision(revision),
                role: AudioRole::Reference,
                note: String::new(),
            }),
        };
        let binding = SoundReferenceBinding {
            member_id: member_id.into(),
            revision,
            recording,
            asset_id: metadata.asset_id.clone(),
            asset,
            source_text,
            source_text_sha256,
            provenance_json,
        };
        binding.validate_for(&contrast).map_err(|e| e.to_string())?;
        self.sound_append(&[reference, attachment])?;
        Ok(binding)
    }
    pub(crate) fn start_sound_focus(
        &mut self,
        definition: &rufren_content::SoundContrastDefinition,
        references: [SoundReferenceBinding; 2],
    ) -> Result<SoundFocusDraft> {
        self.reconcile_daily_plan(now_ms())?;
        let contrast = snapshot(definition)?;
        if let Some(old) = self.sound_stored()? {
            if !old.historical
                && old.phase != SoundFocusPhase::Finished
                && old.contrast == contrast
                && old.references == references
            {
                self.resume_sound_focus(old.session_id)?;
                return self.sound_focus_draft()?.ok_or_else(fail);
            }
        }
        self.note_reference_access()?;
        self.sound_archive_active()?;
        let session = Uuid::new_v4();
        let mut cells = vec![(0usize, false), (0, true), (1, false), (1, true)];
        // UUID entropy is persisted as a complete shuffled block, never recomputed on reload.
        let mut ranked: Vec<_> = cells.drain(..).map(|cell| (Uuid::new_v4(), cell)).collect();
        ranked.sort_by_key(|x| x.0);
        let mut events = Vec::new();
        for (position, (_, (target, reverse))) in ranked.into_iter().enumerate() {
            let order = if reverse {
                [
                    contrast.members[1].id.clone(),
                    contrast.members[0].id.clone(),
                ]
            } else {
                [
                    contrast.members[0].id.clone(),
                    contrast.members[1].id.clone(),
                ]
            };
            events.push(
                Event::new(
                    self.device,
                    now_ms(),
                    Fact::SoundTrialPrepared(SoundTrialPrepared {
                        trial_id: Uuid::new_v4(),
                        token: Uuid::new_v4(),
                        block_id: session,
                        position,
                        mode: SoundTrialMode::Listening,
                        contrast: contrast.clone(),
                        target_member_id: contrast.members[target].id.clone(),
                        option_member_ids: order,
                        references: references.clone(),
                    }),
                )
                .map_err(|e| e.to_string())?,
            );
        }
        let stored = StoredFocus {
            version: 1,
            session_id: session,
            contrast,
            references,
            prepared: events,
            current_trial: 0,
            phase: SoundFocusPhase::Examples,
            speaking: None,
            selected_take: None,
            cues: BTreeMap::new(),
            historical: false,
            response_packets: BTreeMap::new(),
        };
        self.sound_save(&stored)?;
        self.resume_sound_focus(session)?;
        self.sound_focus_draft()?.ok_or_else(fail)
    }
    pub(crate) fn resume_sound_focus(&mut self, session: Uuid) -> Result<()> {
        self.reconcile_daily_plan(now_ms())?;
        let stored = match self.sound_stored()? {
            Some(s) if s.session_id == session && !s.historical => s,
            _ => {
                self.sound_archive_active()?;
                self.repo
                    .get::<StoredFocus>(&key(session))?
                    .ok_or("That saved sound practice is unavailable.")?
            }
        };
        stored.validate()?;
        let mut events = stored.prepared.clone();
        if let Some(e) = &stored.speaking {
            events.push(e.clone());
        }
        self.sound_append(&events)?;
        self.sound_save(&stored)?;
        self.repo.set(SURFACE, &"sound_focus")?;
        self.page = Page::Practice;
        Ok(())
    }
    pub(crate) fn paused_sound_focus(&self) -> Result<Vec<(Uuid, String)>> {
        let active = self
            .sound_stored()?
            .filter(|s| !s.historical)
            .map(|s| s.session_id);
        let ids: BTreeSet<Uuid> = self.repo.get(PAUSED)?.unwrap_or_default();
        let mut saved = Vec::new();
        for id in ids {
            if Some(id) == active {
                continue;
            }
            if let Some(s) = self.repo.get::<StoredFocus>(&key(id))? {
                s.validate()?;
                if !s.historical && s.phase != SoundFocusPhase::Finished {
                    saved.push((id, s.contrast.title));
                }
            }
        }
        Ok(saved)
    }
    pub(crate) fn pause_sound_focus(&mut self) -> Result<()> {
        self.sound_archive_active()?;
        self.select_ordinary_practice_surface()?;
        self.page = Page::Today;
        Ok(())
    }
    fn sound_completed(&self, trial: Uuid) -> Option<(Uuid, SoundTrialCompleted)> {
        let mut answers = BTreeMap::new();
        for (_, c) in self.library.sound_trial_completion(trial) {
            if self.library.attempt(c.attempt).is_some()
                && self.library.events.contains_key(&c.context)
            {
                answers.insert(c.attempt, c.clone());
            }
        }
        if answers.len() == 1 {
            answers.into_iter().next()
        } else {
            None
        }
    }
    fn sound_has_response(&self, trial: Uuid) -> bool {
        !self.library.sound_trial_completion(trial).is_empty()
    }
    fn sound_plays(&self, scope: &SoundPlaybackScope) -> Vec<(&Event, &SoundReferencePlayed)> {
        self.library
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::SoundReferencePlayed(p) if &p.scope == scope => Some((e, p)),
                _ => None,
            })
            .collect()
    }
    fn sound_natural_complete(&self, scope: &SoundPlaybackScope) -> bool {
        self.sound_plays(scope).iter().filter(|(e,_)|e.recorded_at_ms<=now_ms()).any(|(_,p)| {
            let exact=match scope {
                SoundPlaybackScope::Trial {prepared:id}=>self.library.events.get(id).and_then(|e|prepared(e).ok()).is_some_and(|trial|trial.token==p.token&&trial.references.iter().find(|r|r.member_id==trial.target_member_id)==Some(&p.reference)),
                SoundPlaybackScope::Example {block_id,member_id}=>p.token==*block_id&&p.reference.member_id==*member_id,
            };
            let SoundPlaybackStage::Completed {started_event}=p.stage else{return false;};
            exact && p.natural_speed && self.library.events.get(&started_event).is_some_and(|e|matches!(&e.fact,Fact::SoundReferencePlayed(start) if start.scope==p.scope&&start.token==p.token&&start.reference==p.reference&&start.natural_speed&&matches!(start.stage,SoundPlaybackStage::Started)))
        })
    }
    pub(crate) fn sound_focus_draft(&self) -> Result<Option<SoundFocusDraft>> {
        let Some(s) = self.sound_stored()? else {
            return Ok(None);
        };
        let mut trials = Vec::new();
        for event in &s.prepared {
            let p = prepared(event)?;
            let plays = self.sound_plays(&SoundPlaybackScope::Trial { prepared: event.id });
            let done = self.sound_completed(p.trial_id);
            let choice = done.as_ref().and_then(|(_, c)| match &c.response {
                SoundTrialResponse::Listening { chosen_member_id } => {
                    Some(chosen_member_id.clone())
                }
                _ => None,
            });
            trials.push(SoundListeningTrial {
                trial_id: p.trial_id,
                token: p.token,
                prepared_id: event.id,
                target_member_id: p.target_member_id.clone(),
                option_member_ids: p.option_member_ids.clone(),
                choice_member_id: choice,
                saved_attempt: done.as_ref().map(|d| d.0),
                can_answer: self
                    .sound_natural_complete(&SoundPlaybackScope::Trial { prepared: event.id }),
                play_count: plays
                    .iter()
                    .filter(|(_, p)| matches!(p.stage, SoundPlaybackStage::Started))
                    .count(),
                conflicted: self
                    .library
                    .sound_trial_completion(p.trial_id)
                    .iter()
                    .map(|(_, c)| c.attempt)
                    .collect::<BTreeSet<_>>()
                    .len()
                    > 1,
                incomplete: self
                    .library
                    .sound_trials()
                    .iter()
                    .find(|t| t.trial_id == p.trial_id)
                    .is_none_or(|t| t.incomplete),
                revealed: s
                    .cues
                    .get(&p.trial_id)
                    .is_some_and(|c| c.contains(&Cue::OriginalText)),
            });
        }
        let mut labelled = [false, false];
        for (i, m) in s.contrast.members.iter().enumerate() {
            labelled[i] = self.sound_natural_complete(&SoundPlaybackScope::Example {
                block_id: s.session_id,
                member_id: m.id.clone(),
            });
        }
        let speaking = s
            .speaking
            .as_ref()
            .map(|event| {
                let p = prepared(event)?;
                let m = s
                    .contrast
                    .members
                    .iter()
                    .find(|m| m.id == p.target_member_id)
                    .ok_or_else(fail)?;
                let (passage, revision) =
                    sound_reference_identity(&m.utterance.id, &m.utterance.version);
                let historical_take =
                    self.sound_completed(p.trial_id)
                        .and_then(|(_, c)| match c.response {
                            SoundTrialResponse::Speaking {
                                asset_id,
                                original_recording,
                                ..
                            } => Some((asset_id, original_recording)),
                            _ => None,
                        });
                Ok::<SoundSpeakingIntent, String>(SoundSpeakingIntent {
                    intent_id: p.trial_id,
                    token: p.token,
                    prepared_id: event.id,
                    member_id: p.target_member_id.clone(),
                    passage_id: passage,
                    revision_id: revision,
                    selected_asset_id: s
                        .selected_take
                        .as_ref()
                        .map(|t| t.metadata.asset_id.clone())
                        .or_else(|| historical_take.as_ref().map(|t| t.0.clone())),
                    selected_attachment: s
                        .selected_take
                        .as_ref()
                        .map(|t| t.attachment)
                        .or_else(|| historical_take.as_ref().map(|t| t.1)),
                    saved_attempt: self.sound_completed(p.trial_id).map(|d| d.0),
                    selected_interrupted: s
                        .selected_take
                        .as_ref()
                        .is_some_and(|t| t.metadata.interrupted),
                })
            })
            .transpose()?;
        Ok(Some(SoundFocusDraft {
            session_id: s.session_id,
            contrast: s.contrast,
            references: s.references,
            trials,
            current_trial: s.current_trial,
            phase: s.phase,
            labelled_examples_done: labelled,
            speaking,
            historical: s.historical,
            history_incomplete: self
                .library
                .sound_trials()
                .iter()
                .any(|t| t.block_id == s.session_id && t.incomplete),
        }))
    }
    fn sound_current<'a>(s: &'a StoredFocus, token: Uuid) -> Result<&'a Event> {
        let event = match s.phase {
            SoundFocusPhase::Listening => s.prepared.get(s.current_trial),
            SoundFocusPhase::Speaking => s.speaking.as_ref(),
            _ => None,
        }
        .ok_or_else(fail)?;
        if prepared(event)?.token != token {
            return Err(fail());
        }
        Ok(event)
    }
    fn sound_reference_ready(&self, r: &SoundReferenceBinding) -> Result<()> {
        let Some(Fact::RecordingAttached(a)) =
            self.library.events.get(&r.recording).map(|e| &e.fact)
        else {
            return Err("Sync the exact sound reference links before continuing.".into());
        };
        if a.target != AnnotationTarget::Revision(r.revision)
            || a.role != AudioRole::Reference
            || a.asset_id != r.asset_id
            || a.asset != r.asset
            || self.library.revision(r.revision).is_none()
        {
            return Err(fail());
        }
        Ok(())
    }
    fn sound_play_started(
        &mut self,
        mut s: StoredFocus,
        scope: SoundPlaybackScope,
        token: Uuid,
        r: SoundReferenceBinding,
        metadata: &AudioRecordMetadata,
    ) -> Result<Uuid> {
        check_metadata(&r, metadata)?;
        self.sound_reference_ready(&r)?;
        let now = now_ms();
        let event = Event::new(
            self.device,
            now,
            Fact::SoundReferencePlayed(SoundReferencePlayed {
                scope: scope.clone(),
                token,
                reference: r.clone(),
                stage: SoundPlaybackStage::Started,
                natural_speed: true,
            }),
        )
        .map_err(|e| e.to_string())?;
        let (session, purpose) = match scope {
            SoundPlaybackScope::Example { block_id, .. } => {
                (block_id, PresentationPurpose::Reference)
            }
            SoundPlaybackScope::Trial { prepared: id } => {
                let p = s
                    .prepared
                    .iter()
                    .chain(s.speaking.iter())
                    .find(|e| e.id == id)
                    .and_then(|e| prepared(e).ok())
                    .ok_or_else(fail)?;
                let count = self
                    .sound_plays(&SoundPlaybackScope::Trial { prepared: id })
                    .iter()
                    .filter(|(_, p)| matches!(p.stage, SoundPlaybackStage::Started))
                    .count();
                if !self.sound_has_response(p.trial_id)
                    && (p.mode == SoundTrialMode::Speaking || count > 0)
                {
                    let cues = s.cues.entry(p.trial_id).or_default();
                    if !cues.contains(&Cue::Replay) {
                        cues.push(Cue::Replay);
                    }
                }
                (
                    p.trial_id,
                    if p.mode == SoundTrialMode::Listening && count == 0 {
                        PresentationPurpose::TaskInput
                    } else {
                        PresentationPurpose::Reference
                    },
                )
            }
        };
        let shown = Event::new(
            self.device,
            now,
            Fact::StimulusPresented(StimulusPresentation {
                session: Some(session),
                revision: Some(r.revision),
                russian: r.source_text,
                purpose,
                decision: None,
                utterance_family: Some(format!("sound:{}@{}", s.contrast.id, s.contrast.version)),
            }),
        )
        .map_err(|e| e.to_string())?;
        self.sound_append(&[event.clone(), shown])?;
        self.sound_save(&s)?;
        Ok(event.id)
    }
    fn sound_play_completed(
        &mut self,
        s: StoredFocus,
        scope: SoundPlaybackScope,
        token: Uuid,
        start: Uuid,
        r: SoundReferenceBinding,
        metadata: &AudioRecordMetadata,
    ) -> Result<()> {
        check_metadata(&r, metadata)?;
        self.refresh()?;
        self.sound_reference_ready(&r)?;
        let Some(Fact::SoundReferencePlayed(started)) =
            self.library.events.get(&start).map(|e| &e.fact)
        else {
            return Err("The beginning of that playback was not saved. Play this reference again before answering.".into());
        };
        if started.scope != scope
            || started.token != token
            || started.reference != r
            || !started.natural_speed
            || !matches!(started.stage, SoundPlaybackStage::Started)
        {
            return Err(fail());
        }
        let id = Uuid::new_v5(&start, b"sound-playback-completed-v1");
        if self.library.events.contains_key(&id) {
            return Ok(());
        }
        let mut event = Event::new(
            self.device,
            now_ms(),
            Fact::SoundReferencePlayed(SoundReferencePlayed {
                scope,
                token,
                reference: r,
                stage: SoundPlaybackStage::Completed {
                    started_event: start,
                },
                natural_speed: true,
            }),
        )
        .map_err(|e| e.to_string())?;
        event.id = id;
        self.sound_append(&[event])?;
        self.sound_save(&s)
    }
    pub(crate) fn sound_focus_example_started(
        &mut self,
        session: Uuid,
        member_id: &str,
        metadata: &AudioRecordMetadata,
    ) -> Result<Uuid> {
        let s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Examples {
            return Err(fail());
        }
        let r = s
            .references
            .iter()
            .find(|r| r.member_id == member_id)
            .ok_or_else(fail)?
            .clone();
        self.sound_play_started(
            s,
            SoundPlaybackScope::Example {
                block_id: session,
                member_id: member_id.into(),
            },
            session,
            r,
            metadata,
        )
    }
    pub(crate) fn sound_focus_example_completed(
        &mut self,
        session: Uuid,
        member_id: &str,
        start: Uuid,
        metadata: &AudioRecordMetadata,
    ) -> Result<()> {
        let s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Examples {
            return Err(fail());
        }
        let r = s
            .references
            .iter()
            .find(|r| r.member_id == member_id)
            .ok_or_else(fail)?
            .clone();
        self.sound_play_completed(
            s,
            SoundPlaybackScope::Example {
                block_id: session,
                member_id: member_id.into(),
            },
            session,
            start,
            r,
            metadata,
        )
    }
    pub(crate) fn sound_focus_playback_started(
        &mut self,
        session: Uuid,
        token: Uuid,
        metadata: &AudioRecordMetadata,
    ) -> Result<Uuid> {
        let s = self.sound_guard(session)?;
        let event = Self::sound_current(&s, token)?;
        let p = prepared(event)?;
        let r = s
            .references
            .iter()
            .find(|r| r.member_id == p.target_member_id)
            .ok_or_else(fail)?
            .clone();
        let scope = SoundPlaybackScope::Trial { prepared: event.id };
        self.sound_play_started(s, scope, token, r, metadata)
    }
    pub(crate) fn sound_focus_playback_completed(
        &mut self,
        session: Uuid,
        token: Uuid,
        start: Uuid,
        metadata: &AudioRecordMetadata,
    ) -> Result<()> {
        let s = self.sound_guard(session)?;
        let event = Self::sound_current(&s, token)?;
        let p = prepared(event)?;
        let r = s
            .references
            .iter()
            .find(|r| r.member_id == p.target_member_id)
            .ok_or_else(fail)?
            .clone();
        let scope = SoundPlaybackScope::Trial { prepared: event.id };
        self.sound_play_completed(s, scope, token, start, r, metadata)
    }
    pub(crate) fn sound_focus_continue(&mut self, session: Uuid) -> Result<()> {
        self.refresh()?;
        let mut s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Examples {
            return Err(fail());
        }
        if !s.contrast.members.iter().all(|m| {
            self.sound_natural_complete(&SoundPlaybackScope::Example {
                block_id: session,
                member_id: m.id.clone(),
            })
        }) {
            return Err(
                "Hear both labelled examples at natural speed before trying the meanings.".into(),
            );
        }
        s.phase = SoundFocusPhase::Listening;
        self.sound_save(&s)
    }
    pub(crate) fn sound_focus_support(
        &mut self,
        session: Uuid,
        token: Uuid,
        cue: Cue,
    ) -> Result<()> {
        let mut s = self.sound_guard(session)?;
        let trial = prepared(Self::sound_current(&s, token)?)?.trial_id;
        if self.sound_completed(trial).is_some() {
            return Err(fail());
        }
        let cues = s.cues.entry(trial).or_default();
        if !cues.contains(&cue) {
            cues.push(cue);
        }
        self.sound_save(&s)
    }
    pub(crate) fn sound_focus_reveal(&mut self, session: Uuid, token: Uuid) -> Result<()> {
        self.sound_focus_support(session, token, Cue::OriginalText)?;
        self.sound_focus_support(session, token, Cue::Translation)
    }
    pub(crate) fn sound_focus_mark_active_support(&mut self, cue: Cue) -> Result<()> {
        if !self.sound_focus_is_selected()? {
            return Ok(());
        }
        let Some(s) = self.sound_stored()? else {
            return Ok(());
        };
        if s.historical {
            return Ok(());
        }
        let token = match s.phase {
            SoundFocusPhase::Listening => s
                .prepared
                .get(s.current_trial)
                .and_then(|e| prepared(e).ok())
                .map(|p| p.token),
            SoundFocusPhase::Speaking => s
                .speaking
                .as_ref()
                .and_then(|e| prepared(e).ok())
                .map(|p| p.token),
            _ => None,
        };
        if let Some(token) = token {
            if self
                .sound_completed(prepared(Self::sound_current(&s, token)?)?.trial_id)
                .is_none()
            {
                self.sound_focus_support(s.session_id, token, cue)?;
            }
        }
        Ok(())
    }
    fn sound_response_events(
        &self,
        s: &StoredFocus,
        event: &Event,
        mut response: SoundTrialResponse,
        mut recording: Option<AudioAttachment>,
    ) -> Result<(Uuid, Vec<Event>)> {
        let p = prepared(event)?;
        let attempt = Uuid::new_v4();
        let context = Uuid::new_v5(&attempt, b"sound-context-v1");
        let completed = Uuid::new_v5(&attempt, b"sound-trial-completed-v1");
        let recording_id = Uuid::new_v5(&attempt, b"sound-attempt-recording-v1");
        if let SoundTrialResponse::Speaking {
            attempt_recording, ..
        } = &mut response
        {
            *attempt_recording = recording_id;
        }
        if let Some(r) = &mut recording {
            r.target = AnnotationTarget::Attempt(attempt);
        }
        let r = p
            .references
            .iter()
            .find(|r| r.member_id == p.target_member_id)
            .ok_or_else(fail)?;
        let mut cues = s.cues.get(&p.trial_id).cloned().unwrap_or_default();
        let plays = self.sound_plays(&SoundPlaybackScope::Trial { prepared: event.id });
        let starts = plays
            .iter()
            .filter(|(_, p)| matches!(p.stage, SoundPlaybackStage::Started))
            .count();
        if (p.mode == SoundTrialMode::Speaking && starts > 0) || starts > 1 {
            if !cues.contains(&Cue::Replay) {
                cues.push(Cue::Replay);
            }
        }
        let supports: Vec<_> = self
            .library
            .presentations()
            .filter(|(_, shown)| {
                shown.session == Some(p.trial_id)
                    && matches!(
                        shown.purpose,
                        PresentationPurpose::Answer
                            | PresentationPurpose::Reference
                            | PresentationPurpose::Instruction
                    )
                    && p.references
                        .iter()
                        .any(|r| Some(r.revision) == shown.revision)
            })
            .map(|(e, _)| e.id)
            .collect();
        if !supports.is_empty() && !cues.contains(&Cue::ReferenceMaterial) {
            cues.push(Cue::ReferenceMaterial);
        }
        let (skill, task, taskdef, literal) = match &response {
            SoundTrialResponse::Listening { chosen_member_id } => {
                let m = p
                    .contrast
                    .members
                    .iter()
                    .find(|m| &m.id == chosen_member_id)
                    .ok_or_else(fail)?;
                (
                    Skill::Listening,
                    TaskKind::SoundDiscrimination,
                    &p.contrast.identify_task,
                    m.option_label.clone(),
                )
            }
            SoundTrialResponse::Speaking { asset_id, .. } => (
                Skill::Speaking,
                TaskKind::Pronunciation,
                &p.contrast.produce_task,
                format!("Sound Focus recording · {asset_id}"),
            ),
        };
        let prompt = format!(
            "{}\nMeanings in displayed order:\n{}\n{}",
            taskdef.instruction,
            p.contrast
                .members
                .iter()
                .find(|m| m.id == p.option_member_ids[0])
                .ok_or_else(fail)?
                .option_label,
            p.contrast
                .members
                .iter()
                .find(|m| m.id == p.option_member_ids[1])
                .ok_or_else(fail)?
                .option_label
        );
        let intended = p
            .contrast
            .members
            .iter()
            .find(|m| m.id == p.target_member_id)
            .ok_or_else(fail)?;
        let now = now_ms();
        let make = |id, fact| Event {
            version: EVENT_VERSION,
            id,
            device: self.device,
            recorded_at_ms: now,
            fact,
        };
        let mut events = vec![
            make(
                attempt,
                Fact::AttemptRecorded(Attempt {
                    session: p.trial_id,
                    revision: r.revision,
                    skill,
                    response: literal,
                    cues: cues.clone(),
                    self_rating: SelfRating::NotReported,
                    policy: POLICY.into(),
                }),
            ),
            make(
                context,
                Fact::AttemptContextRecorded(AttemptContext {
                    attempt,
                    content_pack: None,
                    content_revision: None,
                    exercise_id: Some(format!("{}@{}", taskdef.id, taskdef.version)),
                    prompt: Some(prompt),
                    reference_answer: Some(format!(
                        "Intended label: {}\nExact reference text: {}",
                        intended.option_label, intended.utterance.text
                    )),
                    task,
                    target_features: BTreeSet::from([p.contrast.target_id.clone()]),
                    novel_context: false,
                    delayed_probe: false,
                    practice_duration_ms: None,
                    effort: None,
                    grammar_response: None,
                    supersedes: BTreeSet::new(),
                }),
            ),
        ];
        if let Some(recording) = recording {
            events.push(make(recording_id, Fact::RecordingAttached(recording)));
        }
        events.push(make(
            completed,
            Fact::SoundTrialCompleted(SoundTrialCompleted {
                prepared: event.id,
                attempt,
                context,
                response,
                known_playback_ids: plays.into_iter().take(64).map(|(e, _)| e.id).collect(),
                support_presentation_ids: supports.into_iter().take(64).collect(),
                revealed: cues.contains(&Cue::OriginalText) || cues.contains(&Cue::Translation),
            }),
        ));
        Ok((attempt, events))
    }
    fn sound_commit_response(&mut self, events: Vec<Event>) -> Result<()> {
        self.refresh()?;
        let mut missing = Vec::new();
        for e in events {
            if let Some(old) = self.library.events.get(&e.id) {
                if old.fact != e.fact {
                    return Err(
                        "This sound response was already saved differently. Its original is kept."
                            .into(),
                    );
                }
            } else {
                missing.push(e);
            }
        }
        if !missing.is_empty() {
            self.repo.append(&missing)?;
            self.refresh()?;
        }
        Ok(())
    }
    fn sound_commit_packet(
        &mut self,
        mut stored: StoredFocus,
        trial: Uuid,
        new: Option<Vec<Event>>,
    ) -> Result<Uuid> {
        if !stored.response_packets.contains_key(&trial) {
            let events = new.ok_or_else(fail)?;
            stored.response_packets.insert(trial, events);
            self.sound_save(&stored)?; // Preserve exact answer/IDs before even a partial event append.
        }
        let packet = stored
            .response_packets
            .get(&trial)
            .ok_or_else(fail)?
            .clone();
        let attempt = packet
            .iter()
            .find_map(|e| matches!(e.fact, Fact::AttemptRecorded(_)).then_some(e.id))
            .ok_or_else(fail)?;
        self.sound_commit_response(packet)?;
        Ok(attempt)
    }
    pub(crate) fn sound_focus_answer(
        &mut self,
        session: Uuid,
        token: Uuid,
        member_id: &str,
    ) -> Result<Uuid> {
        self.refresh()?;
        let s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Listening {
            return Err(fail());
        }
        let event = Self::sound_current(&s, token)?;
        let p = prepared(event)?;
        if let Some(packet) = s.response_packets.get(&p.trial_id) {
            let same=packet.iter().any(|e|matches!(&e.fact,Fact::SoundTrialCompleted(c) if matches!(&c.response,SoundTrialResponse::Listening {chosen_member_id} if chosen_member_id==member_id)));
            if !same {
                return Err(
                    "Your first choice is already queued. It remains unchanged on retry.".into(),
                );
            }
            return self.sound_commit_packet(s.clone(), p.trial_id, None);
        }
        if let Some((attempt, c)) = self.sound_completed(p.trial_id) {
            if matches!(c.response,SoundTrialResponse::Listening {chosen_member_id} if chosen_member_id==member_id)
            {
                return Ok(attempt);
            }
            return Err("That choice is already saved. Continue for another trial.".into());
        }
        if self.sound_has_response(p.trial_id) {
            return Err("This trial has incomplete or competing saved choices. Open its history; a new choice cannot replace them.".into());
        }
        if !self.sound_natural_complete(&SoundPlaybackScope::Trial { prepared: event.id }) {
            return Err(
                "Let this exact natural-speed reference finish before choosing a meaning.".into(),
            );
        }
        if !p.option_member_ids.iter().any(|id| id == member_id) {
            return Err(fail());
        }
        for r in &p.references {
            self.sound_reference_ready(r)?;
        }
        let (attempt, events) = self.sound_response_events(
            &s,
            event,
            SoundTrialResponse::Listening {
                chosen_member_id: member_id.into(),
            },
            None,
        )?;
        let _ = attempt;
        self.sound_commit_packet(s.clone(), p.trial_id, Some(events))
    }
    pub(crate) fn sound_focus_next(&mut self, session: Uuid) -> Result<()> {
        self.refresh()?;
        let mut s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Listening {
            return Err(fail());
        }
        let p = prepared(s.prepared.get(s.current_trial).ok_or_else(fail)?)?;
        if !self.sound_has_response(p.trial_id) {
            return Err(
                "Keep a choice before continuing. Your current trial stays the same.".into(),
            );
        }
        if s.current_trial < 3 {
            s.current_trial += 1;
        } else {
            s.phase = SoundFocusPhase::Speaking;
        }
        self.sound_save(&s)
    }
    pub(crate) fn sound_focus_intend(&mut self, session: Uuid, member_id: &str) -> Result<()> {
        self.refresh()?;
        let mut s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Speaking
            || !s.contrast.members.iter().any(|m| m.id == member_id)
        {
            return Err(fail());
        }
        if let Some(e) = &s.speaking {
            let trial = prepared(e)?;
            if s.response_packets.contains_key(&trial.trial_id)
                || self.sound_has_response(trial.trial_id)
            {
                return Err("This spoken response is already kept or queued for saving. Start a new Sound practice for another response; this intent and its original stay unchanged.".into());
            }
            if prepared(e)?.target_member_id == member_id
                && self.sound_completed(prepared(e)?.trial_id).is_none()
            {
                return Ok(());
            }
        }
        let event = Event::new(
            self.device,
            now_ms(),
            Fact::SoundTrialPrepared(SoundTrialPrepared {
                trial_id: Uuid::new_v4(),
                token: Uuid::new_v4(),
                block_id: session,
                position: 4,
                mode: SoundTrialMode::Speaking,
                contrast: s.contrast.clone(),
                target_member_id: member_id.into(),
                option_member_ids: [
                    s.contrast.members[0].id.clone(),
                    s.contrast.members[1].id.clone(),
                ],
                references: s.references.clone(),
            }),
        )
        .map_err(|e| e.to_string())?;
        s.speaking = Some(event.clone());
        s.selected_take = None;
        self.sound_save(&s)?;
        self.sound_append(&[event])
    }
    fn sound_check_take(
        &self,
        s: &StoredFocus,
        intent: Uuid,
        metadata: &AudioRecordMetadata,
        explicit_partial: bool,
    ) -> Result<Uuid> {
        let e = s
            .speaking
            .as_ref()
            .ok_or("Choose the intended meaning before recording.")?;
        let p = prepared(e)?;
        metadata.validate().map_err(|e| e.to_string())?;
        let scope = metadata.device.get("sound_focus_scope").ok_or_else(fail)?;
        let (passage, revision) = sound_reference_identity(
            &s.contrast
                .member(&p.target_member_id)
                .ok_or_else(fail)?
                .utterance
                .id,
            &s.contrast
                .member(&p.target_member_id)
                .ok_or_else(fail)?
                .utterance
                .version,
        );
        if p.trial_id != intent
            || metadata.role != "attempt"
            || metadata.created_at_ms < e.recorded_at_ms
            || metadata.created_at_ms > now_ms()
            || metadata.passage_id != passage.to_string()
            || metadata.revision_id != revision.to_string()
            || scope.get("session_id").and_then(|v| v.as_str())
                != Some(session_string(&s.session_id).as_str())
            || scope.get("intent_id").and_then(|v| v.as_str())
                != Some(session_string(&intent).as_str())
            || scope.get("member_id").and_then(|v| v.as_str()) != Some(p.target_member_id.as_str())
            || scope.get("origin").and_then(|v| v.as_str()) != Some("microphone_capture")
            || (metadata.interrupted && !explicit_partial)
        {
            return Err("Choose a fresh microphone take made for this exact intended meaning. Earlier and imported originals are kept separately.".into());
        }
        if self.library.events.values().any(|e|matches!(&e.fact,Fact::RecordingAttached(a) if a.role==AudioRole::Attempt&&a.asset.sha256==metadata.content_hash&&matches!(a.target,AnnotationTarget::Attempt(_)))) {
            return Err("That original has already been used as a response. Record a fresh take for this meaning.".into());
        }
        Ok(revision)
    }
    pub(crate) fn register_sound_focus_take(
        &mut self,
        session: Uuid,
        intent: Uuid,
        metadata: &AudioRecordMetadata,
    ) -> Result<Uuid> {
        let s = self.sound_guard(session)?;
        let revision = self.sound_check_take(&s, intent, metadata, true)?;
        let id = Uuid::new_v5(
            &revision,
            format!("audio:{}:attempt", metadata.asset_id).as_bytes(),
        );
        let event = Event {
            version: 6,
            id,
            device: Uuid::new_v5(&revision, b"audio-attachment-writer"),
            recorded_at_ms: metadata.created_at_ms,
            fact: Fact::RecordingAttached(AudioAttachment {
                asset_id: metadata.asset_id.clone(),
                asset: metadata_asset(metadata),
                target: AnnotationTarget::Revision(revision),
                role: AudioRole::Attempt,
                note: if metadata.interrupted {
                    "Recording interrupted; captured portion preserved".into()
                } else {
                    String::new()
                },
            }),
        };
        self.sound_append(&[event])?;
        Ok(id)
    }
    pub(crate) fn sound_focus_select_take(
        &mut self,
        session: Uuid,
        intent: Uuid,
        selected: Option<(Uuid, AudioRecordMetadata)>,
        explicit_partial: bool,
    ) -> Result<()> {
        self.refresh()?;
        let mut s = self.sound_guard(session)?;
        let p = prepared(s.speaking.as_ref().ok_or_else(fail)?)?;
        if p.trial_id != intent
            || self.sound_has_response(intent)
            || s.response_packets.contains_key(&intent)
        {
            return Err(fail());
        }
        s.selected_take = if let Some((attachment, metadata)) = selected {
            let revision = self.sound_check_take(&s, intent, &metadata, explicit_partial)?;
            let Some(Fact::RecordingAttached(a)) =
                self.library.events.get(&attachment).map(|e| &e.fact)
            else {
                return Err("Keep this original before selecting it.".into());
            };
            if a.target != AnnotationTarget::Revision(revision)
                || a.role != AudioRole::Attempt
                || a.asset_id != metadata.asset_id
                || a.asset != metadata_asset(&metadata)
            {
                return Err(fail());
            }
            Some(SelectedTake {
                attachment,
                metadata,
                explicit_partial,
            })
        } else {
            None
        };
        self.sound_save(&s)
    }
    pub(crate) fn sound_focus_keep_take(&mut self, session: Uuid, intent: Uuid) -> Result<Uuid> {
        self.refresh()?;
        let s = self.sound_guard(session)?;
        if s.phase != SoundFocusPhase::Speaking {
            return Err(fail());
        }
        let e = s.speaking.as_ref().ok_or_else(fail)?;
        let p = prepared(e)?;
        if p.trial_id != intent {
            return Err(fail());
        }
        if s.response_packets.contains_key(&intent) {
            return self.sound_commit_packet(s.clone(), intent, None);
        }
        if let Some((attempt, _)) = self.sound_completed(intent) {
            return Ok(attempt);
        }
        let take = s
            .selected_take
            .as_ref()
            .ok_or("Choose the exact fresh take to keep.")?;
        self.sound_check_take(&s, intent, &take.metadata, take.explicit_partial)?;
        let Some(Fact::RecordingAttached(original)) =
            self.library.events.get(&take.attachment).map(|e| &e.fact)
        else {
            return Err(fail());
        };
        let mut copy = original.clone();
        copy.target = AnnotationTarget::Attempt(Uuid::new_v5(&intent, b"attempt"));
        let response = SoundTrialResponse::Speaking {
            intended_member_id: p.target_member_id.clone(),
            original_recording: take.attachment,
            attempt_recording: Uuid::new_v5(&intent, b"recording"),
            capture_scope: SoundCaptureScope {
                session_id: session,
                intent_id: intent,
                member_id: p.target_member_id.clone(),
            },
            captured_at_ms: take.metadata.created_at_ms,
            asset_id: take.metadata.asset_id.clone(),
            asset: metadata_asset(&take.metadata),
        };
        let (attempt, events) = self.sound_response_events(&s, e, response, Some(copy))?;
        let _ = attempt;
        self.sound_commit_packet(s.clone(), intent, Some(events))
    }
    pub(crate) fn finish_sound_focus(&mut self, session: Uuid) -> Result<()> {
        self.refresh()?;
        let mut s = self.sound_guard(session)?;
        let p = prepared(
            s.speaking
                .as_ref()
                .ok_or("Choose a meaning and keep a fresh take before finishing.")?,
        )?;
        if self.sound_completed(p.trial_id).is_none() {
            return Err("Keep your actual spoken response before finishing. A transcript is optional and is not a sound grade.".into());
        }
        s.phase = SoundFocusPhase::Finished;
        self.sound_save(&s)
    }
    /// Reopen the actual frozen block and responses from replicated facts alone. No fresh token,
    /// response, exposure or completion is invented when the device has no workspace metadata.
    pub(crate) fn open_sound_focus_history(&mut self, block: Uuid) -> Result<()> {
        self.refresh()?;
        self.reconcile_daily_plan(now_ms())?;
        let mut listening = BTreeMap::<usize, Event>::new();
        let mut speaking = Vec::new();
        for event in self.library.events.values() {
            if let Fact::SoundTrialPrepared(p) = &event.fact {
                if p.block_id == block {
                    match p.mode {
                        SoundTrialMode::Listening => {
                            listening.entry(p.position).or_insert_with(|| event.clone());
                        }
                        SoundTrialMode::Speaking => {
                            speaking.push(event.clone());
                        }
                    }
                }
            }
        }
        let events: Vec<_> = listening.into_values().collect();
        let first = prepared(
            events
                .first()
                .or_else(|| speaking.first())
                .ok_or("This saved sound block has no available prepared trial yet.")?,
        )?;
        let mut stored = StoredFocus {
            version: 1,
            session_id: block,
            contrast: first.contrast.clone(),
            references: first.references.clone(),
            prepared: events,
            current_trial: 0,
            phase: SoundFocusPhase::Finished,
            speaking: if speaking.len() == 1 {
                speaking.pop()
            } else {
                None
            },
            selected_take: None,
            cues: BTreeMap::new(),
            historical: true,
            response_packets: BTreeMap::new(),
        };
        self.note_reference_access()?;
        self.sound_archive_active()?;
        stored.historical = true;
        self.sound_save(&stored)?;
        self.repo.set(SURFACE, &"sound_focus")?;
        self.page = Page::Practice;
        Ok(())
    }
}
fn session_string(id: &Uuid) -> String {
    id.to_string()
}
