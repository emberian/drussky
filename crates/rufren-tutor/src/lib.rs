//! Primary AI teaching and exact-response language feedback. Transport and credentials stay server-side.
//! AI judgments retain their own provenance and never masquerade as human or acoustic evidence.

pub use rufren_tutor_contract::*;
#[cfg(feature = "server")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
mod anthropic;
#[cfg(feature = "server")]
mod claude_cli;
#[cfg(feature = "server")]
mod teacher;

#[cfg(feature = "server")]
struct Budget {
    reservations: u64,
    remaining_micro_usd: u64,
}

#[cfg(feature = "server")]
#[derive(Clone)]
enum Provider {
    Anthropic(anthropic::Anthropic),
    ClaudeCli(claude_cli::ClaudeCli),
}

/// Clone this service into request handlers: clones share one budget and concurrency gate.
#[cfg(feature = "server")]
#[derive(Clone)]
pub struct TutorService {
    provider: Option<Provider>,
    budget: Arc<Mutex<Budget>>,
    gate: Arc<tokio::sync::Semaphore>,
    max_requests: u64,
}

#[cfg(feature = "server")]
impl Default for TutorService {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(feature = "server")]
impl TutorService {
    pub fn disabled() -> Self {
        Self {
            provider: None,
            budget: Arc::new(Mutex::new(Budget {
                reservations: 0,
                remaining_micro_usd: 0,
            })),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            max_requests: 20,
        }
    }

    /// Opt-in requires BOTH the provider selector and a server-side credential.
    /// Does not contact a provider, inspect billing, or read credentials from disk.
    pub fn from_env() -> Result<Self, TutorError> {
        let provider = std::env::var("RUFREN_TUTOR_PROVIDER").unwrap_or_default();
        match provider.as_str() {
            "" | "disabled" => Ok(Self::disabled()),
            "claude_cli" => {
                let max_requests = std::env::var("RUFREN_TUTOR_MAX_REQUESTS")
                    .map(|s| s.parse::<u64>().map_err(|_| TutorError::Configuration))
                    .unwrap_or(Ok(20))?;
                if !(1..=1000).contains(&max_requests) {
                    return Err(TutorError::Configuration);
                }
                let mut service = Self::disabled();
                service.provider = Some(Provider::ClaudeCli(claude_cli::ClaudeCli::from_env()?));
                service.max_requests = max_requests;
                Ok(service)
            }
            "anthropic" => {
                let key =
                    std::env::var("ANTHROPIC_API_KEY").map_err(|_| TutorError::Configuration)?;
                let budget = std::env::var("RUFREN_TUTOR_SESSION_BUDGET_MICRO_USD")
                    .map(|s| s.parse::<u64>().map_err(|_| TutorError::Configuration))
                    .unwrap_or(Ok(DEFAULT_SESSION_BUDGET_MICRO_USD))?;
                // Deliberately modest process cap. Raising it requires a code/config review.
                if budget == 0 || budget > 5_000_000 {
                    return Err(TutorError::Configuration);
                }
                let max_requests = std::env::var("RUFREN_TUTOR_MAX_REQUESTS")
                    .map(|s| s.parse::<u64>().map_err(|_| TutorError::Configuration))
                    .unwrap_or(Ok(20))?;
                if !(1..=1000).contains(&max_requests) {
                    return Err(TutorError::Configuration);
                }
                let mut service = Self::anthropic(key, budget)?;
                if let Some(Provider::Anthropic(api)) = service.provider.as_mut() {
                    let model =
                        std::env::var("RUFREN_TUTOR_MODEL").unwrap_or_else(|_| MODEL.into());
                    api.select_model(&model)?;
                }
                service.max_requests = max_requests;
                Ok(service)
            }
            _ => Err(TutorError::Configuration),
        }
    }

    pub fn anthropic(api_key: String, session_budget_micro_usd: u64) -> Result<Self, TutorError> {
        Self::configured(
            api_key,
            session_budget_micro_usd,
            "https://api.anthropic.com".into(),
            Duration::from_secs(100),
        )
    }

