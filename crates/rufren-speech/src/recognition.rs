//! Shared independent-transcription contract. A transcript is an observation
//! hypothesis; confirmation/editing and language feedback are separate facts.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const RECOGNITION_SCHEMA: &str = "rufren-speech-transcript-v1";
pub const RECOGNITION_MODEL: &str = "mlx-community/Qwen3-ASR-1.7B-bf16";
pub const RECOGNITION_REVISION: &str = "e1f6c266914abc5a46e8756e02580f834a6cf8a7";
pub const RECOGNITION_RUNTIME: &str = "mlx-audio==0.6.4";
pub const RECOGNITION_MAX_AUDIO_BYTES: usize = 9_000_000;
pub const RECOGNITION_MAX_REQUEST_BYTES: usize = 12_100_000;
pub const RECOGNITION_MAX_RESPONSE_BYTES: usize = 65_536;
pub const RECOGNITION_MAX_TRANSCRIPT_BYTES: usize = 16_384;
pub const RECOGNITION_MAX_DURATION_SECONDS: u64 = 45;
pub const RECOGNITION_DEADLINE_SECONDS: u64 = 120;
const SOURCE_PCM_FORMAT: &str = "interleaved-little-endian-f32-no-header-v1";
const DERIVATIVE_PCM_FORMAT: &str = "little-endian-f32-no-header-v1";
const DERIVATIVE_METHOD: &str =
    "arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5";

