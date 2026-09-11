// tecton-web/src/extractors.rs
use crate::state::AppState;
use anyhow::Result;
use axum::{
    RequestPartsExt,
    extract::{
        FromRequestParts, //    , Request
    },
    http::request::Parts,
};
use axum_extra::TypedHeader;
use axum_extra::headers::{Authorization, authorization::Bearer};
use std::sync::Arc;

// API Key extractor for authentication
#[derive(Clone)]
pub struct ApiKey(pub String);

impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get API key from header
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            return Ok(ApiKey(bearer.token().to_string()));
        }

        // Try to get from query parameter
        if let Some(query) = parts.uri.query() {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key == "api_key" {
                        return Ok(ApiKey(value.to_string()));
                    }
                }
            }
        }

        // No API key found
        Err(axum::response::Response::builder()
            .status(axum::http::StatusCode::UNAUTHORIZED)
            .body(axum::body::Body::from("Missing API key"))
            .unwrap())
    }
}

// Request ID extractor for tracing
#[derive(Clone)]
pub struct RequestId(pub String);

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get from header (set by tower-http request-id layer)
        if let Some(req_id) = parts.headers.get("x-request-id") {
            if let Ok(id) = req_id.to_str() {
                return Ok(RequestId(id.to_string()));
            }
        }

        // Generate new one
        Ok(RequestId(uuid::Uuid::new_v4().to_string()))
    }
}

// Validated pagination parameters
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    10
}

impl PaginationParams {
    pub fn new(limit: usize, offset: usize) -> Self {
        Self {
            limit: limit.min(100),
            offset,
        }
    }
}

// Date range extractor
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DateRangeParams {
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    pub max_date: Option<chrono::DateTime<chrono::Utc>>,
}

// Search query extractor with validation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SearchQueryParams {
    pub q: Option<String>,
    pub lang: Option<String>,
    pub document_name: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub date_range: DateRangeParams,
}

// App state extractor (convenience)
pub async fn get_app_state(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Arc<AppState> {
    state
}
