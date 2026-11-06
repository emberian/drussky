use super::*;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone)]
struct Mock {
    reply: Value,
    status: u16,
    delay: Duration,
    calls: Arc<AtomicUsize>,
    seen: Arc<Mutex<Option<Value>>>,
}
async fn mock(State(state): State<Mock>, headers: HeaderMap, body: axum::body::Bytes) -> Response {
    assert_eq!(headers["x-goog-api-key"], "test-secret");
    state.calls.fetch_add(1, Ordering::SeqCst);
    *state.seen.lock().unwrap() = Some(serde_json::from_slice(&body).unwrap());
    tokio::time::sleep(state.delay).await;
    Response::builder()
        .status(StatusCode::from_u16(state.status).unwrap())
        .header("Content-Type", "application/json")
        .body(Body::from_stream(futures_util::stream::iter([Ok::<
            _,
            std::io::Error,
        >(
            state.reply.to_string(),
        )])))
        .unwrap()
}
async fn service(
    reply: Value,
    status: u16,
    delay: Duration,
    timeout: Duration,
    budget: u64,
) -> (SpeechService, Mock, tokio::task::JoinHandle<()>) {
    let state = Mock {
        reply,
        status,
        delay,
        calls: Arc::default(),
        seen: Arc::default(),
    };
    let app = Router::new()
        .route("/v1beta/interactions", post(mock))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "http://{}/v1beta/interactions",
        listener.local_addr().unwrap()
    );
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (
        SpeechService::configured("test-secret".into(), budget, endpoint, timeout).unwrap(),
        state,
        task,
    )
}
fn request() -> SpeechRequest {
    SpeechRequest {
        text: "Привет! Как дела?".into(),
        request_id: "request-1".into(),
        revision_id: Some("revision-2".into()),
    }
}
fn success() -> Value {
    let pcm: Vec<_> = (0..2400i16)
        .flat_map(|n| (n % 400 - 200).to_le_bytes())
        .collect();
    json!({"status":"completed","model":MODEL,"steps":[{"type":"model_output","content":[{"type":"audio","data":STANDARD.encode(crate::wav::encode(&pcm)),"mime_type":"audio/wav","sample_rate":SAMPLE_RATE,"channels":1}]}],"usage":{"total_input_tokens":20,"total_output_tokens":3,"total_thought_tokens":0,"total_tool_use_tokens":0}})
}

#[tokio::test]
async fn speech_success_preserves_exact_bindings_and_hash_verified_wav() {
    let (service, mock, task) = service(
        success(),
        200,
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    let result = service.generate(&request()).await.unwrap();
    assert_eq!(result.request_id, "request-1");
    assert_eq!(result.revision_id, Some("revision-2".into()));
    assert_eq!(result.source_text, request().text);
    assert_eq!(
        result.provenance.kind,
        SpeechProvenanceKind::AiGeneratedSpeech
    );
    assert_eq!(result.provenance.model, MODEL);
    assert_eq!(result.provenance.voice, VOICE);
    let wav = STANDARD.decode(result.wav_base64).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[36..40], b"data");
    assert_eq!(wav.len(), 44 + 4800);
    assert_eq!(result.frames, 2400);
    assert_eq!(result.sha256, format!("{:x}", Sha256::digest(&wav)));
    assert_eq!(result.usage.estimated_micro_usd, Some(80));
    let seen = mock.seen.lock().unwrap();
    let payload = seen.as_ref().unwrap();
    assert_eq!(payload["store"], false);
    assert_eq!(payload["model"], MODEL);
    assert_eq!(
        payload["generation_config"]["max_output_tokens"],
        MAX_OUTPUT_TOKENS
    );
    assert_eq!(payload["input"][0]["content"][0]["text"], request().text);
    assert_eq!(payload["response_format"]["mime_type"], "audio/wav");
    assert!(payload.get("tools").is_none());
    assert!(!payload.to_string().contains("request-1"));
    assert!(!payload.to_string().contains("revision-2"));
    task.abort();
}

