use crate::{Result, config::HnswConfig, languages};

use ahash::RandomState;
use dashmap::DashMap; // Fast keyed hasher
use std::hash::{BuildHasher, Hash, Hasher};
use tantivy::tokenizer::{
    Language, LowerCaser, SimpleTokenizer, Stemmer, StopWordFilter, TextAnalyzer, TokenStream,
    TokenizerManager,
};

use fast_hnsw::distance::Cosine;
use fast_hnsw::{Builder, LabeledIndex};

use std::path::PathBuf;

use tracing::info;

const INDEX_FILE_SUFFIX: &str = "_index.hnsw";

pub struct MultiLangHashIndex {
    indexes: DashMap<String, LabeledIndex<Cosine, String>>,
    tokenizers: TokenizerManager,
    // Config for persistence
    config: HnswConfig,
    index_files_path: PathBuf,
}

impl MultiLangHashIndex {
    pub fn load_or_create(
        config: &HnswConfig,
        main_path: &PathBuf,
        languages: &Vec<String>,
    ) -> Self {
        //let main_path=PathBuf::from(&config.index_path);

        let mut index_files_path = PathBuf::from(main_path);

        index_files_path.push(&config.hnsw_dir);
        if !&index_files_path.exists() {
            std::fs::create_dir_all(&index_files_path).unwrap();
        }
        // let path = path_buff.as_path();

        let indexes: DashMap<String, LabeledIndex<Cosine, String>> = DashMap::new();
        let tokenizers = TokenizerManager::default();
        for lang in languages.iter() {
            //let mut file_path = path_buff.clone();
            //file_path.push(format!("{}{}", lang.as_str(), INDEX_FILE_SUFFIX).as_str());
            let file_path = build_index_file_path(&index_files_path, lang);
            let hnsw: LabeledIndex<Cosine, String> = if file_path.exists() {
                info!("Loading HNSW index from {:?}", file_path);
                LabeledIndex::<Cosine, String>::load(&file_path, Cosine).expect("Hnsw index loaded")
            } else {
                info!("Creating new HNSW index at {:?}", file_path);
                //let max_layers = 16;
                Builder::new()
                    .m(config.m)
                    .ef_construction(config.ef_construction)
                    .capacity(1000)
                    //.m0(hnsw_cfg.m)
                    .seed(config.seed)
                    .build_labeled(Cosine)
            };
            indexes.insert(lang.clone(), hnsw);
            if config.stem {
                tokenizers.register(get_stem_lang(lang).as_str(), build_stemer_tokenizer(lang));
            }
        }

        Self {
            indexes: indexes,
            tokenizers: tokenizers,
            config: config.clone(),
            index_files_path: index_files_path.clone(),
        }
    }

    pub fn search(
        &self,
        text: &str,
        top_k: usize,
        ef: usize,
        lang: &str,
    ) -> Result<Vec<(String, f32)>> {
        let mut analyzer = if self.config.stem {
            self.tokenizers.get(get_stem_lang(lang).as_str()).unwrap()
        } else {
            self.tokenizers.get("default").unwrap()
        };
        let mut token_stream = analyzer.token_stream(text);

        let query_vec = token_stream_to_vector(
            &mut token_stream,
            self.config.dimension,
            self.config.n_grams,
        );
        let index = self.indexes.get(lang).unwrap();
        let hswn = index.value();
        let neighbors = hswn.search(&query_vec, top_k, ef);

        let mut results = Vec::with_capacity(neighbors.len());
        for hit in neighbors {
            //println!("hit.distance {}", hit.distance);
            let score = 1.0 - hit.distance;
            results.push((hit.payload.clone(), score.max(0.0).min(1.0)));
            //results.push((hit.payload.clone(), score));
        }
        Ok(results)
    }

    pub fn index(&self, text: &str, id: &str, lang: &str) {
        //println!("bm25: {}", self.config.hnsw.bm25);
        let mut analyzer = if self.config.stem {
            self.tokenizers.get(get_stem_lang(lang).as_str()).unwrap()
        } else {
            self.tokenizers.get("default").unwrap()
        };
        let mut token_stream = analyzer.token_stream(text);

        let query_vec = token_stream_to_vector(
            &mut token_stream,
            self.config.dimension,
            self.config.n_grams,
        );

        let mut index = self.indexes.get_mut(lang).unwrap();
        let hswn = index.value_mut();
        hswn.insert(query_vec, id.to_string());
    }

    /// Persist to disk.
    pub fn persist(&self, lang: &str) -> Result<()> {
        let index = self.indexes.get(lang).unwrap();
        let hswn = index.value();
        let path = build_index_file_path(&self.index_files_path, lang);
        // Save HNSW
        hswn.save(path.as_path())?;
        info!("HNSW persisted to {:?}", path);
        Ok(())
    }
}

fn build_index_file_path(index_files_path: &PathBuf, lang: &str) -> PathBuf {
    let mut file_path = PathBuf::from(index_files_path);
    file_path.push(format!("{}{}", lang, INDEX_FILE_SUFFIX).as_str());
    file_path.clone()
}

fn get_stem_lang(lang: &str) -> String {
    format!("{}_stem_v", lang)
}

fn build_stemer_tokenizer(lang: &str) -> TextAnalyzer {
    let language = languages::get_language_for_lang(lang);
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(
            StopWordFilter::new(language)
                .unwrap_or(StopWordFilter::new(Language::English).unwrap()),
        )
        .filter(Stemmer::new(language))
        //.filter(RemoveLongFilter::limit(40))
        .build()
}

/// Converts a Tantivy TokenStream into a dense f32 vector (Vec<f32>).
/// Uses Feature Hashing with Signed Hashing for unbiased estimation.
fn token_stream_to_vector(
    token_stream: &mut dyn TokenStream,
    dim: usize,
    n_gnrams: usize,
) -> Vec<f32> {
    let mut vec = vec![0.0f32; dim];
    let mask = dim - 1; // Assumes dim is power of 2

    // We need to peek ahead for N-grams. Easiest: collect tokens first.
    let mut tokens = Vec::new();
    while let Some(token) = token_stream.next() {
        tokens.push(token.text.clone());
    }
    //println!("tokens: {:?}", tokens);
    // Hash N-grams
    for window in tokens.windows(n_gnrams) {
        // Create a unique string/bytes representation for the n-gram
        // e.g., "quick|brown" or just hash sequentially
        let hash_builder = RandomState::with_seeds(0xDEADBEEF, 0xDEADBEEF, 0xCAFEBABE, 0xCAFEBABE);
        let mut hasher = hash_builder.build_hasher(); // Fixed seeds = Deterministic
        for (i, token_text) in window.iter().enumerate() {
            token_text.hash(&mut hasher);
            i.hash(&mut hasher); // Position within n-gram matters
        }
        let hash_val = hasher.finish();

        // Signed Hashing (Second hash for sign)
        // This makes E[vector] = 0, variance = 1/dim. Unbiased estimator.
        let hash_builder = RandomState::with_seeds(0xBEEFDEAD, 0xBEEFDEAD, 0xBABECAFE, 0xBABECAFE);
        let mut sign_hasher = hash_builder.build_hasher();
        window.hash(&mut sign_hasher);
        let sign = if (sign_hasher.finish() & 1) == 0 {
            1.0
        } else {
            -1.0
        };
        // Map to index
        let idx = (hash_val & mask as u64) as usize;
        vec[idx] += sign;
    }
    // L2 Normalize (Essential for Cosine Similarity / HNSW)
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}
