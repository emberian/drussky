pub mod assets;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use rufren_core::*;
use rufren_speech::{
    RECOGNITION_MAX_REQUEST_BYTES, RecognitionCancellation, RecognitionError, RecognitionRequest,
    RecognitionResponse, RecognitionService, RecognitionStatus, SpeechError, SpeechRequest,
    SpeechResponse, SpeechService, SpeechStatus,
};
use rufren_store::Store;
use rufren_tutor::{
    TeacherDialogueRequest, TeacherDialogueResponse, TeacherRequest, TeacherResponse, TutorError,
    TutorRequest, TutorResponse, TutorService, TutorStatus,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tower_http::cors::CorsLayer;

type HttpError = (StatusCode, String);
#[derive(Clone)]
struct Relay {
    store: Arc<Mutex<Store>>,
    token: Arc<String>,
    capacity: Arc<tokio::sync::Semaphore>,
    tutor: TutorService,
    speech: SpeechService,
    recognition: RecognitionService,
    recognition_jobs: RecognitionJobs,
    assets: Option<Arc<Mutex<assets::AssetStore>>>,
    upload_capacity: Arc<tokio::sync::Semaphore>,
}
/// A disabled tutor is the default; server configuration must opt into paid requests.
pub fn router(store: Store, token: String, origins: Vec<HeaderValue>) -> Router {
    router_with_tutor(store, token, origins, TutorService::disabled())
}
pub fn router_with_tutor(
    store: Store,
    token: String,
    origins: Vec<HeaderValue>,
    tutor: TutorService,
) -> Router {
    router_with_services(store, token, origins, tutor, None)
}
pub fn router_with_services(
    store: Store,
    token: String,
    origins: Vec<HeaderValue>,
    tutor: TutorService,
    assets: Option<assets::AssetStore>,
) -> Router {
    router_with_teacher_services(
        store,
        token,
        origins,
        tutor,
        assets,
        SpeechService::disabled(),
    )
}
pub fn router_with_teacher_services(
    store: Store,
    token: String,
    origins: Vec<HeaderValue>,
    tutor: TutorService,
    assets: Option<assets::AssetStore>,
    speech: SpeechService,
) -> Router {
    router_with_ai_services(
        store,
        token,
        origins,
        tutor,
        assets,
        speech,
        RecognitionService::disabled(),
    )
}
/// Explicit services for the local AI teacher, voice models and independent ASR.
/// Existing constructors keep optional recognition disabled.
pub fn router_with_ai_services(
    store: Store,
    token: String,
    origins: Vec<HeaderValue>,
    tutor: TutorService,
    assets: Option<assets::AssetStore>,
    speech: SpeechService,
    recognition: RecognitionService,
) -> Router {
    let state = Relay {
        store: Arc::new(Mutex::new(store)),
        token: Arc::new(token),
        capacity: Arc::new(tokio::sync::Semaphore::new(8)),
        tutor,
        speech,
        recognition,
        recognition_jobs: RecognitionJobs::default(),
        assets: assets.map(|store| Arc::new(Mutex::new(store))),
        upload_capacity: Arc::new(tokio::sync::Semaphore::new(1)),
    };
    Router::new()
        .route("/v1/sync", post(exchange))
        .route("/v1/assets", get(list_assets))
        .route(
            "/v1/assets/records/{asset_id}",
            put(put_asset_record).layer(DefaultBodyLimit::max(MAX_AUDIO_RECORD_BYTES)),
        )
        .route(
            "/v1/assets/blobs/{hash}",
            get(get_blob)
                .head(head_blob)
                .put(put_blob)
                .layer(DefaultBodyLimit::max(MAX_AUDIO_BYTES)),
        )
        .route("/api/tutor/status", get(tutor_status))
        .route(
            "/api/tutor/suggest",
            post(tutor_suggest).layer(DefaultBodyLimit::max(16_384)),
        )
        .route("/api/teacher/status", get(tutor_status))
        .route(
            "/api/teacher/teach",
            post(teacher_teach).layer(DefaultBodyLimit::max(32_768)),
        )
        .route(
            "/api/teacher/dialogue",
            post(teacher_dialogue).layer(DefaultBodyLimit::max(32_768)),
        )
        .route("/api/speech/status", get(speech_status))
        .route(
            "/api/speech/generate",
            post(speech_generate).layer(DefaultBodyLimit::max(8192)),
        )
        .route("/api/speech/recognition/status", get(recognition_status))
        .route(
            "/api/speech/transcribe",
            post(recognize).layer(DefaultBodyLimit::max(RECOGNITION_MAX_REQUEST_BYTES)),
        )
        .route(
            "/api/speech/transcribe/cancel",
            post(cancel_recognition).layer(DefaultBodyLimit::max(2048)),
        )
        // Authenticate before JSON extraction or storage work, including malformed requests.
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize))
        .route(
            "/health",
            get(|| async { format!("rufren-relay/{SYNC_VERSION}") }),
        )
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::POST, Method::GET, Method::PUT, Method::HEAD])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::HeaderName::from_static("x-rufren-relay"),
                ]),
        )
        .with_state(state)
}
async fn authorize(State(state): State<Relay>, request: Request, next: Next) -> Response {
    let supplied = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if supplied.is_empty() || !bool::from(supplied.as_bytes().ct_eq(state.token.as_bytes())) {
        return (StatusCode::UNAUTHORIZED, "Sync key not accepted").into_response();
    }
    let Ok(_permit) = state.capacity.clone().try_acquire_owned() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Relay is busy; retry shortly",
        )
            .into_response();
    };
    if request.uri().path().starts_with("/v1/assets") {
        let expected = request
            .headers()
            .get("x-rufren-relay")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| uuid::Uuid::parse_str(value).ok());
        let Some(expected) = expected else {
            return (
                StatusCode::BAD_REQUEST,
                "Sync notebook facts before transferring recordings",
            )
                .into_response();
        };
        let store = state.store.clone();
        let actual = tokio::task::spawn_blocking(move || {
            store
                .lock()
                .ok()
                .and_then(|store| store.identity("relay").ok())
        })
        .await
        .ok()
        .flatten();
        if actual != Some(expected) {
            return (
                StatusCode::CONFLICT,
                "This is a different sync library. Reconnect explicitly before syncing.",
            )
                .into_response();
        }
    }
    // Teaching can use deep reasoning; private data transfers retain their shorter bound.
    let is_dialogue = request.uri().path() == "/api/teacher/dialogue";
    let deadline = match request.uri().path() {
        "/api/teacher/teach" | "/api/teacher/dialogue" | "/api/tutor/suggest" => {
            Duration::from_secs(110)
        }
        "/api/speech/transcribe" => Duration::from_secs(130),
        _ => Duration::from_secs(60),
    };
    match tokio::time::timeout(deadline, next.run(request)).await {
        Ok(response) => response,
        Err(_) if is_dialogue => {
            (StatusCode::GATEWAY_TIMEOUT, TutorError::Timeout.to_string()).into_response()
        }
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            "Request timed out; saved batches can safely be retried",
        )
            .into_response(),
    }
}
async fn exchange(
    State(state): State<Relay>,
    Json(request): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, HttpError> {
    tokio::task::spawn_blocking(move || {
        let mut store = state.store.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Storage unavailable".into(),
            )
        })?;
        store.exchange(&request).map(Json).map_err(|error| {
            if let Some(problem) = error.downcast_ref::<Error>() {
                (StatusCode::CONFLICT, problem.to_string())
            } else {
                eprintln!("sync storage error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not save sync batch".into(),
                )
            }
        })
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Sync worker stopped".into(),
        )
    })?
}
async fn tutor_status(State(state): State<Relay>) -> Json<TutorStatus> {
    Json(state.tutor.status())
}
async fn tutor_suggest(
    State(state): State<Relay>,
    Json(request): Json<TutorRequest>,
) -> Result<Json<TutorResponse>, HttpError> {
    state
        .tutor
        .suggest(&request)
        .await
        .map(Json)
        .map_err(|error| {
            let status = match error {
                TutorError::Disabled => StatusCode::SERVICE_UNAVAILABLE,
                TutorError::InvalidRequest => StatusCode::BAD_REQUEST,
                TutorError::BudgetExceeded | TutorError::RateLimited => {
                    StatusCode::TOO_MANY_REQUESTS
                }
                TutorError::Timeout => StatusCode::GATEWAY_TIMEOUT,
                TutorError::Configuration | TutorError::Authentication => {
                    StatusCode::SERVICE_UNAVAILABLE
                }
                _ => StatusCode::BAD_GATEWAY,
            };
            (status, error.to_string())
        })
}

