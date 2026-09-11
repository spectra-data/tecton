// tecton-core/src/lib.rs
pub mod config;

pub mod error;
pub mod hash_indexer;
pub mod index;
pub mod languages;
pub mod metrics;
pub mod schema;
pub mod search;

use crate::{
    config::{CoreConfig, IndexConfig},
    error::{CoreError, Result},
    index::IndexManager,
    search::{SearchParams, SearchResult, SearcherWrapper},
};

use tantivy::Searcher;
use tantivy::query::QueryParser;
use tantivy::schema::Field;

/// High-level API for the tecton core library
pub struct TectonIndex {
    index_manager: IndexManager,
    searcher: SearcherWrapper,
}

impl TectonIndex {
    /// Create or open an index with the given configuration
    pub fn new(config: CoreConfig) -> Result<Self> {
        let index_manager = IndexManager::open_or_create(&config.index)?;
        let searcher = SearcherWrapper::new(
            index_manager.schema(),
            index_manager.tokenizer().clone(),
            config.index.enable_steam,
            config.index.languages,
        )?;
        Ok(Self {
            index_manager,
            searcher,
        })
    }

    /// Create a new index (fails if exists)
    pub fn create(config: CoreConfig) -> Result<Self> {
        let index_manager = IndexManager::create(&config.index)?;
        let searcher = SearcherWrapper::new(
            index_manager.schema(),
            index_manager.tokenizer().clone(),
            config.index.enable_steam,
            config.index.languages,
        )?;

        Ok(Self {
            index_manager,
            searcher,
        })
    }

    /// Open existing index
    pub fn open(config: CoreConfig) -> Result<Self> {
        let index_manager = IndexManager::open(&config.index)?;
        let searcher = SearcherWrapper::new(
            index_manager.schema(),
            index_manager.tokenizer().clone(),
            config.index.enable_steam,
            config.index.languages,
        )?;

        Ok(Self {
            index_manager,
            searcher,
        })
    }

    /// Add a single text block
    pub fn add_text_block(
        &self,
        id: Option<String>,
        document_name: String,
        date: Option<chrono::DateTime<chrono::Utc>>,
        lang: String,
        tree: String,
        text: String,
        //vector: Option<Vec<f32>>,
        keywords: Vec<String>,
    ) -> Result<String> {
        self.index_manager.add_document(
            id,
            document_name,
            date.unwrap_or_else(chrono::Utc::now),
            lang,
            tree,
            text,
            keywords,
        )
    }

    /// Commit pending changes
    pub fn commit(&self) -> Result<()> {
        self.index_manager.commit()
    }

    /// Update keywords for a document
    pub fn update_keywords(&self, id: &str, keywords: Vec<String>, replace: bool) -> Result<()> {
        self.index_manager.update_keywords(id, keywords, replace)
    }

    pub fn delete_document(&self, id: &str) -> Result<()> {
        self.index_manager.delete_document(id)
    }

    /// get searcher from the index reader
    pub fn get_searcher(&self) -> Searcher {
        self.index_manager.searcher()
    }

    pub fn get_query_parser(&self, fields: Vec<Field>) -> QueryParser {
        self.index_manager.get_query_parser(fields)
    }

    /// Compact the index
    pub fn compact(&self) -> Result<()> {
        self.index_manager.compact()
    }

    /// Stemming search
    pub async fn search_stern(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        if !self.index_manager.config.enable_steam {
            return Err(CoreError::IndexDisabled("Stemming".to_string()));
        }
        self.searcher
            .search_stern(params, &self.get_searcher())
            .await
    }

    /// Fulltext search
    pub async fn search_fulltext(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.searcher
            .search_fulltext(params, &self.get_searcher())
            .await
    }

    /// Vector search
    pub async fn search_vector(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        if !self.index_manager.config.enable_hnsw {
            return Err(CoreError::IndexDisabled("Vector".to_string()));
        }
        self.searcher
            .search_vector(
                params,
                &self.get_searcher(),
                self.index_manager.hnsw_index().unwrap(),
                &self.config().hnsw.ef_search,
            )
            .await
    }

    /// Tree search
    pub async fn search_tree(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.searcher
            .search_tree(params, &self.get_searcher())
            .await
    }

    /// Keyword search
    pub async fn search_keywords(&self, params: &SearchParams) -> Result<Vec<SearchResult>> {
        self.searcher
            .search_keywords(params, &self.get_searcher())
            .await
    }

    /// Search by document name
    pub async fn search_by_document(
        &self,
        document_name: &str,
        limit: &usize,
    ) -> Result<Vec<SearchResult>> {
        self.searcher
            .search_by_document(document_name, limit, &self.get_searcher())
            .await
    }

    /// Search by document name
    pub async fn search_by_id(&self, id: &str) -> Result<Vec<SearchResult>> {
        self.searcher.search_by_id(id, &self.get_searcher()).await
    }

    /// Get number of documents
    pub fn num_docs(&self) -> u64 {
        self.index_manager.num_docs()
    }

    /// Get metrics handle for Prometheus endpoint
    //pub fn metrics_handle(&self) -> Option<metrics_exporter_prometheus::PrometheusHandle> {
    //    get_metrics_handle()
    //}

    /// Get index config
    pub fn config(&self) -> &IndexConfig {
        &self.index_manager.config()
    }
}

// Re-export commonly used types
pub use chrono::DateTime;
//use tantivy::Index;
pub use ulid::Ulid;

pub use metrics::TB_INDEX_NAME;
