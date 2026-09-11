use std::usize;

// tecton-web/src/routes/search.rs
use crate::state::{
    AppState, IdResponse, SearchKeywordRequest, SearchRequest, SearchResponse, SearchSternRequest,
    SearchTreeRequest, VectorSearchRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tecton_core::{error::CoreError, search::SearchParams};
use tracing::instrument;
use utoipa::{
    IntoParams,
    // ToSchema
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(stemming))
        .routes(routes!(fulltext))
        .routes(routes!(search_vector))
        .routes(routes!(by_tree))
        .routes(routes!(by_keywords))
        .routes(routes!(by_name))
        .routes(routes!(by_id))
    //.with_state(state)
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/stemming",
    description = "Word Stem (stemming) search of the text field via query for the specified language.",
    responses(
        (status = OK, body=SearchResponse),
        (status =NOT_FOUND , description="Not active"),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn stemming(
    State(state): State<AppState>,
    Json(req): Json<SearchSternRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    //let index = state.index.read().await;
    let index = state.index;
    let params: SearchParams = req.into();

    let results = index.search_stern(&params).await.map_err(|e| {
        tracing::error!("Stemming search failed: {}", e);
        match e {
            CoreError::IndexDisabled(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(SearchResponse {
        total: results.len(),
        limit: params.limit,
        offset: params.offset,
        results,
    }))
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/fulltext",
    description = "Fulltext search of the text field via query.",
    responses(
        (status = OK, body=SearchResponse),
        (status =NOT_FOUND , description="Not active"),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn fulltext(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    //let index = state.index.read().await;
    let index = state.index;
    let params: SearchParams = req.into();
    let results = index.search_fulltext(&params).await.map_err(|e| {
        tracing::error!("Fulltext search failed: {}", e);
        match e {
            CoreError::IndexDisabled(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(SearchResponse {
        total: results.len(),
        limit: params.limit,
        offset: params.offset,
        results,
    }))
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/vector",
    responses(
        (status = OK, body=SearchResponse),
        (status =NOT_FOUND , description="Not active"),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn search_vector(
    State(state): State<AppState>,
    Json(req): Json<VectorSearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    //let index = state.index.read().await;
    let index = state.index;
    let params: SearchParams = req.into();

    let results = index.search_vector(&params).await.map_err(|e| {
        tracing::error!("Vector search failed: {}", e);
        match e {
            CoreError::IndexDisabled(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(SearchResponse {
        total: results.len(),
        limit: params.limit,
        offset: params.offset,
        results,
    }))
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/tree",
    description = "Search by tree e.g. /root/first. Will respond with all documents of /root/first and possible children.",
    responses(
        (status = OK, body=SearchResponse),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn by_tree(
    State(state): State<AppState>,
    Json(req): Json<SearchTreeRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    //let index = state.index.read().await;
    let index = state.index;
    let params: SearchParams = req.into();
    let results = index.search_tree(&params).await.map_err(|e| {
        tracing::error!("Tree search failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(SearchResponse {
        total: results.len(),
        limit: params.limit,
        offset: params.offset,
        results,
    }))
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    path = "/keywords",
    description = "Search by keywords.",
    responses(
        (status = OK, body=SearchResponse),
        (status = NOT_FOUND , description="Invalid language argument"),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn by_keywords(
    State(state): State<AppState>,
    Json(req): Json<SearchKeywordRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    //let index = state.index.read().await;
    let index = state.index;
    let params: SearchParams = req.into();
    let results = index.search_keywords(&params).await.map_err(|e| {
        tracing::error!("Keyword search failed: {}", e);
        match e {
            CoreError::InvalidArgument(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(SearchResponse {
        total: results.len(),
        limit: params.limit,
        offset: params.offset,
        results,
    }))
}

#[derive(Deserialize, IntoParams, Debug)]
#[into_params(style = Form, parameter_in = Query)]
pub struct LimitQuery {
    limit: Option<usize>,
}

#[instrument(skip(state))]
#[utoipa::path(
    get,
    path = "/document/{name}",
    description = "Search all blocks for a named document.",
    params(
        LimitQuery
    ),
    responses(
        (status = OK, body=SearchResponse),
        (status = NOT_FOUND , description="Invalid language argument"),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
    query: Query<LimitQuery>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let index = state.index;
    let limit: usize = match query.limit {
        Some(l) => l,
        _ => 100,
    };

    let results = index.search_by_document(&name, &limit).await.map_err(|e| {
        tracing::error!("Document search failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(SearchResponse {
        total: results.len(),
        limit,
        offset: 0,
        results,
    }))
}

#[instrument(skip(state))]
#[utoipa::path(
    get,
    path = "/id/{id}",
    description = "Return the block with the id or nothing.",
    responses(
        (status = OK, body=SearchResponse),
        (status = NOT_FOUND , description="Invalid language argument"),
        (status = INTERNAL_SERVER_ERROR)
    )
)]
pub async fn by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
    //query: Query<LimitQuery>,
) -> Result<Json<IdResponse>, StatusCode> {
    let index = state.index;

    let results = index.search_by_id(&id).await.map_err(|e| {
        tracing::error!("Id search failed: {}", e);
        match e {
            CoreError::InvalidArgument(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(IdResponse {
        total: results.len(),
        results,
    }))
}
