// tecton-core/src/config.rs
use num_cpus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Root directory for all index data
    pub index_path: PathBuf,

    /// Tantivy specific subdirectory default tantivy_index
    #[serde(default = "default_tantivy_dir")]
    pub tantivy_dir: String,

    /// commit interval in ms default 5000
    #[serde(default = "default_commit_interval_ms")]
    pub commit_interval_ms: u64,
    /// ma index threads for tantivy writer default 8
    #[serde(default = "default_max_threads")]
    pub max_threads: usize,
    /// buffer size default 100_000_000
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    /// enable stem on text index default true
    #[serde(default = "default_true")]
    pub enable_steam: bool,
    /// enable hswn index (vector index) default true
    #[serde(default = "default_false")]
    pub enable_hnsw: bool,
    /// default enabled languages en,de,nl
    /// possible are : "ar", "da", "nl", "en", "fi", "fr", "de", "el", "hu", "it", "no", "pt", "ro", "ru", "es", "sv", "ta", "tr"
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    /// HNSW Index Parameters
    #[serde(default)]
    pub hnsw: HnswConfig,
}

fn default_commit_interval_ms() -> u64 {
    5000
}
fn default_max_threads() -> usize {
    num_cpus::get()
}
fn default_buffer_size() -> usize {
    100_000_000
} // 100MB
fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_tantivy_dir() -> String {
    "tantivy_index".to_string()
}

fn default_languages() -> Vec<String> {
    vec!["en".to_string()]
}

fn default_hnsw_dir() -> String {
    "hnsw_index".to_string()
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("./index"),
            tantivy_dir: default_tantivy_dir(),
            commit_interval_ms: default_commit_interval_ms(),
            max_threads: default_max_threads(),
            buffer_size: default_buffer_size(),
            enable_steam: default_true(),
            enable_hnsw: default_false(),
            languages: default_languages(),
            hnsw: HnswConfig::default(),
        }
    }
}

impl IndexConfig {
    pub fn is_language_supported(&self, lang: &str) -> bool {
        self.languages.iter().any(|l| l.as_str() == lang)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// HNSW subdirectory default 'hnsw_index'
    #[serde(default = "default_hnsw_dir")]
    pub hnsw_dir: String,

    /// index N-Grams default 2 tokens
    #[serde(default = "default_n_grams")]
    pub n_grams: usize,

    /// dimension for layer default: 1024 (dim)
    #[serde(default = "default_dim")]
    pub dimension: usize,

    /// Max connections per layer (16)
    #[serde(default = "default_m")]
    pub m: usize,

    /// Construction ef (ef_construction)
    #[serde(default = "default_ef_construction")]
    pub ef_construction: usize,

    /// Search ef (ef_search) default 64
    #[serde(default = "default_ef_search")]
    pub ef_search: usize,

    /// Fix RNG seed for reproducible index layouts default 42
    #[serde(default = "default_seed")]
    pub seed: u64,

    /// Stem (stemming) enabled: true
    #[serde(default = "default_true")]
    pub stem: bool,
}

fn default_n_grams() -> usize {
    2
}

fn default_dim() -> usize {
    1024
}

fn default_m() -> usize {
    16
}
fn default_ef_construction() -> usize {
    200
}
fn default_ef_search() -> usize {
    64
}
fn default_seed() -> u64 {
    42
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            hnsw_dir: default_hnsw_dir(),
            n_grams: default_n_grams(),
            dimension: default_dim(),
            m: default_m(),
            ef_construction: default_ef_construction(),
            ef_search: default_ef_search(),
            seed: default_seed(),
            stem: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_limit")]
    pub default_limit: usize,
    #[serde(default = "default_max_limit")]
    pub max_limit: usize,
}

fn default_limit() -> usize {
    10
}
fn default_max_limit() -> usize {
    100
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: default_limit(),
            max_limit: default_max_limit(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_from")]
    pub from: String,

    #[serde(default = "default_false")]
    pub enable_tls: bool,

    pub cert_path: Option<PathBuf>,

    pub key_path: Option<PathBuf>,

    /// Enable OpenTelemetry tracing
    #[serde(default = "default_false")]
    pub otel: bool,

    /// OTLP endpoint
    pub otel_endpoint: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            from: default_from(),
            enable_tls: default_false(),
            cert_path: None,
            key_path: None,
            otel: default_false(),
            otel_endpoint: None,
        }
    }
}

fn default_from() -> String {
    "0.0.0.0:8080".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreConfig {
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub server: ServerConfig,
}
