//! Immutable Sound Focus definitions, exact originals and durable trial evidence.
//! Preparation is not exposure; a nominal listening match is not an accent grade.
use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const REFERENCE_PREFIX: &str = "Rufren Sound Focus reference · ";
const POLICY: &str = "sound-focus-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundUtteranceShape {
    IsolatedWord,
    CarrierSentence,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundUtteranceSnapshot {
    pub id: String,
    pub version: String,
    pub text: String,
    pub meaning: String,
    pub kind: SoundUtteranceShape,
    pub related_phrase_id: String,
    pub related_phrase_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundMemberSnapshot {
    pub id: String,
    pub version: String,
    pub option_label: String,
    pub lexeme: String,
    pub supported_lexeme: String,
    pub utterance: SoundUtteranceSnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundTaskSnapshot {
    pub id: String,
    pub version: String,
    pub instruction: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundContrastSnapshot {
    pub id: String,
    pub version: String,
    pub lesson_id: String,
    pub target_id: String,
    pub title: String,
    pub teaching_note: String,
    pub detail_note: String,
    pub provenance_note: String,
    pub source_ids: Vec<String>,
    pub members: [SoundMemberSnapshot; 2],
    pub identify_task: SoundTaskSnapshot,
    pub produce_task: SoundTaskSnapshot,
    pub retired_exercise_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundReferenceBinding {
    pub member_id: String,
    pub revision: Uuid,
    pub recording: Uuid,
    pub asset_id: String,
    pub asset: AudioAsset,
    pub source_text: String,
    pub source_text_sha256: String,
    pub provenance_json: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundTrialMode {
    Listening,
    Speaking,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundTrialPrepared {
    pub trial_id: Uuid,
    pub token: Uuid,
    pub block_id: Uuid,
    pub position: usize,
    pub mode: SoundTrialMode,
    pub contrast: SoundContrastSnapshot,
    pub target_member_id: String,
    pub option_member_ids: [String; 2],
    pub references: [SoundReferenceBinding; 2],
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoundPlaybackScope {
    Example { block_id: Uuid, member_id: String },
    Trial { prepared: Uuid },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoundPlaybackStage {
    Started,
    Completed { started_event: Uuid },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundReferencePlayed {
    pub scope: SoundPlaybackScope,
    pub token: Uuid,
    pub reference: SoundReferenceBinding,
    pub stage: SoundPlaybackStage,
    pub natural_speed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundCaptureScope {
    pub session_id: Uuid,
    pub intent_id: Uuid,
    pub member_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoundTrialResponse {
    Listening {
        chosen_member_id: String,
    },
    Speaking {
        intended_member_id: String,
        original_recording: Uuid,
        attempt_recording: Uuid,
        capture_scope: SoundCaptureScope,
        captured_at_ms: i64,
        asset_id: String,
        asset: AudioAsset,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoundTrialCompleted {
    pub prepared: Uuid,
    pub attempt: Uuid,
    pub context: Uuid,
    pub response: SoundTrialResponse,
    pub known_playback_ids: Vec<Uuid>,
    pub support_presentation_ids: Vec<Uuid>,
    pub revealed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoundTrialSummary {
    pub trial_id: Uuid,
    pub block_id: Uuid,
    pub mode: SoundTrialMode,
    pub target_member_id: String,
    pub choice_member_id: Option<String>,
    pub intended_match: Option<bool>,
    pub assisted: bool,
    pub incomplete: bool,
}

fn invalid() -> Error {
    Error::Invalid("Sound Focus differs from its immutable definition, exact original, trial intent or actual attempt".into())
}
fn bounded(text: &str, max: usize) -> bool {
    !text.trim().is_empty()
        && text.len() <= max
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
}
fn identity(text: &str) -> bool {
    bounded(text, 128)
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn sha(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl SoundContrastSnapshot {
    pub fn member(&self, id: &str) -> Option<&SoundMemberSnapshot> {
        self.members.iter().find(|m| m.id == id)
    }
    pub fn validate(&self) -> Result<(), Error> {
        if ![&self.id, &self.version, &self.lesson_id, &self.target_id]
            .iter()
            .all(|s| identity(s))
            || !bounded(&self.title, 512)
            || !bounded(&self.teaching_note, 4096)
            || !bounded(&self.detail_note, 8192)
            || !bounded(&self.provenance_note, 4096)
            || self.source_ids.is_empty()
            || self.source_ids.len() > 16
            || self.source_ids.iter().any(|s| !identity(s))
            || self.retired_exercise_ids.len() > 32
            || self.retired_exercise_ids.iter().any(|s| !identity(s))
        {
            return Err(invalid());
        }
        let mut ids = BTreeSet::new();
        for task in [&self.identify_task, &self.produce_task] {
            if !identity(&task.id)
                || !identity(&task.version)
                || !bounded(&task.instruction, 4096)
                || !ids.insert(&task.id)
            {
                return Err(invalid());
            }
        }
        for m in &self.members {
            let u = &m.utterance;
            if !identity(&m.id)
                || !identity(&m.version)
                || !ids.insert(&m.id)
                || !bounded(&m.option_label, 256)
                || m.option_label
                    .chars()
                    .any(|c| ('\u{400}'..='\u{52f}').contains(&c))
                || !bounded(&m.lexeme, 512)
                || !bounded(&m.supported_lexeme, 512)
                || !identity(&u.id)
                || !identity(&u.version)
                || !ids.insert(&u.id)
                || !bounded(&u.text, 1024)
                || u.text.chars().count() > 256
                || !bounded(&u.meaning, 2048)
                || !identity(&u.related_phrase_id)
                || !identity(&u.related_phrase_revision)
            {
                return Err(invalid());
            }
        }
        if self.members[0].option_label == self.members[1].option_label
            || self.source_ids.iter().collect::<BTreeSet<_>>().len() != self.source_ids.len()
            || self
                .retired_exercise_ids
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.retired_exercise_ids.len()
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// Callers use validated authored IDs/versions (which contain no ':' separator).
pub fn sound_reference_identity(utterance_id: &str, version: &str) -> (Uuid, Uuid) {
    let passage = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("rufren:sound-reference:{utterance_id}:{version}").as_bytes(),
    );
    (passage, Uuid::new_v5(&passage, b"reference-v1"))
}
pub fn sound_reference_event(
    snapshot: &SoundContrastSnapshot,
    member_id: &str,
) -> Result<Event, Error> {
    snapshot.validate()?;
    let member = snapshot.member(member_id).ok_or_else(invalid)?;
    let u = &member.utterance;
    let (passage, id) = sound_reference_identity(&u.id, &u.version);
    Ok(Event {
        version: 6,
        id,
        device: Uuid::new_v5(&passage, b"sound-reference-writer"),
        recorded_at_ms: 0,
        fact: Fact::PassageRevised(Revision {
            parents: BTreeSet::new(),
            passage: Passage {
                id: passage,
                title: "Sound Focus reference".into(),
                russian: u.text.clone(),
                translation: u.meaning.clone(),
                source: format!("{REFERENCE_PREFIX}{}@{}", u.id, u.version),
                coach_reviewed: false,
            },
        }),
    })
}
pub fn is_sound_focus_reference(event: &Event) -> bool {
    matches!(&event.fact,Fact::PassageRevised(r) if r.passage.source.starts_with(REFERENCE_PREFIX))
}

impl SoundReferenceBinding {
    pub fn validate(&self) -> Result<(), Error> {
        self.asset.validate()?;
        if !identity(&self.member_id)
            || self.revision.is_nil()
            || self.recording.is_nil()
            || !bounded(&self.asset_id, 256)
            || self.asset_id.chars().any(char::is_control)
            || !bounded(&self.source_text, 1024)
            || self.source_text.chars().count() > 256
            || !sha(&self.source_text_sha256)
            || hash(&self.source_text) != self.source_text_sha256
            || self.provenance_json.len() > 8192
        {
            return Err(invalid());
        }
        let p: serde_json::Value =
            serde_json::from_str(&self.provenance_json).map_err(|_| invalid())?;
        if !matches!(
            p["kind"].as_str(),
            Some("ai_generated_speech" | "system_generated_speech")
        ) || p["source_text"].as_str() != Some(self.source_text.as_str())
            || p["source_text_sha256"].as_str() != Some(self.source_text_sha256.as_str())
            || ["provider", "model", "voice", "request_id"]
                .iter()
                .any(|k| p[*k].as_str().is_none_or(|s| !bounded(s, 256)))
        {
            return Err(invalid());
        }
        let mut pending = vec![&p];
        while let Some(value) = pending.pop() {
            match value {
                serde_json::Value::Object(map) => {
                    for (k, v) in map {
                        if matches!(
                            k.as_str(),
                            "acoustic_assessment"
                                | "accent_score"
                                | "pronunciation_score"
                                | "nativeness_score"
                        ) && !v.is_null()
                        {
                            return Err(invalid());
                        }
                        if matches!(
                            k.as_str(),
                            "native_verified" | "human_verified" | "coach_reviewed"
                        ) && v.as_bool() == Some(true)
                        {
                            return Err(invalid());
                        }
                        pending.push(v);
                    }
                }
                serde_json::Value::Array(values) => pending.extend(values),
                _ => (),
            }
        }
        Ok(())
    }
    pub fn validate_for(&self, snapshot: &SoundContrastSnapshot) -> Result<(), Error> {
        self.validate()?;
        let m = snapshot.member(&self.member_id).ok_or_else(invalid)?;
        if self.revision != sound_reference_identity(&m.utterance.id, &m.utterance.version).1
            || self.source_text != m.utterance.text
        {
            return Err(invalid());
        }
        Ok(())
    }
}

impl SoundTrialPrepared {
    pub fn validate(&self) -> Result<(), Error> {
        self.contrast.validate()?;
        if self.trial_id.is_nil()
            || self.token.is_nil()
            || self.block_id.is_nil()
            || !matches!(
                (self.mode, self.position),
                (SoundTrialMode::Listening, 0..=3) | (SoundTrialMode::Speaking, 4)
            )
            || self.contrast.member(&self.target_member_id).is_none()
            || self.option_member_ids[0] == self.option_member_ids[1]
            || self
                .option_member_ids
                .iter()
                .any(|id| self.contrast.member(id).is_none())
            || self.references[0].member_id == self.references[1].member_id
        {
            return Err(invalid());
        }
        for reference in &self.references {
            reference.validate_for(&self.contrast)?;
        }
        if serde_json::to_vec(self).map_err(|_| invalid())?.len() > 65_536 {
            return Err(invalid());
        }
        Ok(())
    }
    fn reference(&self, member: &str) -> Result<&SoundReferenceBinding, Error> {
        self.references
            .iter()
            .find(|r| r.member_id == member)
            .ok_or_else(invalid)
    }
}

pub(crate) fn validate_sound_fact(event: &Event) -> Result<(), Error> {
    let sound = matches!(
        event.fact,
        Fact::SoundTrialPrepared(_) | Fact::SoundReferencePlayed(_) | Fact::SoundTrialCompleted(_)
    ) || is_sound_focus_reference(event);
    if sound && event.version < 6 {
        return Err(Error::Version(event.version));
    }
    match &event.fact {
        Fact::SoundTrialPrepared(p) => p.validate(),
        Fact::SoundReferencePlayed(p) => {
            p.reference.validate()?;
            if p.token.is_nil() {
                return Err(invalid());
            }
            match &p.scope {
                SoundPlaybackScope::Example {
                    block_id,
                    member_id,
                } => {
                    if block_id.is_nil()
                        || p.token != *block_id
                        || *member_id != p.reference.member_id
                    {
                        return Err(invalid());
                    }
                }
                SoundPlaybackScope::Trial { prepared } => {
                    if prepared.is_nil() || *prepared == event.id {
                        return Err(invalid());
                    }
                }
            }
            if matches!(p.stage,SoundPlaybackStage::Completed {started_event} if started_event.is_nil()||started_event==event.id)
            {
                return Err(invalid());
            }
            Ok(())
        }
        Fact::SoundTrialCompleted(c) => {
            if c.prepared.is_nil()
                || c.attempt.is_nil()
                || c.context.is_nil()
                || [c.prepared, c.attempt, c.context].contains(&event.id)
                || c.known_playback_ids.len() > 256
                || c.support_presentation_ids.len() > 64
                || c.known_playback_ids
                    .iter()
                    .chain(&c.support_presentation_ids)
                    .any(|id| id.is_nil() || *id == event.id)
                || c.known_playback_ids.iter().collect::<BTreeSet<_>>().len()
                    != c.known_playback_ids.len()
                || c.support_presentation_ids
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != c.support_presentation_ids.len()
            {
                return Err(invalid());
            }
            match &c.response {
                SoundTrialResponse::Listening { chosen_member_id } => {
                    if !identity(chosen_member_id) {
                        return Err(invalid());
                    }
                }
                SoundTrialResponse::Speaking {
                    intended_member_id,
                    original_recording,
                    attempt_recording,
                    capture_scope,
                    asset_id,
                    asset,
                    ..
                } => {
                    asset.validate()?;
                    if !identity(intended_member_id)
                        || original_recording.is_nil()
                        || attempt_recording.is_nil()
                        || original_recording == attempt_recording
                        || [*original_recording, *attempt_recording].contains(&event.id)
                        || capture_scope.session_id.is_nil()
                        || capture_scope.intent_id.is_nil()
                        || !identity(&capture_scope.member_id)
                        || !bounded(asset_id, 256)
                        || asset_id.chars().any(char::is_control)
                    {
                        return Err(invalid());
                    }
                }
            }
            Ok(())
        }
        Fact::PassageRevised(r) if is_sound_focus_reference(event) => {
            let suffix = r
                .passage
                .source
                .strip_prefix(REFERENCE_PREFIX)
                .ok_or_else(invalid)?;
            let (id, version) = suffix.split_once('@').ok_or_else(invalid)?;
            if !identity(id) || !identity(version) {
                return Err(invalid());
            }
            let (passage, revision) = sound_reference_identity(id, version);
            if event.id != revision
                || event.device != Uuid::new_v5(&passage, b"sound-reference-writer")
                || event.recorded_at_ms != 0
                || r.passage.id != passage
                || r.passage.title != "Sound Focus reference"
                || r.passage.coach_reviewed
                || !r.parents.is_empty()
            {
                return Err(invalid());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn known_reference(
    reference: &SoundReferenceBinding,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    if let Some(e) = events.get(&reference.revision) {
        match &e.fact {
            Fact::PassageRevised(r)
                if is_sound_focus_reference(e) && r.passage.russian == reference.source_text =>
            {
                validate_sound_fact(e)?
            }
            _ => return Err(invalid()),
        }
    }
    if let Some(e) = events.get(&reference.recording) {
        match &e.fact {
            Fact::RecordingAttached(a)
                if a.target == AnnotationTarget::Revision(reference.revision)
                    && a.role == AudioRole::Reference
                    && a.asset_id == reference.asset_id
                    && a.asset == reference.asset =>
            {
                ()
            }
            _ => return Err(invalid()),
        }
    }
    Ok(())
}
fn prepared_in(events: &BTreeMap<Uuid, Event>, id: Uuid) -> Option<(&Event, &SoundTrialPrepared)> {
    match events.get(&id)? {
        e @ Event {
            fact: Fact::SoundTrialPrepared(_),
            ..
        } => match &e.fact {
            Fact::SoundTrialPrepared(p) => Some((e, p)),
            _ => None,
        },
        _ => None,
    }
}
fn bound_support(p: &StimulusPresentation, prepared: &SoundTrialPrepared) -> bool {
    p.session == Some(prepared.trial_id)
        && p.revision
            .is_some_and(|id| prepared.references.iter().any(|r| r.revision == id))
        && matches!(
            p.purpose,
            PresentationPurpose::Instruction
                | PresentationPurpose::Answer
                | PresentationPurpose::Reference
        )
}
fn valid_end(events: &BTreeMap<Uuid, Event>, event: &Event, p: &SoundReferencePlayed) -> bool {
    let SoundPlaybackStage::Completed { started_event } = p.stage else {
        return false;
    };
    let Some(start) = events.get(&started_event) else {
        return false;
    };
    matches!(&start.fact,Fact::SoundReferencePlayed(s) if matches!(s.stage,SoundPlaybackStage::Started)
        && s.scope==p.scope&&s.token==p.token&&s.reference==p.reference&&s.natural_speed==p.natural_speed&&start.id!=event.id)
}

pub(crate) fn validate_sound_references(
    event: &Event,
    events: &BTreeMap<Uuid, Event>,
) -> Result<(), Error> {
    match &event.fact {
        Fact::SoundTrialPrepared(p) => {
            for reference in &p.references {
                known_reference(reference, events)?;
                if let Some(actual) = events.get(&reference.revision) {
                    if actual != &sound_reference_event(&p.contrast, &reference.member_id)? {
                        return Err(invalid());
                    }
                }
            }
        }
        Fact::SoundReferencePlayed(p) => {
            known_reference(&p.reference, events)?;
            match &p.scope {
                SoundPlaybackScope::Trial { prepared } => {
                    if let Some(e) = events.get(prepared) {
                        let (_, trial) = prepared_in(events, *prepared).ok_or_else(invalid)?;
                        if p.token != trial.token
                            || trial.reference(&p.reference.member_id)? != &p.reference
                            || e.id == event.id
                        {
                            return Err(invalid());
                        }
                    }
                }
                SoundPlaybackScope::Example {
                    block_id,
                    member_id,
                } => {
                    for e in events.values() {
                        if let Fact::SoundTrialPrepared(trial) = &e.fact {
                            if trial.block_id == *block_id
                                && trial.reference(member_id)? != &p.reference
                            {
                                return Err(invalid());
                            }
                        }
                    }
                }
            }
            if let SoundPlaybackStage::Completed { started_event } = p.stage {
                if events.contains_key(&started_event) && !valid_end(events, event, p) {
                    return Err(invalid());
                }
            }
        }
        Fact::SoundTrialCompleted(c) => {
            let known_attempt = match events.get(&c.attempt) {
                Some(e) => match &e.fact {
                    Fact::AttemptRecorded(a) => Some((e, a)),
                    _ => return Err(invalid()),
                },
                None => None,
            };
            let known_context = match events.get(&c.context) {
                Some(e) => match &e.fact {
                    Fact::AttemptContextRecorded(ctx) if ctx.attempt == c.attempt => Some(ctx),
                    _ => return Err(invalid()),
                },
                None => None,
            };
            for id in &c.known_playback_ids {
                if events
                    .get(id)
                    .is_some_and(|e| !matches!(e.fact, Fact::SoundReferencePlayed(_)))
                {
                    return Err(invalid());
                }
            }
            for id in &c.support_presentation_ids {
                if events
                    .get(id)
                    .is_some_and(|e| !matches!(e.fact, Fact::StimulusPresented(_)))
                {
                    return Err(invalid());
                }
            }
            if let Some((_, a)) = known_attempt {
                let skill = match c.response {
                    SoundTrialResponse::Listening { .. } => Skill::Listening,
                    SoundTrialResponse::Speaking { .. } => Skill::Speaking,
                };
                if a.skill != skill
                    || a.self_rating != SelfRating::NotReported
                    || a.policy != POLICY
                {
                    return Err(invalid());
                }
            }
            if let SoundTrialResponse::Speaking {
                original_recording,
                attempt_recording,
                captured_at_ms,
                asset_id,
                asset,
                ..
            } = &c.response
            {
                for (id, original) in [(*original_recording, true), (*attempt_recording, false)] {
                    if let Some(record) = events.get(&id) {
                        match &record.fact {
                            Fact::RecordingAttached(a)
                                if a.role == AudioRole::Attempt
                                    && a.asset_id == *asset_id
                                    && a.asset == *asset
                                    && (if original {
                                        matches!(a.target, AnnotationTarget::Revision(_))
                                            && record.recorded_at_ms == *captured_at_ms
                                    } else {
                                        a.target == AnnotationTarget::Attempt(c.attempt)
                                    }) =>
                            {
                                ()
                            }
                            _ => return Err(invalid()),
                        }
                    }
                }
            }
            if let Some(e) = events.get(&c.prepared) {
                let (_, p) = prepared_in(events, e.id).ok_or_else(invalid)?;
                let target = p.reference(&p.target_member_id)?;
                let (skill, task, exercise, response) = match (&p.mode, &c.response) {
                    (
                        SoundTrialMode::Listening,
                        SoundTrialResponse::Listening { chosen_member_id },
                    ) => {
                        let member = p.contrast.member(chosen_member_id).ok_or_else(invalid)?;
                        if !p.option_member_ids.contains(chosen_member_id) {
                            return Err(invalid());
                        }
                        (
                            Skill::Listening,
                            TaskKind::SoundDiscrimination,
                            &p.contrast.identify_task,
                            member.option_label.clone(),
                        )
                    }
                    (
                        SoundTrialMode::Speaking,
                        SoundTrialResponse::Speaking {
                            intended_member_id,
                            original_recording,
                            attempt_recording,
                            capture_scope,
                            captured_at_ms,
                            asset_id,
                            asset,
                        },
                    ) => {
                        if intended_member_id != &p.target_member_id
                            || capture_scope.session_id != p.block_id
                            || capture_scope.intent_id != p.trial_id
                            || capture_scope.member_id != *intended_member_id
                            || *captured_at_ms < e.recorded_at_ms
                        {
                            return Err(invalid());
                        }
                        for (id, target_kind) in [
                            (
                                *original_recording,
                                AnnotationTarget::Revision(target.revision),
                            ),
                            (*attempt_recording, AnnotationTarget::Attempt(c.attempt)),
                        ] {
                            if let Some(record) = events.get(&id) {
                                match &record.fact {
                                    Fact::RecordingAttached(a)
                                        if a.target == target_kind
                                            && a.role == AudioRole::Attempt
                                            && a.asset_id == *asset_id
                                            && a.asset == *asset
                                            && (id != *original_recording
                                                || record.recorded_at_ms == *captured_at_ms) =>
                                    {
                                        ()
                                    }
                                    _ => return Err(invalid()),
                                }
                            }
                        }
                        (
                            Skill::Speaking,
                            TaskKind::Pronunciation,
                            &p.contrast.produce_task,
                            format!("Sound Focus recording · {asset_id}"),
                        )
                    }
                    _ => return Err(invalid()),
                };
                if let Some((_, a)) = known_attempt {
                    if a.session != p.trial_id
                        || a.revision != target.revision
                        || a.skill != skill
                        || a.self_rating != SelfRating::NotReported
                        || a.policy != POLICY
                        || a.response != response
                    {
                        return Err(invalid());
                    }
                }
                if let Some(ctx) = known_context {
                    if ctx.task != task
                        || ctx.exercise_id.as_deref()
                            != Some(format!("{}@{}", exercise.id, exercise.version).as_str())
                        || ctx.target_features
                            != [p.contrast.target_id.clone()].into_iter().collect()
                        || ctx.novel_context
                        || ctx.delayed_probe
                        || ctx.grammar_response.is_some()
                    {
                        return Err(invalid());
                    }
                }
                let cutoff = known_attempt.map(|(e, _)| e.recorded_at_ms);
                for id in &c.known_playback_ids {
                    if let Some(play) = events.get(id) {
                        match &play.fact {
                            Fact::SoundReferencePlayed(r)
                                if r.scope
                                    == (SoundPlaybackScope::Trial {
                                        prepared: c.prepared,
                                    })
                                    && r.token == p.token
                                    && cutoff.is_none_or(|t| play.recorded_at_ms <= t) =>
                            {
                                ()
                            }
                            _ => return Err(invalid()),
                        }
                    }
                }
                for id in &c.support_presentation_ids {
                    if let Some(presentation) = events.get(id) {
                        match &presentation.fact {
                            Fact::StimulusPresented(s)
                                if bound_support(s, p)
                                    && cutoff.is_none_or(|t| presentation.recorded_at_ms <= t) =>
                            {
                                ()
                            }
                            _ => return Err(invalid()),
                        }
                    }
                }
                // An absent playback/start remains an explicit incomplete graph.
                // Once all cited playback facts and starts are available, a
                // listening response must have heard its actual target to the end.
                if p.mode == SoundTrialMode::Listening {
                    let complete = c.known_playback_ids.iter().all(|id| {
                        events.get(id).is_some_and(|e| match &e.fact {
                            Fact::SoundReferencePlayed(r) => match r.stage {
                                SoundPlaybackStage::Started => true,
                                SoundPlaybackStage::Completed { started_event } => {
                                    events.contains_key(&started_event)
                                }
                            },
                            _ => true,
                        })
                    });
                    let has_end = c
                        .known_playback_ids
                        .iter()
                        .filter_map(|id| events.get(id))
                        .any(|e| {
                            matches!(&e.fact,Fact::SoundReferencePlayed(r)
                        if r.reference==*target&&r.natural_speed&&valid_end(events,e,r))
                        });
                    if complete && !has_end {
                        return Err(invalid());
                    }
                }
            }
        }
        _ => (),
    }
    Ok(())
}

fn same_references(a: &SoundTrialPrepared, b: &SoundTrialPrepared) -> bool {
    a.references
        .iter()
        .all(|r| b.references.iter().any(|other| other == r))
}
pub(crate) fn validate_sound_identities(events: &BTreeMap<Uuid, Event>) -> Result<(), Error> {
    let mut trials = BTreeMap::<Uuid, &SoundTrialPrepared>::new();
    let mut tokens = BTreeMap::<Uuid, Uuid>::new();
    let mut positions = BTreeMap::<(Uuid, usize), Uuid>::new();
    let mut blocks = BTreeMap::<Uuid, Vec<&SoundTrialPrepared>>::new();
    let mut asset_ids = BTreeMap::<&str, &AudioAsset>::new();
    let mut attempts_by_hash = BTreeMap::<&str, BTreeSet<Uuid>>::new();
    let mut speaking = Vec::new();
    let mut contrasts = BTreeMap::new();
    let mut members = BTreeMap::new();
    let mut utterances = BTreeMap::new();
    let mut tasks = BTreeMap::new();
    for e in events.values() {
        match &e.fact {
            Fact::SoundTrialPrepared(p) => {
                if contrasts
                    .insert((&p.contrast.id, &p.contrast.version), &p.contrast)
                    .is_some_and(|old| old != &p.contrast)
                {
                    return Err(invalid());
                }
                for member in &p.contrast.members {
                    if members
                        .insert((&member.id, &member.version), member)
                        .is_some_and(|old| old != member)
                        || utterances
                            .insert(
                                (&member.utterance.id, &member.utterance.version),
                                &member.utterance,
                            )
                            .is_some_and(|old| old != &member.utterance)
                    {
                        return Err(invalid());
                    }
                }
                for task in [&p.contrast.identify_task, &p.contrast.produce_task] {
                    if tasks
                        .insert((&task.id, &task.version), task)
                        .is_some_and(|old| old != task)
                    {
                        return Err(invalid());
                    }
                }
                if trials.insert(p.trial_id, p).is_some_and(|old| old != p)
                    || tokens
                        .insert(p.token, p.trial_id)
                        .is_some_and(|trial| trial != p.trial_id)
                    || (p.mode == SoundTrialMode::Listening
                        && positions
                            .insert((p.block_id, p.position), p.trial_id)
                            .is_some_and(|id| id != p.trial_id))
                {
                    return Err(invalid());
                }
                let block = blocks.entry(p.block_id).or_default();
                if block
                    .first()
                    .is_some_and(|first| first.contrast != p.contrast || !same_references(first, p))
                {
                    return Err(invalid());
                }
                if !block.iter().any(|old| old.trial_id == p.trial_id) {
                    block.push(p);
                }
                for r in &p.references {
                    if asset_ids
                        .insert(&r.asset_id, &r.asset)
                        .is_some_and(|old| old != &r.asset)
                    {
                        return Err(invalid());
                    }
                }
            }
            Fact::SoundReferencePlayed(p) => {
                if asset_ids
                    .insert(&p.reference.asset_id, &p.reference.asset)
                    .is_some_and(|old| old != &p.reference.asset)
                {
                    return Err(invalid());
                }
            }
            Fact::SoundTrialCompleted(c) => {
                if let SoundTrialResponse::Speaking {
                    asset_id, asset, ..
                } = &c.response
                {
                    if asset_ids
                        .insert(asset_id, asset)
                        .is_some_and(|old| old != asset)
                    {
                        return Err(invalid());
                    }
                    attempts_by_hash
                        .entry(&asset.sha256)
                        .or_default()
                        .insert(c.attempt);
                    speaking.push((&asset.sha256, c.attempt));
                }
            }
            Fact::RecordingAttached(a) => {
                if asset_ids
                    .insert(&a.asset_id, &a.asset)
                    .is_some_and(|old| old != &a.asset)
                {
                    return Err(invalid());
                }
                if let AnnotationTarget::Attempt(attempt) = a.target {
                    attempts_by_hash
                        .entry(&a.asset.sha256)
                        .or_default()
                        .insert(attempt);
                }
            }
            Fact::DialogueAudioAttached(a) => {
                if asset_ids
                    .insert(&a.asset_id, &a.asset)
                    .is_some_and(|old| old != &a.asset)
                {
                    return Err(invalid());
                }
            }
            _ => (),
        }
    }
    for (hash, attempt) in speaking {
        if attempts_by_hash
            .get(hash.as_str())
            .is_some_and(|ids| ids.iter().any(|id| *id != attempt))
        {
            return Err(invalid());
        }
    }
    for block in blocks.values() {
        let listening: Vec<_> = block
            .iter()
            .filter(|p| p.mode == SoundTrialMode::Listening)
            .collect();
        if listening.len() == 4 {
            let combinations: BTreeSet<_> = listening
                .iter()
                .map(|p| (&p.target_member_id, &p.option_member_ids))
                .collect();
            if combinations.len() != 4 {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

impl Library {
    pub fn sound_prepared(&self, trial: Uuid) -> Option<(&Event, &SoundTrialPrepared)> {
        self.events.values().find_map(|e| match &e.fact {
            Fact::SoundTrialPrepared(p) if p.trial_id == trial => Some((e, p)),
            _ => None,
        })
    }
    pub fn sound_trial_completion(&self, trial: Uuid) -> Vec<(&Event, &SoundTrialCompleted)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::SoundTrialCompleted(c)
                    if prepared_in(&self.events, c.prepared)
                        .is_some_and(|(_, p)| p.trial_id == trial) =>
                {
                    Some((e, c))
                }
                _ => None,
            })
            .collect()
    }
    pub fn sound_playbacks_for_trial(
        &self,
        prepared: Uuid,
    ) -> Vec<(&Event, &SoundReferencePlayed)> {
        self.events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::SoundReferencePlayed(p)
                    if p.scope == (SoundPlaybackScope::Trial { prepared }) =>
                {
                    Some((e, p))
                }
                _ => None,
            })
            .collect()
    }
    pub fn sound_trials(&self) -> Vec<SoundTrialSummary> {
        let mut trials = BTreeMap::new();
        for e in self.events.values() {
            if let Fact::SoundTrialPrepared(p) = &e.fact {
                trials.entry(p.trial_id).or_insert(p);
            }
        }
        trials.into_iter().map(|(trial_id,p)| {
            let completions=self.sound_trial_completion(trial_id);
            let conflict=completions.first().is_some_and(|(_,first)|completions.iter().any(|(_,c)|*c!=*first));
            let mut incomplete=completions.is_empty()||conflict||p.references.iter().any(|r| !self.events.contains_key(&r.revision)||!self.events.contains_key(&r.recording));
            let mut choice_member_id=None;let mut intended_match=None;let mut assisted=false;
            for (_,c) in &completions {
                let attempt=self.events.get(&c.attempt);let cutoff=attempt.map(|e|e.recorded_at_ms).unwrap_or(i64::MAX);
                incomplete|=attempt.is_none()||!self.events.contains_key(&c.context)
                    ||c.known_playback_ids.iter().chain(&c.support_presentation_ids).any(|id|!self.events.contains_key(id));
                if let Some(Event{fact:Fact::AttemptRecorded(a),..})=attempt {assisted|=!a.cues.is_empty();}
                assisted|=c.revealed||!c.support_presentation_ids.is_empty();
                match &c.response {
                    SoundTrialResponse::Listening {chosen_member_id} if !conflict=>{
                        choice_member_id=Some(chosen_member_id.clone());intended_match=Some(*chosen_member_id==p.target_member_id);
                    }
                    SoundTrialResponse::Speaking {original_recording,attempt_recording,..}=>{
                        incomplete|=!self.events.contains_key(original_recording)||!self.events.contains_key(attempt_recording);
                    }
                    _=>(),
                }
                let plays=self.sound_playbacks_for_trial(c.prepared);
                let starts=plays.iter().filter(|(e,r)|e.recorded_at_ms<=cutoff&&matches!(r.stage,SoundPlaybackStage::Started)).count();
                assisted|=match p.mode {SoundTrialMode::Listening=>starts>1,SoundTrialMode::Speaking=>starts>0};
                incomplete|=plays.iter().any(|(e,r)|e.recorded_at_ms<=cutoff&&matches!(r.stage,SoundPlaybackStage::Completed{started_event} if !self.events.contains_key(&started_event)));
                if p.mode==SoundTrialMode::Listening {
                    let target=p.references.iter().find(|r|r.member_id==p.target_member_id);
                    incomplete|=!plays.iter().any(|(e,r)|e.recorded_at_ms<=cutoff&&r.natural_speed&&Some(&r.reference)==target&&valid_end(&self.events,e,r));
                }
                for (e,s) in self.presentations() {if e.recorded_at_ms<=cutoff&&bound_support(s,p) {assisted=true;}}
            }
            SoundTrialSummary {trial_id,block_id:p.block_id,mode:p.mode,target_member_id:p.target_member_id.clone(),choice_member_id,intended_match,assisted,incomplete}
        }).collect()
    }
}
