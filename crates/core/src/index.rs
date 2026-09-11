// tecton-core/src/index.rs
use crate::{
    config::IndexConfig,
    //embedder::Embedder,
    error::{CoreError, Result},
    hash_indexer::MultiLangHashIndex,
    //hnsw_wrapper::HnswIndex,
    metrics::{
        commit_duration, compact_duration, update_document_insert, update_document_update,
        update_documents_total,
    },
    schema::{
        FIELD_DATE,
        FIELD_DOCUMENT_NAME,
        FIELD_ID,
        FIELD_KEYWORDS,
        FIELD_LANG,
        FIELD_STEM,
        FIELD_TEXT,
        FIELD_TREE,
        // FIELD_VECTOR,
        build_schema,
        register_tokenizers,
    },
};
use chrono::{DateTime, Utc};

use parking_lot::RwLock;
//use serde_json::value;
use std::path::PathBuf;
use std::str::FromStr;
//use std::sync::Arc;

use tantivy::{
    DocAddress, IndexReader, ReloadPolicy, Searcher,
    collector::{Count, TopDocs},
    tokenizer::TokenizerManager,
};
//use tantivy::directory::MmapDirectory;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::{Index, IndexWriter, Term, schema::*};
use tracing::{info, instrument, warn};
use ulid::Ulid;

//const DIM: usize = 4096; //384;

pub struct IndexManager {
    index: Index,
    pub(crate) config: IndexConfig,
    writer: RwLock<Option<IndexWriter>>,
    num_docs: RwLock<u64>,
    reader: IndexReader,
    // hnsw_embedder: Option<HnswEmbedder>,
    hswn_index: Option<MultiLangHashIndex>,
}

impl IndexManager {
    pub fn create(config: &IndexConfig) -> Result<Self> {
        let tantivy_dir = Self::get_tantivy_index_path(&config);
        std::fs::create_dir_all(&tantivy_dir.as_path())?;
        println!("{:?} ", config);
        let schema = build_schema(config.enable_steam, &config.languages); //, config.enable_vector_index);
        let index = Index::create_in_dir(tantivy_dir, schema)?;

        Self::from_index_with_writer(index, config)
    }

    pub fn open(config: &IndexConfig) -> Result<Self> {
        if !Self::get_tantivy_index_path(&config).exists() {
            return Err(CoreError::IndexNotFound(
                config.index_path.display().to_string(),
            ));
        }

        let index = Index::open_in_dir(Self::get_tantivy_index_path(&config))?;
        Self::from_index_with_writer(index, config)
    }

    fn from_index_with_writer(index: Index, config: &IndexConfig) -> Result<Self> {
        register_tokenizers(&index, config.enable_steam, &config.languages);

        let writer = index.writer(config.buffer_size)?;

        // Count existing documents
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| CoreError::Tantivy(e))?;

        let searcher = reader.searcher();
        let num_docs = searcher.num_docs();

        let hswn_index = if config.enable_hnsw {
            Some(MultiLangHashIndex::load_or_create(
                &config.hnsw,
                &config.index_path.clone(),
                &config.languages,
            ))
        } else {
            None
        };

