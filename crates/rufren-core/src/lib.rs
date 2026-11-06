//! Portable learning facts and replication protocol. No UI, filesystem, or network.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

mod asset_transport;
pub use asset_transport::*;
mod ai_feedback;
pub use ai_feedback::*;
mod presentation;
mod speech_facts;
pub use presentation::*;
pub use speech_facts::*;
mod learner;
mod learning_facts;
mod review;
mod teaching;
pub use learner::*;
pub use learning_facts::*;
pub use review::*;
pub use teaching::*;
mod teacher_archive;
pub use teacher_archive::*;
mod teacher_revisit;
pub use teacher_revisit::*;
mod teacher_dialogue;
pub use teacher_dialogue::*;
mod dialogue_audio;
pub use dialogue_audio::*;

/// Current event encoding. Database migrations and transport compatibility are independent.
pub const EVENT_VERSION: u32 = 5;
pub const MIN_EVENT_VERSION: u32 = 1;
pub const SYNC_VERSION: u32 = 5;
/// Backward-compatible name for callers constructing events; never use for a DB schema.
pub const SCHEMA_VERSION: u32 = EVENT_VERSION;
pub const SYNC_BATCH: usize = 128;
pub const MAX_TEXT: usize = 32_000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unsupported data version {0}. Update Rufren before opening this data.")]
    Version(u32),
    #[error("An event ID was reused with different content: {0}")]
    Collision(Uuid),
    #[error("Invalid data: {0}")]
    Invalid(String),
    #[error("This is a different sync library. Reconnect explicitly before syncing.")]
    RelayChanged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Listening,
    Speaking,
    Reading,
    Writing,
}
impl Skill {
    pub const ALL: [Self; 4] = [
        Self::Listening,
        Self::Speaking,
        Self::Reading,
        Self::Writing,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Listening => "Listening",
            Self::Speaking => "Speaking",
            Self::Reading => "Reading",
            Self::Writing => "Writing",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub id: Uuid,
    pub title: String,
    pub russian: String,
    pub translation: String,
    pub source: String,
    pub coach_reviewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub passage: Passage,
    /// The edit replaces these revisions. Multiple parents explicitly resolve a conflict.
    pub parents: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cue {
    Translation,
    OriginalText,
    Replay,
    Coach,
    /// Reference-bearing screens were opened during retrieval; possible answer exposure.
    ReferenceMaterial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelfRating {
    Again,
    WithHelp,
    Recalled,
    /// The learner supplied a response without reporting a recall grade.
    NotReported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub session: Uuid,
    pub revision: Uuid,
    pub skill: Skill,
    pub response: String,
    pub cues: Vec<Cue>,
    /// A learner report, never an automatic correctness or proficiency judgment.
    pub self_rating: SelfRating,
    pub policy: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Encounter,
    Retrieve,
    Reflect,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub id: Uuid,
    pub revision: Uuid,
    pub skill: Skill,
    pub phase: Phase,
    pub draft: String,
    pub cues: Vec<Cue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Fact {
    PassageRevised(Revision),
    AttemptRecorded(Attempt),
    AttemptContextRecorded(AttemptContext),
    CoachJudgmentRecorded(CoachJudgment),
    CoachAnnotationRecorded(CoachAnnotation),
    TeachingDecisionRecorded(TeachingDecision),
    RecordingAttached(AudioAttachment),
    AiAssessmentRecorded(AiAssessment),
    StimulusPresented(StimulusPresentation),
    SpeechTranscribed(SpeechTranscription),
    TranscriptConfirmed(TranscriptConfirmation),
    TeacherDocumentRecorded(TeacherDocument),
    TeacherDialogueUserRecorded(rufren_tutor_contract::TeacherDialogueRequest),
    TeacherDialogueReplyRecorded(rufren_tutor_contract::TeacherDialogueResponse),
    DialogueAudioAttached(DialogueAudioAttachment),
    DialogueSpeechTranscribed(DialogueSpeechTranscription),
    DialogueTranscriptConfirmed(DialogueTranscriptConfirmation),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub version: u32,
    pub id: Uuid,
    pub device: Uuid,
    /// Informational only: clocks do not decide merge winners.
    pub recorded_at_ms: i64,
    pub fact: Fact,
}
impl Event {
    pub fn new(device: Uuid, recorded_at_ms: i64, fact: Fact) -> Result<Self, Error> {
        let event = Self {
            version: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            device,
            recorded_at_ms,
            fact,
        };
        event.validate()?;
        Ok(event)
    }
    pub fn validate(&self) -> Result<(), Error> {
        let required = match &self.fact {
            Fact::TeacherDialogueUserRecorded(_)
            | Fact::TeacherDialogueReplyRecorded(_)
            | Fact::DialogueAudioAttached(_)
            | Fact::DialogueSpeechTranscribed(_)
            | Fact::DialogueTranscriptConfirmed(_) => 5,
            Fact::TeacherDocumentRecorded(_) => 4,
            Fact::StimulusPresented(_)
            | Fact::SpeechTranscribed(_)
            | Fact::TranscriptConfirmed(_) => 3,
            Fact::AiAssessmentRecorded(a) if a.confirmed_transcript.is_some() => 3,
            Fact::AttemptRecorded(a) if a.self_rating == SelfRating::NotReported => 3,
            Fact::AttemptRecorded(a) if a.cues.contains(&Cue::ReferenceMaterial) => 2,
            Fact::PassageRevised(_) | Fact::AttemptRecorded(_) => 1,
            _ => 2,
        };
        if !(MIN_EVENT_VERSION..=EVENT_VERSION).contains(&self.version) || self.version < required {
            return Err(Error::Version(self.version));
        }
        if self.id.is_nil() || self.device.is_nil() {
            return Err(Error::Invalid("missing identity".into()));
        }
        let bounded = |s: &str| s.len() <= MAX_TEXT;
        match &self.fact {
            Fact::PassageRevised(r) => {
                let p = &r.passage;
                if p.id.is_nil()
                    || p.title.trim().is_empty()
                    || p.russian.trim().is_empty()
                    || ![&p.title, &p.russian, &p.translation, &p.source]
                        .iter()
                        .all(|s| bounded(s))
                    || r.parents.contains(&self.id)
                    || r.parents.iter().any(Uuid::is_nil)
                    || r.parents.len() > 64
                {
                    return Err(Error::Invalid("invalid passage or revision parents".into()));
                }
            }
            Fact::AttemptRecorded(a) => {
                if a.session.is_nil()
                    || a.revision.is_nil()
                    || !bounded(&a.response)
                    || a.policy.len() > 128
                    || a.cues.len() > 32
                {
                    return Err(Error::Invalid("invalid attempt".into()));
                }
            }
            Fact::TeacherDocumentRecorded(document) => {
                document.validate()?;
                if document
                    .context_source_fact_ids
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&self.id))
                {
                    return Err(Error::Invalid(
                        "teacher document cites itself as prepared context".into(),
                    ));
                }
            }
            Fact::TeacherDialogueUserRecorded(_) | Fact::TeacherDialogueReplyRecorded(_) => {
                validate_dialogue_fact(self)?
            }
            Fact::DialogueAudioAttached(_)
            | Fact::DialogueSpeechTranscribed(_)
            | Fact::DialogueTranscriptConfirmed(_) => validate_dialogue_audio_fact(self)?,
            _ => validate_learning_fact(self)?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub events: BTreeMap<Uuid, Event>,
}
impl Library {
    /// Validate a batch before making any change. Union is order-independent and idempotent.
    pub fn merge(&mut self, incoming: &[Event]) -> Result<(), Error> {
        let mut merged = self.events.clone();
        for event in incoming {
            event.validate()?;
            if let Some(old) = merged.get(&event.id) {
                if old != event {
                    return Err(Error::Collision(event.id));
                }
            } else {
                merged.insert(event.id, event.clone());
            }
        }
        // Parents may arrive on a later page. Validate cross-reference types when available.
        validate_audio_identities(&merged)?;
        validate_dialogue_audio_identities(&merged)?;
        validate_teacher_document_identities(&merged)?;
        validate_dialogue_graph(&merged)?;
        for event in merged.values() {
            validate_dialogue_audio_references(event, &merged)?;
            validate_learning_references(event, &merged)?;
            if let Fact::PassageRevised(r) = &event.fact {
                for parent in &r.parents {
                    if let Some(p) = merged.get(parent) {
                        match &p.fact {
                            Fact::PassageRevised(pr) if pr.passage.id == r.passage.id => {}
                            _ => {
                                return Err(Error::Invalid(
                                    "revision parent belongs to another item".into(),
                                ));
                            }
                        }
                    }
                }
            }
            if let Fact::AttemptRecorded(a) = &event.fact {
                if let Some(referenced) = merged.get(&a.revision) {
                    if !matches!(referenced.fact, Fact::PassageRevised(_)) {
                        return Err(Error::Invalid(
                            "attempt does not reference a passage revision".into(),
                        ));
                    }
                }
            }
        }
        // An explicit stack also handles long imported histories without overflowing Rust's stack.
        let mut done = BTreeSet::new();
        for root in merged.keys() {
            let mut active = BTreeSet::new();
            let mut stack = vec![(*root, false)];
            while let Some((id, leaving)) = stack.pop() {
                if leaving {
                    active.remove(&id);
                    done.insert(id);
                    continue;
                }
                if done.contains(&id) {
                    continue;
                }
                if !active.insert(id) {
                    return Err(Error::Invalid("cyclic revision history".into()));
                }
                stack.push((id, true));
                if let Some(Event {
                    fact: Fact::PassageRevised(r),
                    ..
                }) = merged.get(&id)
                {
                    stack.extend(r.parents.iter().map(|p| (*p, false)));
                }
                if let Some(event) = merged.get(&id) {
                    stack.extend(learning_parents(&event.fact).iter().map(|p| (*p, false)));
                }
            }
        }
        self.events = merged;
        Ok(())
    }
    pub fn revision(&self, id: Uuid) -> Option<&Revision> {
        match &self.events.get(&id)?.fact {
            Fact::PassageRevised(r) => Some(r),
            _ => None,
        }
    }
    pub fn heads(&self) -> BTreeMap<Uuid, Vec<Uuid>> {
        let superseded: BTreeSet<_> = self
            .events
            .values()
            .filter_map(|e| match &e.fact {
                Fact::PassageRevised(r) => Some(r.parents.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        let mut heads = BTreeMap::<Uuid, Vec<Uuid>>::new();
        for e in self.events.values() {
            if let Fact::PassageRevised(r) = &e.fact {
                if !superseded.contains(&e.id) {
                    heads.entry(r.passage.id).or_default().push(e.id);
                }
            }
        }
        heads
    }
    pub fn attempts(&self) -> impl Iterator<Item = (&Event, &Attempt)> {
        self.events.values().filter_map(|e| match &e.fact {
            Fact::AttemptRecorded(a) => Some((e, a)),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncBinding {
    pub endpoint: String,
    pub relay: Uuid,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncRequest {
    pub version: u32,
    pub relay: Option<Uuid>,
    pub after: u64,
    pub events: Vec<Event>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncResponse {
    pub version: u32,
    pub relay: Uuid,
    pub accepted: Vec<Uuid>,
    pub events: Vec<Event>,
    pub cursor: u64,
    pub has_more: bool,
}

/// Audio remains a separate asset pipeline; no sample bytes or API secrets in sync facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AudioAsset {
    /// SHA-256 of the original encoded audio blob, not of decoded/resampled PCM.
    pub sha256: String,
    pub media_type: String,
    /// One decoding observation. Device audio contexts can resample identical blobs differently.
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub frames: u64,
}

pub trait TeachingPolicy {
    fn id(&self) -> &str;
    fn choose(&self, library: &Library) -> Option<Uuid>;
}
/// A baseline selection policy, not a scientifically optimized scheduler.
pub struct LeastPracticed;
impl TeachingPolicy for LeastPracticed {
    fn id(&self) -> &str {
        "least-practiced-v1"
    }
    fn choose(&self, library: &Library) -> Option<Uuid> {
        library
            .heads()
            .values()
            .filter(|v| v.len() == 1)
            .map(|v| v[0])
            .min_by_key(|id| {
                (
                    library
                        .attempts()
                        .filter(|(_, a)| a.revision == *id)
                        .count(),
                    *id,
                )
            })
    }
}

pub fn starter(device: Uuid, now: i64) -> Event {
    Event::new(
        device,
        now,
        Fact::PassageRevised(Revision {
            passage: Passage {
                id: Uuid::new_v4(),
                title: "A little closer to the stars".into(),
                russian: "Это корабль. Он летит к звезде.".into(),
                translation: "This is a spaceship. It is flying toward a star.".into(),
                source: "Original practice text · awaiting coach review".into(),
                coach_reviewed: false,
            },
            parents: BTreeSet::new(),
        }),
    )
    .expect("valid built-in passage")
}