#[tokio::test]
async fn speech_invalid_and_oversized_uploads_never_reach_network() {
    let (service, mock, task) = service(
        success(),
        200,
        Duration::ZERO,
        Duration::from_secs(2),
        DEFAULT_SESSION_BUDGET_MICRO_USD,
    )
    .await;
    for text in [
        String::new(),
        "x".repeat(MAX_TEXT_CHARS + 1),
        "я".repeat(MAX_TEXT_BYTES),
        "<laugh>Привет".into(),
        "[[slnc 999999999]]Привет".into(),
        "123 ...".into(),
    ] {
        let mut input = request();
        input.text = text;
        assert!(matches!(
            service.generate(&input).await,
            Err(SpeechError::InvalidRequest)
        ));
    }
    let mut input = request();
    input.request_id = "bad/path".into();
    assert!(matches!(
        service.generate(&input).await,
        Err(SpeechError::InvalidRequest)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        service.status().remaining_budget_micro_usd,
        DEFAULT_SESSION_BUDGET_MICRO_USD
    );
    task.abort();
}

#[tokio::test]
async fn speech_auth_rate_and_upstream_errors_are_redacted_without_retry() {
    for status in [401, 403, 429, 500, 302] {
        let (service, mock, task) = service(
            json!({"error":"secret private request test-secret"}),
            status,
            Duration::ZERO,
            Duration::from_secs(2),
            RESERVATION_MICRO_USD,
        )
        .await;
        let error = service.generate(&request()).await.unwrap_err();
        assert!(match status {
            401 | 403 => matches!(error, SpeechError::Authentication),
            429 => matches!(error, SpeechError::RateLimited),
            _ => matches!(error, SpeechError::Unavailable),
        });
        assert!(!error.to_string().contains("test-secret"));
        assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
        assert_eq!(service.status().remaining_reservations, 0);
        task.abort();
    }
}

#[tokio::test]
async fn speech_invalid_pcm_and_response_contract_are_rejected() {
    for kind in [
        "empty",
        "odd",
        "base64",
        "silent",
        "mime",
        "rate",
        "channels",
        "text",
        "multiple",
        "truncated",
        "model",
        "tokens",
        "tool",
        "remote",
    ] {
        let mut reply = success();
        match kind {
            "empty" => reply["steps"][0]["content"][0]["data"] = json!(""),
            "odd" => reply["steps"][0]["content"][0]["data"] = json!(STANDARD.encode([1, 2, 3])),
            "base64" => reply["steps"][0]["content"][0]["data"] = json!("not base64!!"),
            "silent" => {
                reply["steps"][0]["content"][0]["data"] =
                    json!(STANDARD.encode(crate::wav::encode(&[0, 0])))
            }
            "mime" => reply["steps"][0]["content"][0]["mime_type"] = json!("audio/l16"),
            "rate" => reply["steps"][0]["content"][0]["sample_rate"] = json!(16000),
            "channels" => reply["steps"][0]["content"][0]["channels"] = json!(2),
            "text" => reply["steps"][0]["content"][0]["type"] = json!("text"),
            "multiple" => {
                let extra = reply["steps"][0]["content"][0].clone();
                reply["steps"][0]["content"]
                    .as_array_mut()
                    .unwrap()
                    .push(extra);
            }
            "truncated" => reply["status"] = json!("incomplete"),
            "model" => reply["model"] = json!("unknown-model"),
            "tokens" => reply["usage"]["total_output_tokens"] = json!(MAX_OUTPUT_TOKENS + 1),
            "tool" => reply["steps"][0]["type"] = json!("function_call"),
            "remote" => {
                reply["steps"][0]["content"][0]["uri"] = json!("https://untrusted.invalid/audio")
            }
            _ => unreachable!(),
        }
        let (service, _, task) = service(
            reply,
            200,
            Duration::ZERO,
            Duration::from_secs(2),
            DEFAULT_SESSION_BUDGET_MICRO_USD,
        )
        .await;
        assert!(
            matches!(
                service.generate(&request()).await,
                Err(SpeechError::InvalidResponse)
            ),
            "{kind}"
        );
        task.abort();
    }
}

#[tokio::test]
async fn speech_duration_and_chunked_body_bounds_are_enforced() {
    let mut reply = success();
    reply["steps"][0]["content"][0]["data"] =
        json!(STANDARD.encode(crate::wav::encode(&vec![1; MAX_PCM_BYTES + 2])));
    for reply in [reply, json!({"padding":"x".repeat(MAX_RESPONSE_BYTES+1)})] {
        let (service, _, task) = service(
            reply,
            200,
            Duration::ZERO,
            Duration::from_secs(2),
            DEFAULT_SESSION_BUDGET_MICRO_USD,
        )
        .await;
        assert!(matches!(
            service.generate(&request()).await,
            Err(SpeechError::ResponseTooLarge)
        ));
        task.abort();
    }
}

#[tokio::test]
async fn speech_timeout_retains_reservation_and_never_retries() {
    let (service, mock, task) = service(
        success(),
        200,
        Duration::from_millis(200),
        Duration::from_millis(30),
        RESERVATION_MICRO_USD,
    )
    .await;
    assert!(matches!(
        service.generate(&request()).await,
        Err(SpeechError::Timeout)
    ));
    assert!(matches!(
        service.generate(&request()).await,
        Err(SpeechError::BudgetExceeded)
    ));
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn speech_concurrency_and_cancellation_keep_budget_conservative() {
    let (service, mock, task) = service(
        success(),
        200,
        Duration::from_millis(500),
        Duration::from_secs(2),
        RESERVATION_MICRO_USD * 2,
    )
    .await;
    let worker = service.clone();
    let request = request();
    let running = tokio::spawn(async move { worker.generate(&request).await });
    for _ in 0..40 {
        if mock.calls.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(matches!(
        service.generate(&super::tests::request()).await,
        Err(SpeechError::RateLimited)
    ));
    assert_eq!(
        service.status().remaining_budget_micro_usd,
        RESERVATION_MICRO_USD
    );
    running.abort();
    let _ = running.await;
    assert_eq!(
        service.status().remaining_budget_micro_usd,
        RESERVATION_MICRO_USD
    );
    // Cancellation releases the single-operation gate, but does not refund its reservation.
    assert!(service.generate(&super::tests::request()).await.is_ok());
    assert_eq!(service.status().remaining_reservations, 0);
    task.abort();
}

#[tokio::test]
async fn speech_disabled_is_explicit_and_credentials_are_validated() {
    let service = SpeechService::disabled();
    assert!(!service.status().enabled);
    assert!(matches!(
        service.generate(&request()).await,
        Err(SpeechError::Disabled)
    ));
    assert!(matches!(
        SpeechService::gemini(String::new(), RESERVATION_MICRO_USD),
        Err(SpeechError::Configuration)
    ));
    assert!(matches!(
        SpeechService::gemini("bad\nheader".into(), RESERVATION_MICRO_USD),
        Err(SpeechError::Configuration)
    ));
}