async fn teacher_dialogue(
    State(state): State<Relay>,
    Json(request): Json<TeacherDialogueRequest>,
) -> Result<Json<TeacherDialogueResponse>, HttpError> {
    state
        .tutor
        .dialogue(&request)
        .await
        .map(Json)
        .map_err(tutor_http_error)
}

async fn teacher_teach(
    State(state): State<Relay>,
    Json(request): Json<TeacherRequest>,
) -> Result<Json<TeacherResponse>, HttpError> {
    state
        .tutor
        .teach(&request)
        .await
        .map(Json)
        .map_err(tutor_http_error)
}
fn tutor_http_error(error: TutorError) -> HttpError {
    let status = match error {
        TutorError::Disabled | TutorError::Configuration | TutorError::Authentication => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        TutorError::InvalidRequest => StatusCode::BAD_REQUEST,
        TutorError::BudgetExceeded | TutorError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        TutorError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::BAD_GATEWAY,
    };
    (status, error.to_string())
}
async fn speech_status(State(state): State<Relay>) -> Json<SpeechStatus> {
    Json(state.speech.checked_status().await)
}
async fn speech_generate(
    State(state): State<Relay>,
    Json(request): Json<SpeechRequest>,
) -> Result<Json<SpeechResponse>, HttpError> {
    state
        .speech
        .generate(&request)
        .await
        .map(Json)
        .map_err(|error| {
            let status = match error {
                SpeechError::Disabled
                | SpeechError::Configuration
                | SpeechError::Authentication => StatusCode::SERVICE_UNAVAILABLE,
                SpeechError::InvalidRequest
                | SpeechError::UnsupportedStress
                | SpeechError::AmbiguousStress
                | SpeechError::SpokenFormRequired => StatusCode::BAD_REQUEST,
                SpeechError::BudgetExceeded | SpeechError::RateLimited => {
                    StatusCode::TOO_MANY_REQUESTS
                }
                SpeechError::Timeout => StatusCode::GATEWAY_TIMEOUT,
                _ => StatusCode::BAD_GATEWAY,
            };
            (status, error.to_string())
        })
}

