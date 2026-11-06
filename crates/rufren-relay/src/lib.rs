use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    routing::{get, post},
};
use rufren_core::*;
use rufren_store::Store;
use std::sync::{Arc, Mutex};
use subtle::ConstantTimeEq;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct Relay {
    store: Arc<Mutex<Store>>,
    token: Arc<String>,
}
pub fn router(store: Store, token: String, origins: Vec<HeaderValue>) -> Router {
    Router::new()
        .route("/health", get(|| async { "rufren-relay/1" }))
        .route("/v1/sync", post(exchange))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::POST, Method::GET])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                ]),
        )
        .with_state(Relay {
            store: Arc::new(Mutex::new(store)),
            token: Arc::new(token),
        })
}
async fn exchange(
    State(state): State<Relay>,
    headers: HeaderMap,
    Json(request): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, (StatusCode, String)> {
    let supplied = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !bool::from(supplied.as_bytes().ct_eq(state.token.as_bytes())) {
        return Err((StatusCode::UNAUTHORIZED, "Sync key not accepted".into()));
    }
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
