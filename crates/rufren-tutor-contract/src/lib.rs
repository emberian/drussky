//! Network-free AI teaching contracts, exact task bindings and bounded validation.
//! Shared by the provider gateway, durable learning history and application.

use serde::{Deserialize, Serialize};
mod teacher;
pub use teacher::*;

pub const MAX_TEXT_BYTES: usize = 8192;
pub const MAX_CONTEXT_BYTES: usize = 2048;
pub const MAX_RESPONSE_BYTES: usize = 131072;
pub const MODEL: &str = "claude-opus-5-5";
pub const MAX_INPUT_TOKENS: u64 = 8192;
pub const MAX_OUTPUT_TOKENS: u64 = 8192;
/// Conservative reservation at the documented standard $4/$20 per million token rates.
/// This is an application estimate, not an invoice or provider account spending limit.
pub const RESERVATION_MICRO_USD: u64 = MAX_INPUT_TOKENS * 4 + MAX_OUTPUT_TOKENS * 20;
pub const DEFAULT_SESSION_BUDGET_MICRO_USD: u64 = 800_000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TutorTask {
    Explain,
    WritingFeedback,
    PrepareContent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TutorRequest {
    pub task: TutorTask,
    pub text: String,
    #[serde(default)]
    pub learner_context: Option<String>,
}

impl TutorRequest {
    pub fn validate(&self) -> Result<(), TutorError> {
        if self.text.trim().is_empty() || self.text.len() > MAX_TEXT_BYTES {
            return Err(TutorError::InvalidRequest);
        }
        if self
            .learner_context
            .as_ref()
            .is_some_and(|s| s.len() > MAX_CONTEXT_BYTES)
        {
            return Err(TutorError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TutorExample {
    pub russian: String,
    pub english: String,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TutorContent {
    pub explanation: String,
    pub examples: Vec<TutorExample>,
    pub next_step: String,
    pub caveats: Vec<String>,
}

impl TutorContent {
    pub fn validate(&self) -> Result<(), TutorError> {
        fn bounded(s: &str, max: usize) -> bool {
            !s.trim().is_empty() && s.len() <= max
        }
        if !bounded(&self.explanation, 12000)
            || !bounded(&self.next_step, 2000)
            || self.examples.len() > 6
            || self.caveats.len() > 6
            || self.examples.iter().any(|e| {
                !bounded(&e.russian, 2000) || !bounded(&e.english, 2000) || !bounded(&e.note, 2000)
            })
            || self.caveats.iter().any(|s| !bounded(s, 2000))
        {
            return Err(TutorError::InvalidResponse);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    UnreviewedAiSuggestion,
    AiGeneratedTeaching,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TutorProvenance {
    pub provider: String,
    pub model: String,
    pub review_status: ReviewStatus,
    pub scope: String,
    pub prompt_version: String,
    #[serde(default)]
    pub response_id: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageAccounting {
    #[default]
    ApiTokenEstimate,
    SubscriptionQuota,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TutorUsage {
    #[serde(default)]
    pub accounting: UsageAccounting,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_micro_usd: u64,
    pub reserved_micro_usd: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TutorResponse {
    pub task: TutorTask,
    pub content: TutorContent,
    pub provenance: TutorProvenance,
    pub usage: TutorUsage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TutorStatus {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub message: String,
    pub data_sent: String,
    pub max_text_bytes: usize,
    pub max_context_bytes: usize,
    pub remaining_reservations: u64,
    pub remaining_budget_micro_usd: u64,
    #[serde(default)]
    pub accounting: UsageAccounting,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lesson_flow: Option<LessonFlowCapability>,
}

/// Errors deliberately exclude provider response bodies, request text, URLs and secrets.
#[derive(Debug, thiserror::Error)]
pub enum TutorError {
    #[error("AI tutoring is not configured on the private relay")]
    Disabled,
    #[error("Invalid tutor configuration")]
    Configuration,
    #[error("Enter nonempty text within the tutor's text and context limits")]
    InvalidRequest,
    #[error("The relay's AI request budget is exhausted; review its configuration")]
    BudgetExceeded,
    #[error("The provider rejected the API credential")]
    Authentication,
    #[error("The provider is rate limited; retry later")]
    RateLimited,
    #[error("The provider request timed out; no automatic retry was made")]
    Timeout,
    #[error("The provider is unavailable")]
    Unavailable,
    #[error("The provider returned an invalid or incomplete tutor response")]
    InvalidResponse,
    #[error("The provider response exceeded the size limit")]
    ResponseTooLarge,
}