#[derive(Debug, thiserror::Error)]
pub enum RecognitionError {
    #[error("Local speech recognition is not configured")]
    Disabled,
    #[error("Invalid local speech recognition configuration or offline installation")]
    Configuration,
    #[error("Choose a valid recording, exact source hash and original-frame crop")]
    InvalidRequest,
    #[error(
        "Recognition supports exact PCM16/float32 WAV, mono/stereo, 8–48 kHz, at most 45 seconds and 9 MB"
    )]
    UnsupportedAudio,
    #[error("The recording bytes do not match the supplied source hash")]
    HashMismatch,
    #[error("This relay process has used its transcription request allowance")]
    BudgetExceeded,
    #[error("Local speech recognition is busy; try again shortly")]
    Busy,
    #[error("Transcription timed out; no automatic retry was made")]
    Timeout,
    #[error("Transcription was cancelled")]
    Cancelled,
    #[error("Local speech recognition is unavailable")]
    Unavailable,
    #[error("Recognition returned an invalid or incorrectly bound hypothesis")]
    InvalidResponse,
    #[error("Recognition exceeded its bounded request or response size")]
    ResponseTooLarge,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionSpan {
    pub start_frame: u64,
    pub end_frame: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionRequest {
    pub request_id: String,
    pub asset_id: String,
    #[serde(default)]
    pub attempt_id: Option<String>,
    #[serde(default)]
    pub revision_id: Option<String>,
    pub language: String,
    /// SHA-256 of exact original WAV bytes, not a decoded PCM identity.
    pub audio_sha256: String,
    pub wav_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<RecognitionSpan>,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
pub(crate) fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn bounded(s: &str, n: usize) -> bool {
    !s.is_empty() && s.len() <= n
}

impl RecognitionRequest {
    /// Format, hash and actual frame containment are also checked before the
    /// server starts a process. There is intentionally no lesson/hotwords field.
    pub fn validate(&self) -> Result<(), RecognitionError> {
        if !valid_id(&self.request_id)
            || !valid_id(&self.asset_id)
            || self.attempt_id.as_deref().is_some_and(|s| !valid_id(s))
            || self.revision_id.as_deref().is_some_and(|s| !valid_id(s))
            || self.language != "ru"
            || !valid_sha(&self.audio_sha256)
            || self.wav_base64.is_empty()
            || self.wav_base64.len() > RECOGNITION_MAX_AUDIO_BYTES.div_ceil(3) * 4
            || self
                .span
                .as_ref()
                .is_some_and(|span| span.start_frame >= span.end_frame)
        {
            return Err(RecognitionError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionOriginalAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub encoding: String,
    pub bits: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionDerivative {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub pcm_sha256: String,
    pub pcm_hash_format: String,
    pub method: String,
    pub scipy_version: String,
    pub amplitude_normalization: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionDecoding {
    pub temperature: f64,
    pub max_tokens: u32,
    pub seed: u64,
    pub language: String,
    pub hotwords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionProvenance {
    pub provider: String,
    pub model: String,
    pub revision: String,
    pub runtime: String,
    pub mlx_version: String,
    pub manifest_sha256: String,
    pub worker_sha256: String,
    pub decoding: RecognitionDecoding,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionTiming {
    pub started_at: String,
    pub completed_at: String,
    pub elapsed_seconds: f64,
    pub process_peak_rss_bytes: u64,
    pub metal_peak_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionUsage {
    pub prompt_tokens: u64,
    pub generation_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionResponse {
    pub schema_version: String,
    pub request_id: String,
    pub attempt_id: Option<String>,
    pub revision_id: Option<String>,
    pub asset_id: String,
    pub audio_sha256: String,
    pub original_audio: RecognitionOriginalAudio,
    /// Worker-specific decoded PCM hash; deliberately not rufren-audio's hash.
    pub source_pcm_sha256: String,
    pub source_pcm_hash_format: String,
    pub span: RecognitionSpan,
    pub span_coordinate_system: String,
    pub derivative: RecognitionDerivative,
    pub evidence_kind: String,
    /// Preserve verbatim as an observed ASR hypothesis, including empty output.
    pub transcript: String,
    pub language: String,
    pub language_selection: String,
    pub target_conditioned: bool,
    pub word_timestamps: Option<serde_json::Value>,
    pub word_alignment_status: String,
    pub confidence: Option<serde_json::Value>,
    pub acoustic_assessment: Option<serde_json::Value>,
    pub provenance: RecognitionProvenance,
    pub timing: RecognitionTiming,
    pub usage: RecognitionUsage,
}

impl RecognitionResponse {
    pub fn validate_for(&self, request: &RecognitionRequest) -> Result<(), RecognitionError> {
        request.validate()?;
        let original = &self.original_audio;
        let expected_span = request.span.clone().unwrap_or(RecognitionSpan {
            start_frame: 0,
            end_frame: original.frames,
        });
        let shape_valid = (8_000..=48_000).contains(&original.sample_rate)
            && (1..=2).contains(&original.channels)
            && original.frames > 0
            && original.frames
                <= u64::from(original.sample_rate) * RECOGNITION_MAX_DURATION_SECONDS
            && matches!(
                (original.encoding.as_str(), original.bits),
                ("pcm_s16le", 16) | ("ieee_f32le", 32)
            );
        if !shape_valid
            || self.span.start_frame >= self.span.end_frame
            || self.span.end_frame > original.frames
        {
            return Err(RecognitionError::InvalidResponse);
        }
        let expected_frames = ((self.span.end_frame - self.span.start_frame) * 16_000)
            .div_ceil(u64::from(original.sample_rate));
        let p = &self.provenance;
        let t = &self.timing;
        if self.schema_version != RECOGNITION_SCHEMA
            || self.request_id != request.request_id
            || self.asset_id != request.asset_id
            || self.attempt_id != request.attempt_id
            || self.revision_id != request.revision_id
            || self.audio_sha256 != request.audio_sha256
            || self.span != expected_span
            || self.span_coordinate_system != "original_frames_half_open"
            || !valid_sha(&self.source_pcm_sha256)
            || self.source_pcm_hash_format != SOURCE_PCM_FORMAT
            || self.derivative.sample_rate != 16_000
            || self.derivative.channels != 1
            || self.derivative.frames != expected_frames
            || !valid_sha(&self.derivative.pcm_sha256)
            || self.derivative.pcm_hash_format != DERIVATIVE_PCM_FORMAT
            || self.derivative.method != DERIVATIVE_METHOD
            || !bounded(&self.derivative.scipy_version, 64)
            || self.derivative.amplitude_normalization
            || self.evidence_kind != "transcript_hypothesis"
            || self.transcript.len() > RECOGNITION_MAX_TRANSCRIPT_BYTES
            || self.language != "ru"
            || self.language_selection != "explicit_not_detected"
            || self.target_conditioned
            || self.word_timestamps.is_some()
            || self.word_alignment_status != "not_run"
            || self.confidence.is_some()
            || self.acoustic_assessment.is_some()
            || p.provider != "local_qwen3_asr"
            || p.model != RECOGNITION_MODEL
            || p.revision != RECOGNITION_REVISION
            || p.runtime != RECOGNITION_RUNTIME
            || !bounded(&p.mlx_version, 64)
            || !valid_sha(&p.manifest_sha256)
            || !valid_sha(&p.worker_sha256)
            || p.decoding.temperature != 0.0
            || p.decoding.max_tokens != 1024
            || p.decoding.seed != 42
            || p.decoding.language != "Russian"
            || !p.decoding.hotwords.is_empty()
            || !bounded(&t.started_at, 64)
            || !bounded(&t.completed_at, 64)
            || !t.elapsed_seconds.is_finite()
            || !(0.0..=120.0).contains(&t.elapsed_seconds)
            || t.process_peak_rss_bytes > 16 * 1024_u64.pow(3)
            || t.metal_peak_bytes > 12 * 1024_u64.pow(3)
            || self.usage.generation_tokens >= 1024
            || self.usage.prompt_tokens > 1_000_000
        {
            return Err(RecognitionError::InvalidResponse);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecognitionStatus {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub revision: Option<String>,
    pub message: String,
    pub data_sent: String,
    pub max_audio_bytes: usize,
    pub max_duration_seconds: u64,
    pub remaining_requests: u64,
}

/// Exact source facts computed locally from the worker-supported WAV subset.
/// The PCM hash is interleaved little-endian f32 without metadata; it is not
/// the WAV-byte hash or rufren-audio's canonical identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecognitionOriginalEvidence {
    pub original_audio: RecognitionOriginalAudio,
    pub source_pcm_sha256: String,
}

/// Inspect exact original bytes without resampling, downmixing or normalizing
/// samples. Supports PCM16/IEEE float32 mono/stereo WAV at 8–48 kHz, at most
/// 45 seconds and 9 MB. Float values must be finite and in [-1,1]; raw -0.0 bits
/// remain significant in the worker-format PCM hash. Silent valid files can be
/// inspected; recognition separately rejects a crop with no usable signal.
pub fn inspect_recognition_wav(
    bytes: &[u8],
) -> Result<RecognitionOriginalEvidence, RecognitionError> {
    inspect_recognition_wav_crop(bytes, None).map(|(evidence, _)| evidence)
}

/// Same single parser/decoder/hash pass, with original-frame crop RMS for the
/// server's signal-readiness check. The source hash always covers the WHOLE file.
pub(crate) fn inspect_recognition_wav_crop(
    bytes: &[u8],
    selected_span: Option<&RecognitionSpan>,
) -> Result<(RecognitionOriginalEvidence, f64), RecognitionError> {
    let (shape, payload) = recognition_wav_shape(bytes)?;
    let span = selected_span.cloned().unwrap_or(RecognitionSpan {
        start_frame: 0,
        end_frame: shape.frames,
    });
    if span.start_frame >= span.end_frame || span.end_frame > shape.frames {
        return Err(RecognitionError::InvalidRequest);
    }
    let width = usize::from(shape.bits / 8);
    let frame_size = width * usize::from(shape.channels);
    let mut hash = Sha256::new();
    let mut encoded = Vec::with_capacity(65_536);
    let mut sum_squares = 0.0_f64;
    for (frame_index, frame) in payload.chunks_exact(frame_size).enumerate() {
        let mut mean = 0.0_f32;
        for sample in frame.chunks_exact(width) {
            let value = if width == 2 {
                i16::from_le_bytes(sample.try_into().unwrap()) as f32 / 32768.0
            } else {
                f32::from_le_bytes(sample.try_into().unwrap())
            };
            if !value.is_finite() || value.abs() > 1.0 {
                return Err(RecognitionError::UnsupportedAudio);
            }
            encoded.extend_from_slice(&value.to_le_bytes());
            mean += value / f32::from(shape.channels);
        }
        if encoded.len() >= 65_536 {
            hash.update(&encoded);
            encoded.clear();
        }
        if (span.start_frame..span.end_frame).contains(&(frame_index as u64)) {
            sum_squares += f64::from(mean).powi(2);
        }
    }
    hash.update(&encoded);
    let crop_rms = (sum_squares / (span.end_frame - span.start_frame) as f64).sqrt();
    Ok((
        RecognitionOriginalEvidence {
            original_audio: shape,
            source_pcm_sha256: format!("{:x}", hash.finalize()),
        },
        crop_rms,
    ))
}

fn recognition_wav_shape(
    bytes: &[u8],
) -> Result<(RecognitionOriginalAudio, &[u8]), RecognitionError> {
    let invalid = || RecognitionError::UnsupportedAudio;
    if bytes.len() < 44
        || bytes.len() > RECOGNITION_MAX_AUDIO_BYTES
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"WAVE"
    {
        return Err(invalid());
    }
    let u16le = |x: &[u8]| u16::from_le_bytes(x.try_into().unwrap());
    let u32le = |x: &[u8]| u32::from_le_bytes(x.try_into().unwrap());
    if u32le(&bytes[4..8]) as u64 + 8 != bytes.len() as u64 {
        return Err(invalid());
    }
    let mut fmt = None;
    let mut payload = None;
    let mut at = 12;
    while at < bytes.len() {
        if bytes.len() - at < 8 {
            return Err(invalid());
        }
        let tag = &bytes[at..at + 4];
        let size = u32le(&bytes[at + 4..at + 8]) as usize;
        at += 8;
        let end = at
            .checked_add(size)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(invalid)?;
        match tag {
            b"fmt " => {
                if fmt.is_some() || size < 16 {
                    return Err(invalid());
                }
                fmt = Some(&bytes[at..end]);
            }
            b"data" => {
                if payload.is_some() {
                    return Err(invalid());
                }
                payload = Some(&bytes[at..end]);
            }
            _ => (),
        }
        at = end
            .checked_add(size % 2)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(invalid)?;
    }
    let f = fmt.ok_or_else(invalid)?;
    let payload = payload.ok_or_else(invalid)?;
    let encoding = u16le(&f[..2]);
    let channels = u16le(&f[2..4]);
    let sample_rate = u32le(&f[4..8]);
    let byte_rate = u32le(&f[8..12]);
    let block_align = u16le(&f[12..14]);
    let bits = u16le(&f[14..16]);
    if !matches!((encoding, bits), (1, 16) | (3, 32))
        || !(1..=2).contains(&channels)
        || !(8_000..=48_000).contains(&sample_rate)
    {
        return Err(invalid());
    }
    if block_align != channels * (bits / 8)
        || byte_rate != sample_rate * u32::from(block_align)
        || payload.len() % usize::from(block_align) != 0
    {
        return Err(invalid());
    }
    let frames = (payload.len() / usize::from(block_align)) as u64;
    if frames == 0 || frames > u64::from(sample_rate) * RECOGNITION_MAX_DURATION_SECONDS {
        return Err(invalid());
    }
    Ok((
        RecognitionOriginalAudio {
            sample_rate,
            channels,
            frames,
            bits,
            encoding: if encoding == 1 {
                "pcm_s16le"
            } else {
                "ieee_f32le"
            }
            .into(),
        },
        payload,
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn wav(encoding: u16, bits: u16, rate: u32, channels: u16, payload: &[u8]) -> Vec<u8> {
        let align = channels * (bits / 8);
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(36 + payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&encoding.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * u32::from(align)).to_le_bytes());
        bytes.extend_from_slice(&align.to_le_bytes());
        bytes.extend_from_slice(&bits.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn recognition_wav_shared_inspection_matches_worker_pcm16_and_raw_float_negative_zero() {
        let pcm16 = [-32768_i16, 0, 32767]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        let pcm = inspect_recognition_wav(&wav(1, 16, 8000, 1, &pcm16)).unwrap();
        let expected = [-1.0_f32, 0.0, 32767.0 / 32768.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(pcm.original_audio.encoding, "pcm_s16le");
        assert_eq!(pcm.original_audio.bits, 16);
        assert_eq!(pcm.original_audio.frames, 3);
        assert_eq!(
            pcm.source_pcm_sha256,
            format!("{:x}", Sha256::digest(&expected))
        );
        let raw = [0.25_f32, -0.0, 1.0, -1.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        let negative = inspect_recognition_wav(&wav(3, 32, 8000, 2, &raw)).unwrap();
        assert_eq!(negative.original_audio.encoding, "ieee_f32le");
        assert_eq!(negative.original_audio.bits, 32);
        assert_eq!(negative.original_audio.channels, 2);
        assert_eq!(negative.original_audio.frames, 2);
        assert_eq!(
            negative.source_pcm_sha256,
            format!("{:x}", Sha256::digest(&raw))
        );
        let mut positive = raw.clone();
        positive[4..8].copy_from_slice(&0.0_f32.to_le_bytes());
        let positive = inspect_recognition_wav(&wav(3, 32, 8000, 2, &positive)).unwrap();
        assert_eq!(negative.original_audio, positive.original_audio);
        assert_ne!(negative.source_pcm_sha256, positive.source_pcm_sha256);
        let other_rate = inspect_recognition_wav(&wav(3, 32, 16000, 2, &raw)).unwrap();
        assert_ne!(negative.original_audio, other_rate.original_audio);
        assert_eq!(
            negative.source_pcm_sha256, other_rate.source_pcm_sha256,
            "worker PCM identity has no metadata header; compare shape as well"
        );
    }

    #[test]
    fn recognition_wav_crop_readiness_keeps_whole_original_identity() {
        let raw = [0.0_f32, -0.0, 0.25, -0.25]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        let bytes = wav(3, 32, 8000, 1, &raw);
        let whole = inspect_recognition_wav(&bytes).unwrap();
        let (quiet, rms) = inspect_recognition_wav_crop(
            &bytes,
            Some(&RecognitionSpan {
                start_frame: 0,
                end_frame: 2,
            }),
        )
        .unwrap();
        assert_eq!(quiet, whole);
        assert_eq!(rms, 0.0);
        let (signal, rms) = inspect_recognition_wav_crop(
            &bytes,
            Some(&RecognitionSpan {
                start_frame: 2,
                end_frame: 4,
            }),
        )
        .unwrap();
        assert_eq!(signal, whole);
        assert_eq!(rms, 0.25);
        assert!(
            inspect_recognition_wav_crop(
                &bytes,
                Some(&RecognitionSpan {
                    start_frame: 0,
                    end_frame: u64::MAX
                })
            )
            .is_err()
        );
    }

    #[test]
    fn recognition_wav_shared_inspection_rejects_unsupported_truncated_and_nonfinite_inputs() {
        let raw = 0.25_f32.to_le_bytes();
        let valid = wav(3, 32, 8000, 1, &raw);
        for len in 0..valid.len() {
            assert!(
                inspect_recognition_wav(&valid[..len]).is_err(),
                "prefix {len}"
            );
        }
        for bad in [f32::NAN, f32::INFINITY, -1.0001, 1.0001] {
            assert!(inspect_recognition_wav(&wav(3, 32, 8000, 1, &bad.to_le_bytes())).is_err());
        }
        assert!(inspect_recognition_wav(&wav(1, 24, 8000, 1, &[0; 6])).is_err());
        assert!(inspect_recognition_wav(&wav(3, 32, 96000, 1, &raw)).is_err());
        assert!(inspect_recognition_wav(&wav(3, 32, 8000, 3, &[0; 12])).is_err());
        let mut duplicate = valid.clone();
        duplicate.extend_from_slice(b"data");
        duplicate.extend_from_slice(&4_u32.to_le_bytes());
        duplicate.extend_from_slice(&raw);
        let length = (duplicate.len() - 8) as u32;
        duplicate[4..8].copy_from_slice(&length.to_le_bytes());
        assert!(inspect_recognition_wav(&duplicate).is_err());
        assert!(inspect_recognition_wav(&vec![0; RECOGNITION_MAX_AUDIO_BYTES + 1]).is_err());
    }

    pub(crate) fn sample_request() -> RecognitionRequest {
        RecognitionRequest {
            request_id: "recognition-1".into(),
            asset_id: "asset-1".into(),
            attempt_id: Some("attempt-1".into()),
            revision_id: Some("revision-1".into()),
            language: "ru".into(),
            audio_sha256: "a".repeat(64),
            wav_base64: "AAAA".into(),
            span: Some(RecognitionSpan {
                start_frame: 4,
                end_frame: 20,
            }),
        }
    }
    pub(crate) fn sample_response(request: &RecognitionRequest) -> RecognitionResponse {
        serde_json::from_value(json!({
            "schema_version":RECOGNITION_SCHEMA,"request_id":request.request_id,"asset_id":request.asset_id,
            "attempt_id":request.attempt_id,"revision_id":request.revision_id,"audio_sha256":request.audio_sha256,
            "original_audio":{"sample_rate":8000,"channels":1,"frames":32,"encoding":"pcm_s16le","bits":16},
            "source_pcm_sha256":"b".repeat(64),"source_pcm_hash_format":SOURCE_PCM_FORMAT,
            "span":{"start_frame":4,"end_frame":20},"span_coordinate_system":"original_frames_half_open",
            "derivative":{"sample_rate":16000,"channels":1,"frames":32,"pcm_sha256":"c".repeat(64),
                "pcm_hash_format":DERIVATIVE_PCM_FORMAT,"method":DERIVATIVE_METHOD,"scipy_version":"1.17.0","amplitude_normalization":false},
            "evidence_kind":"transcript_hypothesis","transcript":"Привет!","language":"ru","language_selection":"explicit_not_detected",
            "target_conditioned":false,"word_timestamps":null,"word_alignment_status":"not_run","confidence":null,"acoustic_assessment":null,
            "provenance":{"provider":"local_qwen3_asr","model":RECOGNITION_MODEL,"revision":RECOGNITION_REVISION,"runtime":RECOGNITION_RUNTIME,
                "mlx_version":"0.31.2","manifest_sha256":"d".repeat(64),"worker_sha256":"e".repeat(64),
                "decoding":{"temperature":0.0,"max_tokens":1024,"seed":42,"language":"Russian","hotwords":[]}},
            "timing":{"started_at":"2026-11-05T12:00:00+00:00","completed_at":"2026-11-05T12:00:01+00:00",
                "elapsed_seconds":1.0,"process_peak_rss_bytes":1000000,"metal_peak_bytes":2000000},
            "usage":{"prompt_tokens":20,"generation_tokens":4}
        })).unwrap()
    }

    #[test]
    fn recognition_dto_preserves_hypothesis_and_exact_source_span() {
        let request = sample_request();
        let response = sample_response(&request);
        response.validate_for(&request).unwrap();
        let encoded = serde_json::to_vec(&response).unwrap();
        let decoded: RecognitionResponse = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, response);
        let mut empty = response;
        empty.transcript.clear();
        empty.validate_for(&request).unwrap();
    }

    #[test]
    fn recognition_dto_rejects_target_conditioning_acoustic_claims_and_stale_binding() {
        let request = sample_request();
        let original = sample_response(&request);
        for pointer in [
            "/target_conditioned",
            "/word_timestamps",
            "/confidence",
            "/acoustic_assessment",
            "/request_id",
            "/asset_id",
            "/attempt_id",
            "/revision_id",
            "/audio_sha256",
            "/span/end_frame",
            "/derivative/frames",
            "/provenance/decoding/hotwords",
            "/provenance/revision",
        ] {
            let mut value = serde_json::to_value(&original).unwrap();
            *value.pointer_mut(pointer).unwrap() = match pointer {
                "/target_conditioned" => json!(true),
                "/word_timestamps" => json!([{"word":"Привет","start":0.0,"end":1.0}]),
                "/confidence" => json!(0.99),
                "/acoustic_assessment" => json!({"accent_score":100}),
                "/span/end_frame" | "/derivative/frames" => json!(1),
                "/provenance/decoding/hotwords" => json!(["Привет"]),
                _ => json!("stale"),
            };
            let changed: RecognitionResponse = serde_json::from_value(value).unwrap();
            assert!(changed.validate_for(&request).is_err(), "{pointer}");
        }
        let mut request_value = serde_json::to_value(&request).unwrap();
        request_value["expected_text"] = json!("Привет");
        assert!(serde_json::from_value::<RecognitionRequest>(request_value).is_err());
    }

    #[test]
    fn recognition_dto_bounds_hostile_values_without_frame_overflow() {
        let request = sample_request();
        let mut response = sample_response(&request);
        response.original_audio.frames = u64::MAX;
        response.span.end_frame = u64::MAX;
        assert!(response.validate_for(&request).is_err());
        response.original_audio.sample_rate = 0;
        assert!(response.validate_for(&request).is_err());
        let mut response = sample_response(&request);
        response.timing.elapsed_seconds = f64::NAN;
        assert!(response.validate_for(&request).is_err());
        let mut response = sample_response(&request);
        response.transcript = "x".repeat(RECOGNITION_MAX_TRANSCRIPT_BYTES + 1);
        assert!(response.validate_for(&request).is_err());
        let mut request = request;
        request.language = "auto".into();
        assert!(request.validate().is_err());
    }
}
