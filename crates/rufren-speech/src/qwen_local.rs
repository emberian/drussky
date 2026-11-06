//! Bounded local Qwen3 reference worker client with explicit readiness and text provenance.
use crate::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, StatusCode, header::HeaderValue};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub(crate) const QWEN_MODEL: &str =
    "mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-bf16@7d3824abff87e49756bb0f83fb5411de75d160c4";
pub(crate) const QWEN_VOICE: &str = "rufren-russian-warm-v1";
const QWEN_PROMPT: &str = "rufren-qwen-russian-v2";
const QWEN_WORKER_SCHEMA: &str = "rufren-qwen-worker-v2";
const LOCAL_RESPONSE_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct QwenLocal {
    client: Client,
    authorization: HeaderValue,
    base: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct QwenWorkerStatus {
    pub contract_version: String,
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub voice: String,
    pub loaded: bool,
    pub max_text_bytes: usize,
    pub max_text_chars: usize,
}

impl QwenLocal {
    /// Server-supplied private worker token; destination is always IPv4 loopback.
    /// Does not launch a process, load a model, download weights or generate speech.
    pub(crate) fn new(token: String, port: u16) -> Result<Self, SpeechError> {
        if token.len() < 32 || port < 1024 {
            return Err(SpeechError::Configuration);
        }
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| SpeechError::Configuration)?;
        authorization.set_sensitive(true);
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(55))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| SpeechError::Configuration)?;
        Ok(Self {
            client,
            authorization,
            base: format!("http://127.0.0.1:{port}"),
        })
    }

    async fn exchange(&self, request: Option<&SpeechRequest>) -> Result<Vec<u8>, SpeechError> {
        let query = if let Some(request) = request {
            request.validate()?;
            let encoded = serde_json::to_vec(request).map_err(|_| SpeechError::InvalidRequest)?;
            if encoded.len() > 8192 {
                return Err(SpeechError::InvalidRequest);
            }
            self.client
                .post(format!("{}/generate", self.base))
                .header("Content-Type", "application/json")
                .body(encoded)
        } else {
            self.client
                .get(format!("{}/status", self.base))
                .timeout(Duration::from_secs(5))
        };
        let mut response = query
            .header("Authorization", self.authorization.clone())
            .send()
            .await
            .map_err(http_error)?;
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(SpeechError::Authentication);
            }
            StatusCode::TOO_MANY_REQUESTS => return Err(SpeechError::RateLimited),
            StatusCode::UNPROCESSABLE_ENTITY => return Err(SpeechError::UnsupportedStress),
            StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => {
                return Err(SpeechError::Timeout);
            }
            s if !s.is_success() => return Err(SpeechError::Unavailable),
            _ => (),
        }
        if response
            .content_length()
            .is_some_and(|n| n > LOCAL_RESPONSE_LIMIT as u64)
        {
            return Err(SpeechError::ResponseTooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(http_error)? {
            if bytes.len() + chunk.len() > LOCAL_RESPONSE_LIMIT {
                return Err(SpeechError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    pub(crate) async fn status(&self) -> Result<QwenWorkerStatus, SpeechError> {
        let status: QwenWorkerStatus = serde_json::from_slice(&self.exchange(None).await?)
            .map_err(|_| SpeechError::InvalidResponse)?;
        if status.contract_version != QWEN_WORKER_SCHEMA
            || (status.enabled && !status.loaded)
            || status.provider != "local_qwen3_tts"
            || status.model != QWEN_MODEL
            || status.voice != QWEN_VOICE
            || status.max_text_bytes != MAX_TEXT_BYTES
            || status.max_text_chars != MAX_TEXT_CHARS
        {
            return Err(SpeechError::InvalidResponse);
        }
        Ok(status)
    }

    pub(crate) async fn generate(
        &self,
        request: &SpeechRequest,
    ) -> Result<SpeechResponse, SpeechError> {
        request.validate()?;
        crate::qwen_frontend::input(&request.text)?;
        tokio::time::timeout(Duration::from_secs(55), self.generate_ready(request))
            .await
            .map_err(|_| SpeechError::Timeout)?
    }

    async fn generate_ready(&self, request: &SpeechRequest) -> Result<SpeechResponse, SpeechError> {
        let status = self.status().await?;
        if !status.enabled || !status.loaded {
            return Err(SpeechError::Unavailable);
        }
        let expected_input = crate::qwen_frontend::input(&request.text)?;
        let response: SpeechResponse = serde_json::from_slice(&self.exchange(Some(request)).await?)
            .map_err(|_| SpeechError::InvalidResponse)?;
        if response.request_id != request.request_id
            || response.revision_id != request.revision_id
            || response.source_text != request.text
            || response.sample_rate != SAMPLE_RATE
            || response.channels != 1
            || response.provenance.kind != SpeechProvenanceKind::AiGeneratedSpeech
            || response.provenance.provider != "local_qwen3_tts"
            || response.provenance.model != QWEN_MODEL
            || response.provenance.voice != QWEN_VOICE
            || response.provenance.prompt_version != QWEN_PROMPT
            || response.provenance.synthesis_input.as_ref() != Some(&expected_input)
            || response.usage.reserved_micro_usd != 0
            || response.usage.estimated_micro_usd != Some(0)
        {
            return Err(SpeechError::InvalidResponse);
        }
        if response.wav_base64.len() > (MAX_PCM_BYTES + 4096).div_ceil(3) * 4 {
            return Err(SpeechError::ResponseTooLarge);
        }
        let bytes = STANDARD
            .decode(&response.wav_base64)
            .map_err(|_| SpeechError::InvalidResponse)?;
        let pcm = crate::wav::decode(&bytes)?;
        if response.frames != pcm.len() as u64 / 2
            || response.sha256 != format!("{:x}", Sha256::digest(&bytes))
            || crate::wav::encode(pcm) != bytes
        {
            return Err(SpeechError::InvalidResponse);
        }
        Ok(response)
    }
}
fn http_error(error: reqwest::Error) -> SpeechError {
    if error.is_timeout() {
        SpeechError::Timeout
    } else {
        SpeechError::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        extract::State,
        http::{HeaderMap, StatusCode},
        response::{IntoResponse, Response},
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    const TOKEN: &str = "public-qwen-worker-test-token-0123456789";
    #[derive(Clone)]
    struct Mock {
        ready: bool,
        status: u16,
        delay: Duration,
        response: Value,
        posts: Arc<AtomicUsize>,
        calls: Arc<AtomicUsize>,
    }
    fn request() -> SpeechRequest {
        SpeechRequest {
            text: "Всё готово? Для каждого натурального числа существует следующее число.".into(),
            request_id: "public-qwen-1".into(),
            revision_id: Some("revision-2".into()),
        }
    }
    fn reply() -> Value {
        let request = request();
        let wav = crate::wav::encode(&[1, 0, 2, 0]);
        json!({"request_id":request.request_id,"revision_id":request.revision_id,"source_text":request.text,"wav_base64":STANDARD.encode(&wav),"sha256":format!("{:x}",Sha256::digest(&wav)),"sample_rate":SAMPLE_RATE,"channels":1,"frames":2,
            "provenance":{"kind":"ai_generated_speech","provider":"local_qwen3_tts","model":QWEN_MODEL,"voice":QWEN_VOICE,"prompt_version":QWEN_PROMPT,"validation":"Public mock only","synthesis_input":crate::qwen_frontend::input(&request.text).unwrap()},
            "usage":{"input_tokens":null,"output_tokens":null,"estimated_micro_usd":0,"reserved_micro_usd":0}})
    }
    async fn status(State(state): State<Mock>, headers: HeaderMap) -> Response {
        assert_eq!(headers["authorization"], format!("Bearer {TOKEN}"));
        state.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(state.delay).await;
        (StatusCode::from_u16(state.status).unwrap(),axum::Json(json!({"contract_version":QWEN_WORKER_SCHEMA,"enabled":state.ready,"provider":"local_qwen3_tts","model":QWEN_MODEL,"voice":QWEN_VOICE,"loaded":state.ready,"max_text_bytes":MAX_TEXT_BYTES,"max_text_chars":MAX_TEXT_CHARS}))).into_response()
    }
    async fn generate(
        State(state): State<Mock>,
        headers: HeaderMap,
        axum::Json(input): axum::Json<SpeechRequest>,
    ) -> Response {
        assert_eq!(headers["authorization"], format!("Bearer {TOKEN}"));
        assert_eq!(input.text, request().text);
        state.calls.fetch_add(1, Ordering::SeqCst);
        state.posts.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(state.delay).await;
        (
            StatusCode::from_u16(state.status).unwrap(),
            axum::Json(state.response),
        )
            .into_response()
    }
    async fn start(
        ready: bool,
        http_status: u16,
        response: Value,
        delay: Duration,
    ) -> (SpeechService, Mock, tokio::task::JoinHandle<()>, u16) {
        let state = Mock {
            ready,
            status: http_status,
            delay,
            response,
            posts: Arc::default(),
            calls: Arc::default(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new()
            .route("/status", get(status))
            .route("/generate", post(generate))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (
            SpeechService::qwen(TOKEN.into(), port).unwrap(),
            state,
            task,
            port,
        )
    }
    #[tokio::test]
    async fn speech_qwen_ready_status_and_exact_synthesis_provenance_are_preserved() {
        let (service, mock, task, _) = start(true, 200, reply(), Duration::ZERO).await;
        assert!(!service.status().enabled);
        assert!(service.checked_status().await.enabled);
        let response = service.generate(&request()).await.unwrap();
        response
            .provenance
            .synthesis_input
            .as_ref()
            .unwrap()
            .validate_for(&response.source_text)
            .unwrap();
        assert_eq!(response.source_text, request().text);
        assert_eq!(mock.posts.load(Ordering::SeqCst), 1);
        assert_eq!(response.usage.reserved_micro_usd, 0);
        task.abort();
    }
    #[tokio::test]
    async fn speech_qwen_cold_worker_is_not_enabled_and_never_generates() {
        let (service, mock, task, _) = start(false, 200, reply(), Duration::ZERO).await;
        let status = service.checked_status().await;
        assert!(!status.enabled);
        assert_eq!(status.remaining_reservations, 0);
        assert!(matches!(
            service.generate(&request()).await,
            Err(SpeechError::Unavailable)
        ));
        assert_eq!(mock.posts.load(Ordering::SeqCst), 0);
        task.abort();
    }
    #[tokio::test]
    async fn speech_qwen_isolated_stress_fails_before_network_or_allowance() {
        let (service, mock, task, _) = start(true, 200, reply(), Duration::ZERO).await;
        let mut input = request();
        input.text = "Это за́мок.".into();
        assert!(matches!(
            service.generate(&input).await,
            Err(SpeechError::UnsupportedStress)
        ));
        assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
        assert_eq!(service.checked_status().await.remaining_reservations, 40);
        task.abort();
    }
    #[tokio::test]
    async fn speech_qwen_rejects_false_binding_or_synthesis_metadata() {
        for change in ["source", "transform", "hash", "missing"] {
            let mut response = reply();
            match change {
                "source" => response["source_text"] = json!("Другой текст"),
                "transform" => {
                    response["provenance"]["synthesis_input"]["synthesis_text"] =
                        json!("Все готово?")
                }
                "hash" => response["sha256"] = json!("0".repeat(64)),
                "missing" => response["provenance"]
                    .as_object_mut()
                    .unwrap()
                    .remove("synthesis_input")
                    .map(|_| ())
                    .unwrap(),
                _ => unreachable!(),
            }
            let (service, _, task, _) = start(true, 200, response, Duration::ZERO).await;
            assert!(
                matches!(
                    service.generate(&request()).await,
                    Err(SpeechError::InvalidResponse)
                ),
                "{change}"
            );
            task.abort();
        }
    }
    #[tokio::test]
    async fn speech_qwen_upstream_auth_rate_and_redirect_errors_are_bounded() {
        for code in [401, 403, 429, 302] {
            let (service, mock, task, _) = start(true, code, reply(), Duration::ZERO).await;
            let error = service.generate(&request()).await.unwrap_err();
            assert!(match code {
                401 | 403 => matches!(error, SpeechError::Authentication),
                429 => matches!(error, SpeechError::RateLimited),
                _ => matches!(error, SpeechError::Unavailable),
            });
            assert!(!error.to_string().contains(TOKEN));
            assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
            task.abort();
        }
    }
    #[tokio::test]
    async fn speech_qwen_worker_timeout_has_no_retry() {
        let (_, mock, task, port) = start(true, 200, reply(), Duration::from_millis(200)).await;
        let mut client = QwenLocal::new(TOKEN.into(), port).unwrap();
        client.client = Client::builder()
            .timeout(Duration::from_millis(25))
            .no_proxy()
            .build()
            .unwrap();
        // Exercise POST transport directly: status() has its own explicit deadline.
        assert!(matches!(
            client.exchange(Some(&request())).await,
            Err(SpeechError::Timeout)
        ));
        assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
        task.abort();
    }
}