const RECOGNITION_CANCEL_TTL: Duration = Duration::from_secs(300);
const RECOGNITION_CANCEL_CAPACITY: usize = 128;
#[derive(Clone, Default)]
struct RecognitionJobs {
    state: Arc<Mutex<RecognitionJobState>>,
}
#[derive(Default)]
struct RecognitionJobState {
    active: Option<ActiveRecognition>,
    cancelled: VecDeque<(String, Instant)>,
}
impl RecognitionJobState {
    fn prune(&mut self, now: Instant) {
        self.cancelled
            .retain(|(_, at)| now.saturating_duration_since(*at) < RECOGNITION_CANCEL_TTL);
    }
}
struct ActiveRecognition {
    request_id: String,
    cancellation: RecognitionCancellation,
}
struct RecognitionLease {
    jobs: RecognitionJobs,
    request_id: String,
    cancellation: RecognitionCancellation,
}
impl RecognitionJobs {
    fn begin(&self, request_id: &str) -> Result<RecognitionLease, HttpError> {
        self.begin_at(request_id, Instant::now())
    }
    fn begin_at(&self, request_id: &str, now: Instant) -> Result<RecognitionLease, HttpError> {
        let mut state = self.state.lock().map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Recognition state is unavailable".into(),
            )
        })?;
        state.prune(now);
        if state.cancelled.iter().any(|(id, _)| id == request_id) {
            return Err((
                StatusCode::CONFLICT,
                "Transcription was cancelled before it started".into(),
            ));
        }
        if state.active.is_some() {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Local speech recognition is busy; retry after the current operation finishes"
                    .into(),
            ));
        }
        let cancellation = RecognitionCancellation::default();
        state.active = Some(ActiveRecognition {
            request_id: request_id.into(),
            cancellation: cancellation.clone(),
        });
        Ok(RecognitionLease {
            jobs: self.clone(),
            request_id: request_id.into(),
            cancellation,
        })
    }
    /// Records intent even before body upload/admission finishes. The bool says
    /// whether an already admitted job was signalled, not whether intent exists.
    fn cancel(&self, request_id: &str) -> Result<bool, HttpError> {
        self.cancel_at(request_id, Instant::now())
    }
    fn cancel_at(&self, request_id: &str, now: Instant) -> Result<bool, HttpError> {
        let token = {
            let mut state = self.state.lock().map_err(|_| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Recognition state is unavailable".into(),
                )
            })?;
            state.prune(now);
            if let Some((_, at)) = state.cancelled.iter_mut().find(|(id, _)| id == request_id) {
                *at = now;
            } else {
                // Never discard an accepted, unexpired cancellation to admit another.
                if state.cancelled.len() >= RECOGNITION_CANCEL_CAPACITY {
                    return Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        "Cancellation registry is full; retry cancellation shortly".into(),
                    ));
                }
                state.cancelled.push_back((request_id.into(), now));
            }
            state
                .active
                .as_ref()
                .filter(|job| job.request_id == request_id)
                .map(|job| job.cancellation.clone())
        };
        if let Some(token) = token {
            token.cancel();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
impl Drop for RecognitionLease {
    fn drop(&mut self) {
        self.cancellation.cancel();
        let mut state = self
            .jobs
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if state
            .active
            .as_ref()
            .is_some_and(|job| job.request_id == self.request_id)
        {
            state.active = None;
        }
    }
}

async fn recognition_status(State(state): State<Relay>) -> Json<RecognitionStatus> {
    Json(state.recognition.status())
}
async fn recognize(
    State(state): State<Relay>,
    Json(request): Json<RecognitionRequest>,
) -> Result<Json<RecognitionResponse>, HttpError> {
    request.validate().map_err(recognition_error)?;
    let lease = state.recognition_jobs.begin(&request.request_id)?;
    state
        .recognition
        .recognize_cancellable(&request, lease.cancellation.clone())
        .await
        .map(Json)
        .map_err(recognition_error)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RecognitionCancelRequest {
    request_id: String,
}
async fn cancel_recognition(
    State(state): State<Relay>,
    Json(request): Json<RecognitionCancelRequest>,
) -> Result<Json<serde_json::Value>, HttpError> {
    if request.request_id.is_empty()
        || request.request_id.len() > 128
        || !request
            .request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid transcription request identity".into(),
        ));
    }
    let active = state.recognition_jobs.cancel(&request.request_id)?;
    Ok(Json(serde_json::json!({"cancelled":true,"active":active})))
}
fn recognition_error(error: RecognitionError) -> HttpError {
    let status = match error {
        RecognitionError::Disabled | RecognitionError::Configuration => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        RecognitionError::InvalidRequest | RecognitionError::HashMismatch => {
            StatusCode::BAD_REQUEST
        }
        RecognitionError::UnsupportedAudio => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        RecognitionError::ResponseTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        RecognitionError::BudgetExceeded | RecognitionError::Busy => StatusCode::TOO_MANY_REQUESTS,
        RecognitionError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        RecognitionError::Cancelled => StatusCode::CONFLICT,
        _ => StatusCode::BAD_GATEWAY,
    };
    (status, error.to_string())
}