        Ok(Self {
            index,
            config: config.clone(),
            writer: RwLock::new(Some(writer)),
            num_docs: RwLock::new(num_docs),
            reader,
            // hnsw_embedder: hnsw_embedder,
            hswn_index: hswn_index,
        })
    }

    pub fn open_or_create(config: &IndexConfig) -> Result<Self> {
        if config.index_path.exists() {
            Self::open(config)
        } else {
            Self::create(config)
        }
    }

    fn get_tantivy_index_path(config: &IndexConfig) -> PathBuf {
        let mut path_buff = PathBuf::from(&config.index_path);
        path_buff.push(&config.tantivy_dir);
        path_buff
    }

    #[instrument(skip(self, text))]
    pub fn add_document(
        &self,
        id: Option<String>,
        document_name: String,
        date: DateTime<Utc>,
        lang: String,
        tree: String,
        text: String,
        keywords: Vec<String>,
    ) -> Result<String> {
        let doc_id = id.unwrap_or_else(|| Ulid::generate().to_string());
        let timestamp = date.timestamp_millis();
        let schema = self.index.schema();

        if !self.config.is_language_supported(lang.as_str()) {
            return Err(CoreError::InvalidArgument("Language not supported".into()));
        }

        let id_field = schema.get_field(FIELD_ID)?;
        let doc_name_field = schema.get_field(FIELD_DOCUMENT_NAME)?;
        let date_field = schema.get_field(FIELD_DATE)?;
        let lang_field = schema.get_field(FIELD_LANG)?;
        let tree_field = schema.get_field(FIELD_TREE)?;

        let mut doc = TantivyDocument::default();
        let ulid = match Ulid::from_str(&doc_id) {
            Ok(u) => u,
            Err(_err) => return Err(CoreError::InvalidArgument("Invalid ULID".into())),
        };

        if self.id_exists(&doc_id) {
            return Err(CoreError::InvalidArgument("Id exists.".to_owned()));
        }
        doc.add_text(id_field, ulid.to_string());
        doc.add_text(doc_name_field, document_name);
        doc.add_i64(date_field, timestamp);
        doc.add_text(lang_field, lang.clone());

        //let text_field = schema.get_field(FIELD_TEXT)?;
        let text_lang = format!("{}_{}", FIELD_TEXT, lang);
        let text_lang_field = schema.get_field(text_lang.as_str())?;
        doc.add_text(text_lang_field, &text);

        if self.config.enable_steam & self.config.is_language_supported(lang.as_str()) {
            let stem_lang = format!("{}_{}", FIELD_STEM, lang);
            let stem_lang_field = schema.get_field(stem_lang.as_str())?;
            doc.add_text(stem_lang_field, &text);
        }

        if !tree.is_empty() {
            doc.add_facet(tree_field, Facet::from(&tree));
        }
        let field = schema.get_field(FIELD_KEYWORDS)?;
        for keyword in keywords {
            doc.add_text(field, keyword);
        }

        let mut writer_guard = self.writer.write();
        let writer = writer_guard.as_mut().ok_or(CoreError::Locked)?;

        writer.add_document(doc)?;

        if self.config.enable_hnsw {
            println!("index hnsw");
            let hswn_index = self.hswn_index.as_ref().unwrap();
            hswn_index.index(text.as_str(), ulid.to_string().as_str(), lang.as_str());
            hswn_index.persist(lang.as_str())?;
        }

        *self.num_docs.write() += 1;

        update_document_insert(text.len() as f64, &lang);
        update_documents_total(self.num_docs());
        Ok(doc_id)
    }

    #[instrument(skip(self))]
    pub fn commit(&self) -> Result<()> {
        let mut writer_guard = self.writer.write();
        let writer = writer_guard.as_mut().ok_or(CoreError::Locked)?;

        let start = std::time::Instant::now();
        writer.commit()?;
        // language specific persist
        /*
        if self.config.enable_hnsw {
            self.hswn_index.as_ref().unwrap().hnsw.persist()?
        }
        */

        commit_duration(start.elapsed().as_secs_f64());
        info!("Index committed successfully");

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn update_keywords(&self, id: &str, keywords: Vec<String>, replace: bool) -> Result<()> {
        let ulid = match Ulid::from_str(&id) {
            Ok(u) => u,
            Err(_err) => return Err(CoreError::InvalidArgument("Invalid ULID".into())),
        };

        let searcher = self.reader.searcher();
        let schema = self.index.schema();

        let id_field = schema.get_field(FIELD_ID).unwrap();
        let term = Term::from_field_text(id_field, &ulid.to_string());

        let query = TermQuery::new(term, IndexRecordOption::Basic);

        let (top_docs, count) =
            searcher.search(&query, &(TopDocs::with_limit(2).order_by_score(), Count))?;
        // use only the first result
        let doc_address: DocAddress = if count > 0 {
            let (_score, doc_address) = top_docs[0];
            doc_address
        } else {
            return Err(CoreError::DocumentNotFound(ulid.to_string()));
        };

        let keyword_field = schema.get_field(FIELD_KEYWORDS)?;
        let org_doc: TantivyDocument = searcher.doc(doc_address)?;
        let mut doc = if !replace {
            org_doc
        } else {
            // clone all fields except the keywords
            let mut new_doc = TantivyDocument::default();
            for (field, value) in org_doc.iter_fields_and_values() {
                if field.field_id() != keyword_field.field_id() {
                    new_doc.add_field_value(field, value);
                }
            }
            new_doc
        };
        // add the new keywords
        for keyword in keywords {
            doc.add_text(keyword_field, keyword);
        }

        // delete old document
        self.delete_document(id)?;

        // add the new document
        let mut writer_guard = self.writer.write();
        let writer = writer_guard.as_mut().ok_or(CoreError::Locked)?;
        writer.add_document(doc)?;

        *self.num_docs.write() += 1;

        update_document_update();
        update_documents_total(self.num_docs());
        Ok(())
    }

    #[instrument(skip(self))]
    pub fn delete_document(&self, id: &str) -> Result<()> {
        let mut writer_guard = self.writer.write();
        let writer = writer_guard.as_mut().ok_or(CoreError::Locked)?;

        let schema = self.index.schema();
        let id_field = schema.get_field(FIELD_ID).unwrap();
        let term = Term::from_field_text(id_field, id);

        writer.delete_term(term);
        writer.commit()?;
        *self.num_docs.write() -= 1;
        update_documents_total(self.num_docs());
        Ok(())
    }

    #[instrument(skip(self))]
    pub fn compact(&self) -> Result<()> {
        info!("Starting index compaction");
        let start = std::time::Instant::now();

        let mut writer_guard = self.writer.write();
        let writer = writer_guard.as_mut().ok_or(CoreError::Locked)?;

        writer.commit()?;

        //drop(writer_guard);
        compact_duration(start.elapsed().as_secs_f64());
        info!("Index compaction completed");

        Ok(())
    }

    pub fn num_docs(&self) -> u64 {
        *self.num_docs.read()
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn schema(&self) -> Schema {
        self.index.schema()
    }

    pub fn tokenizer(&self) -> &TokenizerManager {
        &self.index.tokenizers()
    }
    pub fn reader(&self) -> &IndexReader {
        &self.reader
    }

    pub fn searcher(&self) -> Searcher {
        self.reader.searcher()
    }

    pub fn is_language_supported(&self, lang: &str) -> bool {
        self.config.is_language_supported(lang)
    }

    pub fn hnsw_index(&self) -> Result<&MultiLangHashIndex> {
        match &self.hswn_index {
            Some(index) => Ok(&index),
            _ => Err(CoreError::ModelNotInit),
        }
    }

    pub fn get_query_parser(&self, fields: Vec<Field>) -> QueryParser {
        QueryParser::for_index(&self.index, fields)
    }

    pub fn config(&self) -> &IndexConfig {
        &self.config
    }

    fn id_exists(&self, id: &str) -> bool {
        let searcher = self.searcher();
        let field = self.index.schema().get_field(FIELD_ID).unwrap();
        match searcher.search(
            &TermQuery::new(Term::from_field_text(field, id), IndexRecordOption::Basic),
            &Count,
        ) {
            Ok(val) => val > 0_usize,
            Err(_) => false,
        }
    }
}
