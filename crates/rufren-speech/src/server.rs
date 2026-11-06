use crate::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

const STYLE: &str = "Clear natural standard Russian for an adult language learner, warm conversational delivery, a measured natural pace, accurate word stress. Speak the supplied text verbatim, without introductions or commentary.";

struct Budget {
    count: u64,
    remaining: u64,
}
#[derive(Clone)]
pub struct SpeechService {
    provider: Option<Backend>,
    budget: Arc<Mutex<Budget>>,
    gate: Arc<tokio::sync::Semaphore>,
    max_requests: u64,
}
#[derive(Clone)]
enum Backend {
    Gemini(Provider),
    Macos,
    Qwen(crate::qwen_local::QwenLocal),
    LocalRussian(crate::local_russian::LocalRussian),
}
#[derive(Clone)]
struct Provider {
    client: Client,
    key: reqwest::header::HeaderValue,
    endpoint: String,
    timeout: Duration,
}

impl Default for SpeechService {
    fn default() -> Self {
        Self::disabled()
    }
}
impl SpeechService {
    pub fn disabled() -> Self {
        Self {
            provider: None,
            budget: Arc::new(Mutex::new(Budget {
                count: 0,
                remaining: 0,
            })),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            max_requests: 40,
        }
    }
    /// Reads explicit server environment only. No requests or billing mutations.
    pub fn from_env() -> Result<Self, SpeechError> {
        match std::env::var("RUFREN_SPEECH_PROVIDER")
            .unwrap_or_default()
            .as_str()
        {
            "" | "disabled" => Ok(Self::disabled()),
            "macos" => {
                let mut service = Self::macos()?;
                let max_requests = std::env::var("RUFREN_SPEECH_MAX_REQUESTS")
                    .map(|s| s.parse::<u64>().map_err(|_| SpeechError::Configuration))
                    .unwrap_or(Ok(40))?;
                if !(1..=1000).contains(&max_requests) {
                    return Err(SpeechError::Configuration);
                }
                service.max_requests = max_requests;
                Ok(service)
            }
            "qwen" | "local_russian" => {
                let hybrid =
                    std::env::var("RUFREN_SPEECH_PROVIDER").unwrap_or_default() == "local_russian";
                let token_path = std::env::var("RUFREN_QWEN_WORKER_TOKEN_FILE")
                    .unwrap_or_else(|_| ".local/qwen3-tts/worker.token".into());
                let metadata = std::fs::symlink_metadata(&token_path)
                    .map_err(|_| SpeechError::Configuration)?;
                if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1024
                {
                    return Err(SpeechError::Configuration);
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o077 != 0 {
                        return Err(SpeechError::Configuration);
                    }
                }
                let token =
                    std::fs::read_to_string(token_path).map_err(|_| SpeechError::Configuration)?;
                let port = std::env::var("RUFREN_QWEN_WORKER_PORT")
                    .map(|s| s.parse::<u16>().map_err(|_| SpeechError::Configuration))
                    .unwrap_or(Ok(8790))?;
                let mut service = if hybrid {
                    Self::local_russian(token.trim().into(), port)?
                } else {
                    Self::qwen(token.trim().into(), port)?
                };
                let max_requests = std::env::var("RUFREN_SPEECH_MAX_REQUESTS")
                    .map(|s| s.parse::<u64>().map_err(|_| SpeechError::Configuration))
                    .unwrap_or(Ok(40))?;
                if !(1..=1000).contains(&max_requests) {
                    return Err(SpeechError::Configuration);
                }
                service.max_requests = max_requests;
                Ok(service)
            }
            "gemini" => {
                let key =
                    std::env::var("GEMINI_API_KEY").map_err(|_| SpeechError::Configuration)?;
                let budget = std::env::var("RUFREN_SPEECH_SESSION_BUDGET_MICRO_USD")
                    .map(|s| s.parse().map_err(|_| SpeechError::Configuration))
                    .unwrap_or(Ok(DEFAULT_SESSION_BUDGET_MICRO_USD))?;
                let max_requests = std::env::var("RUFREN_SPEECH_MAX_REQUESTS")
                    .map(|s| s.parse::<u64>().map_err(|_| SpeechError::Configuration))
                    .unwrap_or(Ok(40))?;
                if !(1..=1000).contains(&max_requests) {
                    return Err(SpeechError::Configuration);
                }
                let mut service = Self::gemini(key, budget)?;
                service.max_requests = max_requests;
                Ok(service)
            }
            _ => Err(SpeechError::Configuration),
        }
    }
    pub fn gemini(api_key: String, session_budget_micro_usd: u64) -> Result<Self, SpeechError> {
        Self::configured(
            api_key,
            session_budget_micro_usd,
            "https://generativelanguage.googleapis.com/v1beta/interactions".into(),
            Duration::from_secs(45),
        )
    }
    pub(crate) fn configured(
        key: String,
        budget: u64,
        endpoint: String,
        timeout: Duration,
    ) -> Result<Self, SpeechError> {
        if key.trim().is_empty() || budget == 0 || budget > 5_000_000 {
            return Err(SpeechError::Configuration);
        }
        let mut key =
            reqwest::header::HeaderValue::from_str(&key).map_err(|_| SpeechError::Configuration)?;
        key.set_sensitive(true);
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| SpeechError::Configuration)?;
        Ok(Self {
            provider: Some(Backend::Gemini(Provider {
                client,
                key,
                endpoint,
                timeout,
            })),
            budget: Arc::new(Mutex::new(Budget {
                count: 0,
                remaining: budget,
            })),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            max_requests: 40,
        })
    }
    /// Installed macOS Russian voice, with no network or API billing.
    pub fn macos() -> Result<Self, SpeechError> {
        #[cfg(target_os = "macos")]
        {
            if !std::path::Path::new("/usr/bin/say").is_file() {
                return Err(SpeechError::Configuration);
            }
            Ok(Self {
                provider: Some(Backend::Macos),
                budget: Arc::new(Mutex::new(Budget {
                    count: 0,
                    remaining: 0,
                })),
                gate: Arc::new(tokio::sync::Semaphore::new(1)),
                max_requests: 40,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(SpeechError::Configuration)
        }
    }
    /// Local full-BF16 worker. Destination is fixed IPv4 loopback; model setup,
    /// process launch and operator preparation happen explicitly outside this constructor.
    pub fn qwen(worker_token: String, port: u16) -> Result<Self, SpeechError> {
        Ok(Self {
            provider: Some(Backend::Qwen(crate::qwen_local::QwenLocal::new(
                worker_token,
                port,
            )?)),
            budget: Arc::new(Mutex::new(Budget {
                count: 0,
                remaining: 0,
            })),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            max_requests: 40,
        })
    }

    /// Declared local Russian strategy: authored stress uses Silero, suitable
    /// ordinary full utterances use Qwen, other short input uses Silero's
    /// automatic frontend. A selected engine's failure never changes models.
    pub fn local_russian(worker_token: String, port: u16) -> Result<Self, SpeechError> {
        let mut service = Self::disabled();
        service.provider = Some(Backend::LocalRussian(
            crate::local_russian::LocalRussian::installed(crate::qwen_local::QwenLocal::new(
                worker_token,
                port,
            )?)?,
        ));
        Ok(service)
    }

    pub fn status(&self) -> SpeechStatus {
        let b = self.budget.lock().unwrap_or_else(|p| p.into_inner());
        let (provider, model, voice, message, data_sent, cost) = match &self.provider {
            None => (
                None,
                None,
                None,
                "Configure a speech provider on the private relay.",
                "No speech provider is enabled.",
                0,
            ),
            Some(Backend::Gemini(_)) => (
                Some("gemini".into()),
                Some(MODEL.into()),
                Some(VOICE.into()),
                "AI reference speech is configured. Account access and Russian output quality still need live validation.",
                "Only the requested text and fixed Russian voice instructions are sent to Gemini; learner recordings are not uploaded.",
                RESERVATION_MICRO_USD,
            ),
            Some(Backend::Macos) => (
                Some("macos_say".into()),
                Some("Milena".into()),
                Some("Milena".into()),
                "Local Russian system speech uses the installed Milena voice on the relay Mac.",
                "The requested text is synthesized locally on the relay Mac; no speech-provider network request is made.",
                0,
            ),
            Some(Backend::Qwen(_)) => (
                Some("local_qwen3_tts".into()),
                Some(crate::qwen_local::QWEN_MODEL.into()),
                Some(crate::qwen_local::QWEN_VOICE.into()),
                "Local Qwen3 is configured; check worker readiness before sending text.",
                "Text is synthesized on the relay Mac by the explicitly installed local Qwen3 model; no cloud speech request is made.",
                0,
            ),
            Some(Backend::LocalRussian(_)) => (
                Some("local_russian".into()),
                Some(crate::local_russian::STRATEGY.into()),
                Some("Qwen designed voice / Silero xenia".into()),
                "Local Russian routing is configured; both installed engines must be ready. Authored stress uses Silero; contextual ordinary text uses Qwen; short ordinary text uses Silero. Ambiguous unmarked short phrases need a stress mark or context. No fallback on failure.",
                "Requested text stays on the relay Mac. Responses identify the actual model and exact synthesis input; no cloud speech request is made.",
                0,
            ),
        };
        // Qwen readiness is asynchronous; synchronous status never guesses it.
        let enabled = self.provider.is_some()
            && !matches!(
                &self.provider,
                Some(Backend::Qwen(_) | Backend::LocalRussian(_))
            );
        let mut remaining = self.max_requests.saturating_sub(b.count);
        if !enabled {
            remaining = 0;
        } else if cost > 0 {
            remaining = remaining.min(b.remaining / cost);
        }
        SpeechStatus {
            enabled,
            provider,
            model,
            voice,
            message: message.into(),
            data_sent: data_sent.into(),
            max_text_bytes: MAX_TEXT_BYTES,
            max_text_chars: MAX_TEXT_CHARS,
            max_duration_seconds: MAX_DURATION_SECONDS,
            remaining_reservations: remaining,
            remaining_budget_micro_usd: b.remaining,
        }
    }
    /// Checks local worker health/readiness without loading models or generating audio.
    /// Other providers retain their existing configuration-only status contract.
    pub async fn checked_status(&self) -> SpeechStatus {
        let mut status = self.status();
        if let Some(Backend::Qwen(client)) = &self.provider {
            match client.status().await {
                Ok(worker) => {
                    status.enabled = worker.enabled && worker.loaded;
                    let budget = self.budget.lock().unwrap_or_else(|p| p.into_inner());
                    status.remaining_reservations = if status.enabled {
                        self.max_requests.saturating_sub(budget.count)
                    } else {
                        0
                    };
                    status.message=if status.enabled{"Local Qwen3 is ready. Marked short phrases and ambiguous stress contrasts need an explicit-stress voice."}
                        else{"Local Qwen3 is installed but not ready. The relay operator must start and prepare its pinned worker; no speech was generated."}.into();
                }
                Err(_) => {
                    status.enabled = false;
                    status.remaining_reservations = 0;
                    status.message="Local Qwen3 is unavailable or has an incompatible worker contract. Start the pinned worker and prepare it before enabling this provider.".into();
                }
            }
        }
        if let Some(Backend::LocalRussian(router)) = &self.provider {
            status.enabled = router.ready().await;
            let budget = self.budget.lock().unwrap_or_else(|p| p.into_inner());
            status.remaining_reservations = if status.enabled {
                self.max_requests.saturating_sub(budget.count)
            } else {
                0
            };
            if !status.enabled {
                status.message = "Local Russian routing is not ready. Verify the pinned Silero installation and start/prepare the Qwen worker. No model substitution or audio generation occurs during this check.".into();
            }
        }
        status
    }

    pub async fn generate(&self, request: &SpeechRequest) -> Result<SpeechResponse, SpeechError> {
        request.validate()?;
        crate::generation_text::validate(&request.text)?;
        let provider = self.provider.as_ref().ok_or(SpeechError::Disabled)?;
        if matches!(provider, Backend::Qwen(_)) {
            crate::qwen_frontend::input(&request.text)?;
        }
        if matches!(provider, Backend::LocalRussian(_)) {
            crate::local_russian::route(&request.text)?;
        }
        let _permit = self
            .gate
            .try_acquire()
            .map_err(|_| SpeechError::RateLimited)?;
        {
            let mut b = self.budget.lock().map_err(|_| SpeechError::Unavailable)?;
            let reservation = if matches!(provider, Backend::Gemini(_)) {
                RESERVATION_MICRO_USD
            } else {
                0
            };
            if b.count >= self.max_requests || b.remaining < reservation {
                return Err(SpeechError::BudgetExceeded);
            }
            // Never refund cancellation/timeouts: generation may already have been billed.
            b.count += 1;
            b.remaining -= reservation;
        }
        match provider {
            Backend::Gemini(provider) => {
                tokio::time::timeout(provider.timeout, provider.generate(request))
                    .await
                    .map_err(|_| SpeechError::Timeout)?
            }
            Backend::Qwen(client) => client.generate(request).await,
            Backend::LocalRussian(router) => router.generate(request).await,
            Backend::Macos => {
                #[cfg(target_os = "macos")]
                {
                    crate::macos::generate(request).await
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(SpeechError::Configuration)
                }
            }
        }
    }
}

impl Provider {
    async fn generate(&self, request: &SpeechRequest) -> Result<SpeechResponse, SpeechError> {
        // Exact transcript and separate style follow the current official 3.8 TTS guide.
        let payload = json!({"model":MODEL,"store":false,
            "input":[{"type":"user_input","content":[{"type":"text","text":request.text,"annotations":[{"type":"speech_metadata","style":STYLE}]}]}],
            "response_format":{"type":"audio","mime_type":"audio/wav","delivery":"inline","sample_rate":SAMPLE_RATE},
            "generation_config":{"speech_config":[{"voice":VOICE}],"max_output_tokens":MAX_OUTPUT_TOKENS}});
        let encoded = serde_json::to_vec(&payload).map_err(|_| SpeechError::InvalidRequest)?;
        if encoded.len() > 8192 {
            return Err(SpeechError::InvalidRequest);
        }
        let mut response = self
            .client
            .post(&self.endpoint)
            .header("x-goog-api-key", self.key.clone())
            .header("Content-Type", "application/json")
            .body(encoded)
            .send()
            .await
            .map_err(http_error)?;
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(SpeechError::Authentication);
            }
            StatusCode::TOO_MANY_REQUESTS => return Err(SpeechError::RateLimited),
            s if !s.is_success() => return Err(SpeechError::Unavailable),
            _ => (),
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE_BYTES as u64)
        {
            return Err(SpeechError::ResponseTooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(http_error)? {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(SpeechError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| SpeechError::InvalidResponse)?;
        parse(request, value)
    }
}
fn http_error(error: reqwest::Error) -> SpeechError {
    if error.is_timeout() {
        SpeechError::Timeout
    } else {
        SpeechError::Unavailable
    }
}

fn parse(request: &SpeechRequest, value: Value) -> Result<SpeechResponse, SpeechError> {
    if value.get("status").and_then(Value::as_str) != Some("completed")
        || value.get("model").and_then(Value::as_str) != Some(MODEL)
    {
        return Err(SpeechError::InvalidResponse);
    }
    let steps = value
        .get("steps")
        .and_then(Value::as_array)
        .ok_or(SpeechError::InvalidResponse)?;
    if steps.is_empty() || steps.len() > 4 {
        return Err(SpeechError::InvalidResponse);
    }
    let mut audio = None;
    for step in steps {
        match step.get("type").and_then(Value::as_str) {
            Some("user_input") => (),
            Some("model_output") => {
                let blocks = step
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or(SpeechError::InvalidResponse)?;
                if blocks.len() != 1
                    || audio.is_some()
                    || blocks[0].get("type").and_then(Value::as_str) != Some("audio")
                {
                    return Err(SpeechError::InvalidResponse);
                }
                audio = Some(&blocks[0]);
            }
            _ => return Err(SpeechError::InvalidResponse),
        }
    }
    let audio = audio.ok_or(SpeechError::InvalidResponse)?;
    if !audio
        .get("mime_type")
        .and_then(Value::as_str)
        .is_some_and(|s| s.eq_ignore_ascii_case("audio/wav"))
        || audio
            .get("sample_rate")
            .is_some_and(|v| v.as_u64() != Some(SAMPLE_RATE as u64))
        || audio.get("channels").is_some_and(|v| v.as_u64() != Some(1))
        || audio.get("uri").is_some()
    {
        return Err(SpeechError::InvalidResponse);
    }
    let data = audio
        .get("data")
        .and_then(Value::as_str)
        .ok_or(SpeechError::InvalidResponse)?;
    if data.len() > (MAX_PCM_BYTES + 4096).div_ceil(3) * 4 {
        return Err(SpeechError::ResponseTooLarge);
    }
    let encoded_wav = STANDARD
        .decode(data)
        .map_err(|_| SpeechError::InvalidResponse)?;
    let pcm = crate::wav::decode(&encoded_wav)?;
    let usage = value.get("usage");
    if usage.is_some_and(|u| !u.is_object()) {
        return Err(SpeechError::InvalidResponse);
    }
    let count = |name: &str| -> Result<Option<u64>, SpeechError> {
        usage
            .and_then(|u| u.get(name))
            .map(|v| v.as_u64().ok_or(SpeechError::InvalidResponse))
            .transpose()
    };
    let input_tokens = count("total_input_tokens")?;
    let output_tokens = count("total_output_tokens")?;
    if input_tokens.is_some_and(|n| n > MAX_INPUT_TOKENS)
        || output_tokens.is_some_and(|n| n > MAX_OUTPUT_TOKENS)
        || value
            .pointer("/usage/total_tool_use_tokens")
            .is_some_and(|v| v.as_u64() != Some(0))
        || value
            .pointer("/usage/total_thought_tokens")
            .is_some_and(|v| v.as_u64() != Some(0))
    {
        return Err(SpeechError::InvalidResponse);
    }
    let wav = crate::wav::encode(pcm);
    Ok(SpeechResponse{request_id:request.request_id.clone(),revision_id:request.revision_id.clone(),source_text:request.text.clone(),wav_base64:STANDARD.encode(&wav),sha256:format!("{:x}",Sha256::digest(&wav)),sample_rate:SAMPLE_RATE,channels:1,frames:(pcm.len()/2)as u64,
        provenance:SpeechProvenance{kind:SpeechProvenanceKind::AiGeneratedSpeech,provider:"gemini".into(),model:MODEL.into(),voice:VOICE.into(),prompt_version:"rufren-russian-reference-v1".into(),validation:"AI-generated pronunciation model; no human/native-speaker certification or automatic acoustic assessment".into(),synthesis_input:None},
        usage:SpeechUsage{input_tokens,output_tokens,estimated_micro_usd:input_tokens.zip(output_tokens).map(|(i,o)|i+o*20),reserved_micro_usd:RESERVATION_MICRO_USD}})
}
