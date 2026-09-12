// tecton-web/src/state.rs
use chrono::{DateTime, Utc};
use std::sync::Arc;

use tecton_core::{
    //MarkdownBlock,
    TectonIndex,
    config::CoreConfig,
    search::{SearchParams, SearchResult},
};
//use ulid::Ulid;
use utoipa::ToSchema;

#[derive(Clone)]
pub struct AppState {
    //pub index: Arc<RwLock<TxtBlockIndex>>,
    pub index: Arc<TectonIndex>,
    pub config: CoreConfig,
}

impl AppState {
    pub async fn new(config: CoreConfig) -> anyhow::Result<Self> {
        let index = TectonIndex::new(config.clone())?;
        Ok(Self {
            index: Arc::new(index),
            config,
        })
    }
}

// Request/Response types
#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct IndexTextRequest {
    pub text: String,
    pub language: String,
    pub document_name: String,
    pub date: Option<DateTime<Utc>>,
    pub tree: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub id: Option<String>,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct IndexMarkdownRequest {
    pub markdown: String,
    pub language: String,
    pub document_name: String,
    pub date: Option<DateTime<Utc>>,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub language: Option<String>,
    pub name: Option<String>,
    pub min_date: Option<DateTime<Utc>>,
    pub max_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub id: Option<String>,
    pub tree: Option<String>,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct SearchSternRequest {
    pub query: String,
    pub language: Option<String>,
    pub name: Option<String>,
    pub min_date: Option<DateTime<Utc>>,
    pub max_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub id: Option<String>,
    pub tree: Option<String>,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct SearchTreeRequest {
    pub language: Option<String>,
    pub name: Option<String>,
    pub min_date: Option<DateTime<Utc>>,
    pub max_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub tree: String,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct SearchKeywordRequest {
    pub keys: Vec<String>,
    pub language: Option<String>,
    pub name: Option<String>,
    pub min_date: Option<DateTime<Utc>>,
    pub max_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub tree: Option<String>,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct VectorSearchRequest {
    pub query: String,
    pub language: Option<String>,
    pub name: Option<String>,
    pub min_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub id: Option<String>,
    pub tree: Option<String>,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
pub struct UpdateKeywordsRequest {
    pub id: String,
    pub keywords: Vec<String>,
    pub action: KeywordAction,
}

#[derive(serde::Deserialize, Debug, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum KeywordAction {
    Add,
    Replace,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct IdResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct IndexResponse {
    pub id: String,
    pub indexed: usize,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub index_docs: u64,
}

impl From<SearchParams> for SearchRequest {
    fn from(params: SearchParams) -> Self {
        Self {
            query: params.query_text,
            language: params.lang,
            name: params.name,
            min_date: params.min_date,
            max_date: params.max_date,
            limit: Some(params.limit),
            offset: Some(params.offset),
            id: params.id,
            tree: params.tree,
        }
    }
}

impl From<SearchRequest> for SearchParams {
    fn from(req: SearchRequest) -> Self {
        Self {
            query_text: req.query,
            lang: req.language,
            name: req.name,
            min_date: req.min_date,
            max_date: req.max_date,
            limit: req.limit.unwrap_or(10).min(100),
            offset: req.offset.unwrap_or(0),
            id: req.id,
            tree: req.tree,
            keys: None,
        }
    }
}

impl From<SearchSternRequest> for SearchParams {
    fn from(req: SearchSternRequest) -> Self {
        Self {
            query_text: Some(req.query),
            lang: req.language,
            name: req.name,
            min_date: req.min_date,
            max_date: req.max_date,
            limit: req.limit.unwrap_or(10).min(100),
            offset: req.offset.unwrap_or(0),
            id: req.id,
            tree: req.tree,
            keys: None,
        }
    }
}

impl From<SearchTreeRequest> for SearchParams {
    fn from(req: SearchTreeRequest) -> Self {
        Self {
            query_text: None,
            lang: req.language,
            name: req.name,
            min_date: req.min_date,
            max_date: req.max_date,
            limit: req.limit.unwrap_or(10).min(100),
            offset: req.offset.unwrap_or(0),
            id: None,
            tree: Some(req.tree),
            keys: None,
        }
    }
}

impl From<SearchKeywordRequest> for SearchParams {
    fn from(req: SearchKeywordRequest) -> Self {
        Self {
            query_text: None,
            lang: req.language,
            name: req.name,
            min_date: req.min_date,
            max_date: req.max_date,
            limit: req.limit.unwrap_or(10).min(100),
            offset: req.offset.unwrap_or(0),
            id: None,
            tree: req.tree,
            keys: Some(req.keys),
        }
    }
}

impl From<VectorSearchRequest> for SearchParams {
    fn from(req: VectorSearchRequest) -> Self {
        Self {
            query_text: Some(req.query.clone()),
            lang: req.language,
            name: req.name,
            min_date: req.min_date,
            max_date: None,
            limit: req.limit.unwrap_or(10).min(100),
            offset: req.offset.unwrap_or(0),
            id: None,
            tree: req.tree,
            keys: None,
        }
    }
}