#[cfg(test)]
mod recognition_job_tests {
    use super::*;
    #[test]
    fn recognition_registry_cancels_only_exact_job_and_keeps_admission_until_completion() {
        let jobs = RecognitionJobs::default();
        let first = jobs
            .begin("first")
            .unwrap_or_else(|_| panic!("first job should start"));
        assert!(!jobs.cancel("other").unwrap());
        assert!(jobs.cancel("first").unwrap());
        assert!(matches!(
            jobs.begin("second"),
            Err((StatusCode::TOO_MANY_REQUESTS, _))
        ));
        drop(first);
        assert!(!jobs.cancel("first").unwrap());
        let second = jobs
            .begin("second")
            .unwrap_or_else(|_| panic!("completed job should release admission"));
        assert!(!jobs.cancel("first").unwrap());
        assert!(jobs.cancel("second").unwrap());
        drop(second);
    }
    #[test]
    fn recognition_registry_pre_admission_cancel_blocks_upload_race_and_retries_until_expiry() {
        let jobs = RecognitionJobs::default();
        let now = Instant::now();
        assert!(!jobs.cancel_at("upload-not-yet-admitted", now).unwrap());
        for time in [
            now,
            now + Duration::from_secs(140),
            now + RECOGNITION_CANCEL_TTL - Duration::from_secs(1),
        ] {
            assert!(
                jobs.begin_at("upload-not-yet-admitted", time)
                    .err()
                    .is_some_and(|(status, _)| status == StatusCode::CONFLICT)
            );
        }
        let after = jobs
            .begin_at("upload-not-yet-admitted", now + RECOGNITION_CANCEL_TTL)
            .unwrap_or_else(|_| panic!("expired intent should release its identity"));
        drop(after);
    }
    #[test]
    fn recognition_registry_capacity_refuses_new_intent_without_forgetting_old_one() {
        let jobs = RecognitionJobs::default();
        let now = Instant::now();
        for index in 0..RECOGNITION_CANCEL_CAPACITY {
            jobs.cancel_at(&format!("cancel-{index}"), now).unwrap();
        }
        assert!(
            jobs.cancel_at("overflow", now)
                .err()
                .is_some_and(|(status, _)| status == StatusCode::TOO_MANY_REQUESTS)
        );
        assert!(
            jobs.begin_at("cancel-0", now + Duration::from_secs(140))
                .err()
                .is_some_and(|(status, _)| status == StatusCode::CONFLICT)
        );
        assert!(
            !jobs
                .cancel_at("overflow", now + RECOGNITION_CANCEL_TTL)
                .unwrap()
        );
    }
}

