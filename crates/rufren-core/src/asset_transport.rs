//! Audio transport metadata. Original bytes are content-addressed; takes retain separate IDs.
use crate::Error;
use serde::{Deserialize, Serialize};

pub const ASSET_TRANSPORT_VERSION: u32 = 1;
pub const MAX_AUDIO_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_AUDIO_RECORD_BYTES: usize = 96 * 1024;
pub const ASSET_PAGE_BYTES: usize = 256 * 1024;
pub const ASSET_PAGE_SIZE: usize = 64;

pub fn valid_audio_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedAudioCue {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub note: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioRecordMetadata {
    pub asset_id: String,
    pub content_hash: String,
    pub role: String,
    pub name: String,
    pub passage_id: String,
    pub revision_id: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    #[serde(default)]
    pub frames: u64,
    pub byte_length: u64,
    pub created_at_ms: i64,
    pub mime_type: String,
    #[serde(default)]
    pub interrupted: bool,
    #[serde(default)]
    pub device: serde_json::Value,
    #[serde(default)]
    pub annotations: Vec<TimedAudioCue>,
}
impl AudioRecordMetadata {
    pub fn validate(&self) -> Result<(), Error> {
        let bounded =
            |value: &str, limit| value.len() <= limit && !value.chars().any(char::is_control);
        let reference = |value: &str| {
            value.is_empty() || uuid::Uuid::parse_str(value).is_ok_and(|id| !id.is_nil())
        };
        if self.asset_id.is_empty()
            || !bounded(&self.asset_id, 256)
            || !self
                .asset_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            || !valid_audio_hash(&self.content_hash)
            || !matches!(self.role.as_str(), "reference" | "attempt")
            || !bounded(&self.name, 1024)
            || !reference(&self.passage_id)
            || !reference(&self.revision_id)
            || !(1..=60_100).contains(&self.duration_ms)
            || !(8_000..=384_000).contains(&self.sample_rate)
            || !(1..=8).contains(&self.channels)
            || self.frames > u64::from(self.sample_rate) * 61
            || self.byte_length == 0
            || self.byte_length > MAX_AUDIO_BYTES as u64
            || self.mime_type.is_empty()
            || !bounded(&self.mime_type, 128)
            || !self
                .mime_type
                .bytes()
                .all(|b| b.is_ascii_graphic() || b == b' ')
            || self.annotations.len() > 256
        {
            return Err(Error::Invalid("invalid audio recording metadata".into()));
        }
        for cue in &self.annotations {
            if !cue.start_seconds.is_finite()
                || !cue.end_seconds.is_finite()
                || cue.start_seconds < 0.0
                || cue.end_seconds <= cue.start_seconds
                || cue.end_seconds > self.duration_ms as f64 / 1000.0 + 0.01
                || cue.note.trim().is_empty()
                || cue.note.len() > 2048
            {
                return Err(Error::Invalid("invalid timed audio cue".into()));
            }
        }
        if serde_json::to_vec(self).map_or(true, |bytes| bytes.len() > MAX_AUDIO_RECORD_BYTES) {
            return Err(Error::Invalid(
                "audio metadata exceeds transport budget".into(),
            ));
        }
        Ok(())
    }
    /// Append-only cue union. Different takes with identical bytes never collapse.
    pub fn merge(&self, other: &Self) -> Result<Self, Error> {
        self.validate()?;
        other.validate()?;
        let mut left = self.clone();
        let mut right = other.clone();
        left.annotations.clear();
        right.annotations.clear();
        // Device provenance can gain optional fields during migration; existing observations
        // cannot be changed. Null is the legacy representation of absent metadata.
        let device = match (&left.device, &right.device) {
            (a, b) if a == b => a.clone(),
            (serde_json::Value::Null, value) | (value, serde_json::Value::Null)
                if value.is_object() =>
            {
                value.clone()
            }
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                let mut merged = a.clone();
                for (key, value) in b {
                    if merged.get(key).is_some_and(|existing| existing != value) {
                        return Err(Error::Invalid(
                            "audio device provenance has conflicting observations".into(),
                        ));
                    }
                    merged.insert(key.clone(), value.clone());
                }
                serde_json::Value::Object(merged)
            }
            _ => {
                return Err(Error::Invalid(
                    "audio device provenance has conflicting observations".into(),
                ));
            }
        };
        left.device = device.clone();
        right.device = device;
        // Legacy browser records omitted decoded frame counts. Upgrade the missing value only.
        if left.frames == 0 {
            left.frames = right.frames;
        }
        if right.frames == 0 {
            right.frames = left.frames;
        }
        if left != right {
            return Err(Error::Invalid(
                "audio recording ID has conflicting immutable metadata".into(),
            ));
        }
        left.annotations = self.annotations.clone();
        for cue in &other.annotations {
            if !left.annotations.contains(cue) {
                left.annotations.push(cue.clone());
            }
        }
        left.annotations.sort_by(|a, b| {
            a.start_seconds
                .total_cmp(&b.start_seconds)
                .then_with(|| a.end_seconds.total_cmp(&b.end_seconds))
                .then(a.created_at_ms.cmp(&b.created_at_ms))
                .then(a.note.cmp(&b.note))
        });
        left.validate()?;
        Ok(left)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetPage {
    pub version: u32,
    pub records: Vec<AudioRecordMetadata>,
    pub next_after: Option<String>,
}
