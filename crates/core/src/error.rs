// tecton-core/src/error.rs
use tantivy::TantivyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] TantivyError),

    #[error("HNSW error: {0}")]
    Hnsw(String), // hnsw-rs 0.3 errors are often strings or custom

    #[error("Index not active: {0}")]
    IndexDisabled(String),

    #[error("Tokenizer error: {0}")]
    Tokenizer(#[from] tokenizers::Error),

    #[error("HuggingFace Hub error: {0}")]
    HfHub(#[from] hf_hub::HFError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Index not found at path: {0}")]
    IndexNotFound(String),

    #[error("Document not found with id: {0}")]
    DocumentNotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Markdown parsing error: {0}")]
    Markdown(String),

    #[error("Index is locked, another writer may be active")]
    Locked,

    #[error("Model not initialized. Call Indexer::new or Searcher::new first.")]
    ModelNotInit,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