fn asset_error(error: assets::AssetError) -> HttpError {
    use assets::AssetError::*;
    match error {
        Invalid => (StatusCode::BAD_REQUEST, "Invalid audio content hash".into()),
        TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "Audio exceeds the 32 MiB transfer limit".into()),
        Quota => (StatusCode::INSUFFICIENT_STORAGE, "Relay audio storage quota reached; existing recordings were kept".into()),
        Missing => (StatusCode::NOT_FOUND, "Recording bytes are not available on this relay yet. Sync them from their original device.".into()),
        Corrupt => (StatusCode::CONFLICT, "Audio integrity check failed; existing bytes were kept".into()),
        Io(error) => { eprintln!("audio storage error: {error}"); (StatusCode::INTERNAL_SERVER_ERROR, "Audio storage unavailable".into()) },
    }
}
fn configured_assets(state: &Relay) -> Result<Arc<Mutex<assets::AssetStore>>, HttpError> {
    state.assets.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Recording sync is not configured on this relay".into(),
    ))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetQuery {
    #[serde(default)]
    after: String,
}
async fn list_assets(
    State(state): State<Relay>,
    Query(query): Query<AssetQuery>,
) -> Result<Json<AssetPage>, HttpError> {
    configured_assets(&state)?;
    tokio::task::spawn_blocking(move || {
        state
            .store
            .lock()
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage unavailable".into(),
                )
            })?
            .audio_records(&query.after)
            .map(Json)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Could not read audio page".into()))
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Audio worker stopped".into(),
        )
    })?
}
async fn put_asset_record(
    State(state): State<Relay>,
    Path(asset_id): Path<String>,
    Json(record): Json<AudioRecordMetadata>,
) -> Result<Json<AudioRecordMetadata>, HttpError> {
    if asset_id != record.asset_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "Recording ID does not match its metadata".into(),
        ));
    }
    record
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let assets = configured_assets(&state)?;
    tokio::task::spawn_blocking(move || {
        let size = assets
            .lock()
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Audio storage unavailable".into(),
                )
            })?
            .verify(&record.content_hash)
            .map_err(asset_error)?;
        if size != record.byte_length {
            return Err((
                StatusCode::CONFLICT,
                "Audio byte length does not match its metadata".into(),
            ));
        }
        state
            .store
            .lock()
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage unavailable".into(),
                )
            })?
            .merge_audio_record(&record)
            .map(Json)
            .map_err(|e| (StatusCode::CONFLICT, e.to_string()))
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Audio worker stopped".into(),
        )
    })?
}
async fn put_blob(
    State(state): State<Relay>,
    Path(hash): Path<String>,
    request: Request,
) -> Result<StatusCode, HttpError> {
    if !valid_audio_hash(&hash) {
        return Err(asset_error(assets::AssetError::Invalid));
    }
    let assets = configured_assets(&state)?;
    let permit = state
        .upload_capacity
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            (
                StatusCode::TOO_MANY_REQUESTS,
                "An audio upload is already in progress; retry shortly".into(),
            )
        })?;
    let bytes = axum::body::to_bytes(request.into_body(), MAX_AUDIO_BYTES)
        .await
        .map_err(|_| asset_error(assets::AssetError::TooLarge))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        assets
            .lock()
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Audio storage unavailable".into(),
                )
            })?
            .put(&hash, &bytes)
            .map_err(asset_error)?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Audio worker stopped".into(),
        )
    })?
}
async fn head_blob(
    State(state): State<Relay>,
    Path(hash): Path<String>,
) -> Result<Response, HttpError> {
    let assets = configured_assets(&state)?;
    tokio::task::spawn_blocking(move || {
        let size = assets
            .lock()
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Audio storage unavailable".into(),
                )
            })?
            .verify(&hash)
            .map_err(asset_error)?;
        Ok((
            [
                (axum::http::header::CONTENT_LENGTH, size.to_string()),
                (axum::http::header::ETAG, format!("\"{hash}\"")),
                (axum::http::header::CACHE_CONTROL, "no-store".into()),
            ],
            (),
        )
            .into_response())
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Audio worker stopped".into(),
        )
    })?
}
async fn get_blob(
    State(state): State<Relay>,
    Path(hash): Path<String>,
) -> Result<Response, HttpError> {
    let assets = configured_assets(&state)?;
    tokio::task::spawn_blocking(move || {
        let bytes = assets
            .lock()
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Audio storage unavailable".into(),
                )
            })?
            .read(&hash)
            .map_err(asset_error)?;
        Ok((
            [
                (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
                (axum::http::header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response())
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Audio worker stopped".into(),
        )
    })?
}
