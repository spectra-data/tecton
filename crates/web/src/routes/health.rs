// tecton-web/src/routes/health.rs
use crate::state::{AppState, HealthResponse};
use axum::{
    extract::{MatchedPath, Request, State},
    middleware::Next,
    response::{IntoResponse, Json},
};

//use axum::extract::State;
use axum::{
    Router,
    routing::get, //, routing::get
};
use std::time::Instant;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_fun))
        .route("/ready", get(read_fun))
        .with_state(state)
}

pub async fn health_fun(State(state): State<AppState>) -> Json<HealthResponse> {
    //let index = state.index.read().await;
    let index = state.index;
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        index_docs: index.num_docs(),
    })
}
pub async fn read_fun(State(state): State<AppState>) -> Json<HealthResponse> {
    let index = state.index;
    // Could add more readiness checks here (disk space, etc.)
    Json(HealthResponse {
        status: "ready",
        version: env!("CARGO_PKG_VERSION"),
        index_docs: index.num_docs(),
    })
}

/// Middleware to record some common HTTP metrics
/// Generic over B to allow for arbitrary body types (eg Vec<u8>, Streams, a deserialized thing, etc)
/// Someday tower-http might provide a metrics middleware: https://github.com/tower-rs/tower-http/issues/57
pub async fn track_metrics(req: Request, next: Next) -> impl IntoResponse {
    let start = Instant::now();

    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };

    let method = req.method().clone();

    // Run the rest of the request handling first, so we can measure it and get response
    // codes.
    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = vec![
        ("method".to_string(), method.to_string()),
        ("path".to_string(), path),
        ("status".to_string(), status),
    ];
    tecton_core::metrics::http_requests_global(labels, latency);

    response
}