    fn configured(
        key: String,
        budget: u64,
        base_url: String,
        timeout: Duration,
    ) -> Result<Self, TutorError> {
        if budget == 0 || budget > 5_000_000 {
            return Err(TutorError::Configuration);
        }
        Ok(Self {
            provider: Some(Provider::Anthropic(anthropic::Anthropic::new(
                key, base_url, timeout,
            )?)),
            budget: Arc::new(Mutex::new(Budget {
                reservations: 0,
                remaining_micro_usd: budget,
            })),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            max_requests: 20,
        })
    }

    pub fn status(&self) -> TutorStatus {
        let b = self.budget.lock().unwrap_or_else(|p| p.into_inner());
        let enabled = self.provider.is_some();
        let api = matches!(self.provider, Some(Provider::Anthropic(_)));
        TutorStatus {
            enabled,
            lesson_flow: enabled.then_some(LessonFlowCapability::GuidedV1),
            provider:self.provider.as_ref().map(|p|match p {Provider::Anthropic(_)=>"anthropic",Provider::ClaudeCli(_)=>"claude_cli"}.into()),
            model:match &self.provider {Some(Provider::Anthropic(p))=>Some(p.model().into()),Some(Provider::ClaudeCli(p))=>Some(p.configured_profile()),_=>None},
            message:match &self.provider {
                Some(Provider::Anthropic(_))=>"AI teaching is configured through Anthropic; account credit has not been verified.",
                Some(Provider::ClaudeCli(_))=>"AI teaching uses this relay host's explicitly configured Claude CLI login and subscription limits. API-dollar budgets do not control that quota.",
                None=>"Configure an AI teacher on the private relay: RUFREN_TUTOR_PROVIDER=claude_cli for an existing local Claude login, or anthropic with a server API key.",
            }.into(),
            data_sent:"Only the submitted lesson/task, exact response, references and optional context are sent to the configured AI teacher. No audio or whole notebook is sent.".into(),
            max_text_bytes:MAX_TEXT_BYTES,max_context_bytes:MAX_CONTEXT_BYTES,
            remaining_reservations:if enabled {
                let count=self.max_requests.saturating_sub(b.reservations);
                if let Some(Provider::Anthropic(p))=&self.provider {count.min(b.remaining_micro_usd/p.reservation())}else{count}
            }else{0},
            remaining_budget_micro_usd:b.remaining_micro_usd,
            accounting:if api{UsageAccounting::ApiTokenEstimate}else{UsageAccounting::SubscriptionQuota},
        }
    }

    pub async fn suggest(&self, request: &TutorRequest) -> Result<TutorResponse, TutorError> {
        request.validate()?;
        let provider = self.provider.as_ref().ok_or(TutorError::Disabled)?;
        let _permit = self.admit()?;
        match provider {
            Provider::Anthropic(p) => p.suggest(request).await,
            Provider::ClaudeCli(p) => p.suggest(request).await,
        }
    }

    pub async fn teach(&self, request: &TeacherRequest) -> Result<TeacherResponse, TutorError> {
        request.validate()?;
        let provider = self.provider.as_ref().ok_or(TutorError::Disabled)?;
        let _permit = self.admit()?;
        match provider {
            Provider::Anthropic(p) => p.teach(request).await,
            Provider::ClaudeCli(p) => p.teach(request).await,
        }
    }

    fn admit(&self) -> Result<tokio::sync::SemaphorePermit<'_>, TutorError> {
        // Reject concurrent work instead of accumulating an unbounded paid request queue.
        let permit = self
            .gate
            .try_acquire()
            .map_err(|_| TutorError::RateLimited)?;
        {
            let mut b = self.budget.lock().map_err(|_| TutorError::Unavailable)?;
            let reservation = match &self.provider {
                Some(Provider::Anthropic(p)) => p.reservation(),
                _ => 0,
            };
            if b.reservations >= self.max_requests || b.remaining_micro_usd < reservation {
                return Err(TutorError::BudgetExceeded);
            }
            // Keep the full reservation even on failure: a timed-out generation may be billed.
            b.reservations += 1;
            b.remaining_micro_usd -= reservation;
        }
        Ok(permit)
    }
}

#[cfg(all(test, feature = "server"))]
mod tests;
