// tecton-web/src/routes/update.rs
/*use crate::state::{AppState, KeywordAction, UpdateKeywordsRequest};
use axum::{extract::State, http::StatusCode, response::Json};
use tracing::instrument;

use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router(State(state): State<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(update_keywords))
        .with_state(state)
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
    let mut index = state.index.write().await;

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
*/
