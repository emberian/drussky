//! Russian AI reference-speech generation. Shared DTOs work without the server feature.
//! Reference generation and independent local transcription are separate services;
//! neither claims human/native verification or acoustic correctness grading.
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
mod local_russian;
#[cfg(all(feature = "server", target_os = "macos"))]
mod macos;
mod qwen_frontend;
#[cfg(feature = "server")]
mod qwen_local;
mod recognition;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
mod silero_local;
pub use recognition::*;
#[cfg(feature = "server")]
mod recognition_server;
#[cfg(feature = "server")]
pub use recognition_server::{RecognitionCancellation, RecognitionService};
#[cfg(feature = "server")]
mod wav;
#[cfg(feature = "server")]
pub use server::SpeechService;

pub const MODEL: &str = "gemini-3.8-flash-tts";
pub const VOICE: &str = "Kore";
pub const SAMPLE_RATE: u32 = 24_000;
pub const MAX_TEXT_BYTES: usize = 1024;
pub const MAX_TEXT_CHARS: usize = 256;
pub const MAX_DURATION_SECONDS: usize = 45;
pub const MAX_PCM_BYTES: usize = SAMPLE_RATE as usize * 2 * MAX_DURATION_SECONDS;
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_INPUT_TOKENS: u64 = 4096;
pub const MAX_OUTPUT_TOKENS: u64 = 1024;
/// Conservative estimate at $1/$20 per million input/output tokens, above the
/// currently documented 3.8 rates. Process reservation, not an account billing cap.
pub const RESERVATION_MICRO_USD: u64 = MAX_INPUT_TOKENS + MAX_OUTPUT_TOKENS * 20;
pub const DEFAULT_SESSION_BUDGET_MICRO_USD: u64 = 1_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechRequest {
    pub text: String,
    pub request_id: String,
    #[serde(default)]
    pub revision_id: Option<String>,
}
impl SpeechRequest {
    pub fn validate(&self) -> Result<(), SpeechError> {
        let id = |s: &str| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        };
        if self.text.trim().is_empty() || self.text.len() > MAX_TEXT_BYTES
            || self.text.chars().count() > MAX_TEXT_CHARS
            || self.text.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
            || !self.text.chars().any(char::is_alphabetic)
            // 3.8 inline vocal tags are intentionally not enabled in lesson references.
            || self.text.chars().any(|c| matches!(c, '<' | '>'))
            || self.text.contains("[[")
            || !id(&self.request_id) || self.revision_id.as_ref().is_some_and(|s| !id(s))
        {
            return Err(SpeechError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpeechProvenanceKind {
    AiGeneratedSpeech,
    SystemGeneratedSpeech,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechProvenance {
    pub kind: SpeechProvenanceKind,
    pub provider: String,
    pub model: String,
    pub voice: String,
    pub prompt_version: String,
    /// AI generation is provenance; it is not a human/native-speaker certification.
    pub validation: String,
    /// Exact model input when a provider has an explicit text frontend. Legacy
    /// providers omit this field; its context is a heuristic category, not a
    /// certification that the intended pronunciation was disambiguated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthesis_input: Option<SpeechSynthesisInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechSynthesisInput {
    pub source_text_sha256: String,
    pub synthesis_text: String,
    pub synthesis_text_sha256: String,
    pub transform: SpeechTextTransform,
    pub context: SpeechSynthesisContext,
}

impl SpeechSynthesisInput {
    /// Check the exact declared frontend operation, byte hashes, bounded source
    /// and actual synthesis text. Context categories describe this policy's
    /// input shape; they never establish audible stress or disambiguation.
    pub fn validate_for(&self, source_text: &str) -> Result<(), SpeechError> {
        use sha2::{Digest, Sha256};
        let bounded = |text: &str| {
            !text.trim().is_empty()
                && text.len() <= MAX_TEXT_BYTES
                && text.chars().count() <= MAX_TEXT_CHARS
                && text.chars().any(char::is_alphabetic)
                && !text
                    .chars()
                    .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
                && !text.chars().any(|c| matches!(c, '<' | '>'))
                && !text.contains("[[")
        };
        if !bounded(source_text) || !bounded(&self.synthesis_text) {
            return Err(SpeechError::InvalidResponse);
        }
        let hash = |text: &str| format!("{:x}", Sha256::digest(text.as_bytes()));
        if self.source_text_sha256 != hash(source_text)
            || self.synthesis_text_sha256 != hash(&self.synthesis_text)
        {
            return Err(SpeechError::InvalidResponse);
        }
        let expected = match self.transform {
            SpeechTextTransform::Verbatim => SpeechSynthesisInput {
                source_text_sha256: hash(source_text),
                synthesis_text: source_text.into(),
                synthesis_text_sha256: hash(source_text),
                transform: SpeechTextTransform::Verbatim,
                context: SpeechSynthesisContext::OrdinaryText,
            },
            SpeechTextTransform::RemoveCombiningAcuteInContext => {
                qwen_frontend::input(source_text).map_err(|_| SpeechError::InvalidResponse)?
            }
            SpeechTextTransform::PedagogicalAcuteToSileroPlus => {
                qwen_frontend::silero_input(source_text)
                    .map_err(|_| SpeechError::InvalidResponse)?
            }
        };
        if *self != expected {
            return Err(SpeechError::InvalidResponse);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpeechTextTransform {
    Verbatim,
    RemoveCombiningAcuteInContext,
    PedagogicalAcuteToSileroPlus,
}

/// Input-shape categories only; neither claims semantic disambiguation or
/// pronunciation accuracy.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpeechSynthesisContext {
    OrdinaryText,
    ContextualUtterance,
    ExplicitStress,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    /// Conservative estimate at reservation rates, if provider usage is available.
    pub estimated_micro_usd: Option<u64>,
    pub reserved_micro_usd: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechResponse {
    /// These bindings are echoed by the gateway, never invented by the model.
    pub request_id: String,
    pub revision_id: Option<String>,
    pub source_text: String,
    pub wav_base64: String,
    /// SHA-256 of the exact canonical WAV bytes, matching the asset-store contract.
    pub sha256: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub provenance: SpeechProvenance,
    pub usage: SpeechUsage,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechStatus {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub message: String,
    pub data_sent: String,
    pub max_text_bytes: usize,
    pub max_text_chars: usize,
    pub max_duration_seconds: usize,
    pub remaining_reservations: u64,
    pub remaining_budget_micro_usd: u64,
}

/// Deliberately excludes provider bodies, request text, credential values and URLs.
#[derive(Debug, thiserror::Error)]
pub enum SpeechError {
    #[error("AI reference speech needs a configured speech provider on the private relay")]
    Disabled,
    #[error("Invalid speech provider configuration")]
    Configuration,
    #[error("Choose a short text within the speech limits and valid request/revision identities")]
    InvalidRequest,
    #[error(
        "This voice cannot reliably honor the marked stress in this short or ambiguous phrase. Keep the original text and choose an explicit-stress voice; no audio was generated"
    )]
    UnsupportedStress,
    #[error(
        "This short phrase has an ambiguous Russian stress pattern. Add the intended stress mark (for example, за́мок or замо́к), or supply a full sentence that makes the meaning clear; no audio was generated"
    )]
    AmbiguousStress,
    #[error("This relay process has used its speech generation budget")]
    BudgetExceeded,
    #[error("The speech provider rejected its API credential")]
    Authentication,
    #[error("The speech provider or relay is busy; try again later")]
    RateLimited,
    #[error("Speech generation timed out; no automatic retry was made")]
    Timeout,
    #[error("The speech provider is unavailable")]
    Unavailable,
    #[error("The speech provider returned invalid or incomplete audio")]
    InvalidResponse,
    #[error("The speech response exceeded its size or duration limit")]
    ResponseTooLarge,
}

#[cfg(all(test, feature = "server"))]
mod tests;
