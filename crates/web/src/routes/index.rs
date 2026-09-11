// tecton-web/src/routes/index.rs
use crate::state::{
    AppState, IndexResponse, IndexTextRequest, KeywordAction, UpdateKeywordsRequest,
};
use axum::{extract::State, http::StatusCode, response::Json};
//use chrono::{DateTime, Utc};
use tracing::instrument;

use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(index_text))
        //.routes(routes!(index_markdown))
        .routes(routes!(update_keywords))
    //.with_state(state)
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/text",
    responses(
        (status = OK, body=IndexResponse),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn index_text(
    State(state): State<AppState>,
    Json(req): Json<IndexTextRequest>,
) -> Result<Json<IndexResponse>, StatusCode> {
    //let index = state.index.write().await;
    let index = state.index;
    let id = index
        .add_text_block(
            req.id,
            req.document_name,
            req.date,
            req.language,
            req.tree.unwrap_or_default(),
            req.text,
            //None, // vector - would need embedding service
            req.keywords.unwrap_or_default(),
        )
        .map_err(|e| {
            tracing::error!("Failed to index text: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    index.commit().map_err(|e| {
        tracing::error!("Failed to commit: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(IndexResponse { id, indexed: 1 }))
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/keywords",
    responses(
        (status = OK, body=Json::Object),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn update_keywords(
    State(state): State<AppState>,
    Json(req): Json<UpdateKeywordsRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    //let index = state.index.write().await;
    let index = state.index;
    let replace = matches!(req.action, KeywordAction::Replace);

    index
        .update_keywords(&req.id, req.keywords, replace)
        .map_err(|e| {
            tracing::error!("Failed to update keywords: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    index.commit().map_err(|e| {
        tracing::error!("Failed to commit: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "status": "updated",
        "id": req.id
    })))
}
