// ttecton-core/src/search.rs
use crate::{
    //embedder::Embedder,
    error::{CoreError, Result},
    hash_indexer::MultiLangHashIndex,
    languages::{is_language_supported, whichlang_lang},
    metrics::{
        update_search, update_search_document, update_search_id, update_search_keyword,
        update_search_stemming, update_search_tree, update_search_vector,
    },
    schema::{
        FIELD_DATE, FIELD_DOCUMENT_NAME, FIELD_ID, FIELD_KEYWORDS, FIELD_LANG, FIELD_STEM,
        FIELD_TEXT, FIELD_TREE,
    },
};
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use std::str;
use std::{collections::HashMap, ops::Bound};

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, RangeQuery, TermQuery};
use tantivy::schema::{Facet, Field, IndexRecordOption, Schema, Term, Value};
use tantivy::tokenizer::TokenizerManager;
use tantivy::{Searcher, TantivyDocument};
use tracing::{debug, error, instrument};

use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub date: DateTime<Utc>,
    pub lang: String,
    pub tree: String,
    pub text: String,
    pub score: f32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, ToSchema)]
pub struct SearchParams {
    pub query_text: Option<String>,
    pub lang: Option<String>,
    pub name: Option<String>,
    pub min_date: Option<DateTime<Utc>>,
    pub max_date: Option<DateTime<Utc>>,
    pub limit: usize,
    pub offset: usize,
    pub id: Option<String>,
    pub tree: Option<String>,
    pub keys: Option<Vec<String>>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query_text: None,
            lang: None,
            name: None,
            min_date: None,
            max_date: None,
            limit: 10,
            offset: 0,
            id: None,
            tree: None,
            keys: None,
        }
    }
}

pub struct SearcherWrapper {
    schema: Schema,
    tokenizer: TokenizerManager,
    id_field: Field,
    doc_name_field: Field,
    date_field: Field,
    lang_field: Field,
    tree_field: Field,
    text_fields: HashMap<String, Field>,
    stem_fields: HashMap<String, Field>,
    keywords_field: Field,
    enabled_stemmer: bool,
}

impl SearcherWrapper {
    pub fn new(
        schema: Schema,
        tokenizer: TokenizerManager,
        enabled_stemmer: bool,
        languages: Vec<String>, //enabled_hnsw: bool,
    ) -> Result<Self> {
        let id_field = schema.get_field(FIELD_ID)?;
        let doc_name_field = schema.get_field(FIELD_DOCUMENT_NAME)?;
        let date_field = schema.get_field(FIELD_DATE)?;
        let lang_field = schema.get_field(FIELD_LANG)?;
        let tree_field = schema.get_field(FIELD_TREE)?;

        let text_fields: HashMap<String, Field> = languages
            .iter()
            .map(|l| {
                (
                    l.clone(),
                    schema
                        .get_field(format!("{}_{}", FIELD_TEXT, l).as_str())
                        .unwrap(),
                )
            })
            .collect();

        let stem_fields: HashMap<String, Field> = languages
            .iter()
            .map(|l| {
                (
                    l.clone(),
                    schema
                        .get_field(format!("{}_{}", FIELD_STEM, l).as_str())
                        .unwrap(),
                )
            })
            .collect();
        let keywords_field = schema.get_field(FIELD_KEYWORDS)?;

        Ok(Self {
            schema,
            tokenizer,
            id_field,
            doc_name_field,
            date_field,
            lang_field,
            tree_field,
            text_fields,
            stem_fields,
            keywords_field,
            enabled_stemmer,
        })
    }

    /// Search via stemming and language of a language text field. All `*` chars are replced.
    ///
    /// Additional filter are appended [ add_filters() ]
    ///
    /// Result: All blocks of containing the query. Sorted by score.
    #[instrument(skip(self))]
    pub async fn search_stern(
        &self,
        params: &SearchParams,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        if !self.enabled_stemmer {
            return Err(CoreError::InvalidArgument(
                "Search by stemming not possible".to_string(),
            ));
        }

        let Some(q_text) = &params.query_text else {
            return Err(CoreError::InvalidArgument(
                "Empty search not supported".to_string(),
            ));
        };

        let lang = &params
            .lang
            .clone()
            .unwrap_or_else(|| whichlang_lang(q_text).to_string());
        // check if index for language exits
        if !self.stem_fields.contains_key(lang) {
            return Err(CoreError::IndexDisabled(format!(
                "Search language: {} not supported",
                lang
            )));
        }

        // Stemming query
        let stem_field = self.stem_fields.get(lang.as_str()).unwrap();
        let query_parser = QueryParser::new(
            self.schema.clone(),
            vec![stem_field.clone()],
            self.tokenizer.clone(),
        );
        let (query, err): (Box<dyn Query>, Vec<_>) =
            query_parser.parse_query_lenient(&self::query_char_replacer(&q_text));
        if err.len() > 0 {
            error!("Query parser error: {:?}", err);
        }
        queries.push((Occur::Must, query));
        // Add filters
        self.add_filters(&mut queries, &params)?;

        let results = self.query_for_result(queries, params.limit, params.offset, searcher)?;
        update_search_stemming(start.elapsed().as_secs_f64(), &lang);
        Ok(results)
    }

    /// Search via fulltext (stop word query) of a language text field. All `*` chars are replced.
    ///
    /// Additional filter are appended [ add_filters() ]
    ///
    /// Result: All blocks of containing the query. Sorted by score.
    #[instrument(skip(self))]
    pub async fn search_fulltext(
        &self,
        params: &SearchParams,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        let Some(q_text) = &params.query_text else {
            return Err(CoreError::InvalidArgument(
                "Empty search not supported".to_string(),
            ));
        };

        let lang = &params
            .lang
            .clone()
            .unwrap_or_else(|| whichlang_lang(q_text).to_string());

        // check if index for language exits
        if !self.text_fields.contains_key(lang) {
            return Err(CoreError::IndexDisabled(format!(
                "Search language: {} not supported",
                lang
            )));
        }
        // Fulltext query
        let text_field = self.text_fields.get(lang.as_str()).unwrap();
        let query_parser = QueryParser::new(
            self.schema.clone(),
            vec![text_field.clone()],
            self.tokenizer.clone(),
        );
        let (query, err): (Box<dyn Query>, Vec<_>) =
            query_parser.parse_query_lenient(&query_char_replacer(&q_text));
        if err.len() > 0 {
            error!("Query parser error: {:?}", err);
        }
        queries.push((Occur::Must, query));
        // Add filters
        self.add_filters(&mut queries, &params)?;

        let results = self.query_for_result(queries, params.limit, params.offset, searcher)?;
        update_search(start.elapsed().as_secs_f64());
        Ok(results)
    }

    /// Search via vector index (stemming tokenizer) of the text field and return the entries out of the tantivy index via id search.
    /// An deleted tantivy entry would not result in an result.
    ///
    /// Additional filter are appended [ add_filters() ]
    ///
    /// Result: All blocks of containing the query out of the hnsw index. Sorted by score.
    #[instrument(skip(self, hnsw_index))]
    pub async fn search_vector(
        &self,
        params: &SearchParams,
        searcher: &Searcher,
        hnsw_index: &MultiLangHashIndex,
        ef_search: &usize,
    ) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let Some(text) = &params.query_text else {
            return Err(CoreError::InvalidArgument(
                "Empty search not supported".to_string(),
            ));
        };
        let lang = &params
            .lang
            .clone()
            .unwrap_or_else(|| whichlang_lang(text).to_string());
        if !is_language_supported(&lang) {
            return Err(CoreError::InvalidArgument(
                "Search language not supported".to_string(),
            ));
        }
        let top_k = params.limit;
        let vector_hits: Vec<(String, f32)> = hnsw_index
            .search(text, top_k, ef_search.clone(), lang)
            .unwrap();
        let ids = vector_hits
            .iter()
            .filter(|(_id, score)| score.gt(&0.0_f32))
            .map(|(id, _score)| id.as_str())
            .collect::<Vec<&str>>();
        debug!("Vector search count vector ids {}", &ids.len());
        let blocks = self.search_ids_with_filter(&ids, &params, searcher).await;
        let result: Result<Vec<SearchResult>> = match blocks {
            Ok(re) => {
                let v: Vec<SearchResult> = re
                    .iter()
                    .map(|s| {
                        let score: Vec<(String, f32)> = vector_hits
                            .iter()
                            .filter(|(id, _score)| id.as_str() == s.id.as_str())
                            .cloned()
                            .collect();
                        SearchResult {
                            score: score.first().unwrap().1,
                            ..s.clone()
                        }
                    })
                    .collect();
                debug!("Vector search count active ids {}", &v.len());
                Ok(v)
            }
            Err(e) => Err(e),
        };
        update_search_vector(start.elapsed().as_secs_f64(), &lang);
        result
    }

    /// Search for tree elements. Its a path search like e.g. `/first`
    /// will find documents which starts with the root path of "first".
    ///
    /// Additional filter are appended [ add_filters() ]
    ///
    /// Result: Self or all children of the path
    #[instrument(skip(self))]
    pub async fn search_tree(
        &self,
        params: &SearchParams,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        self.add_filters(&mut queries, &params)?;

        let results = self.query_for_result(queries, params.limit, params.offset, searcher)?;

        update_search_tree(start.elapsed().as_secs_f64());
        Ok(results)
    }

    ///Serarch for keywords with OR phrase
    /// Keywords `["txt","first"]` will find all of "txt" and "first".
    /// It will filter out anything with a containing `*`.
    ///
    /// Additional filter are appended [ add_filters() ]
    ///
    /// Result: all blocks with some of the containing keywords
    #[instrument(skip(self))]
    pub async fn search_keywords(
        &self,
        params: &SearchParams,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        // Keywords query - exact match
        if let Some(kw_query) = &params.keys {
            //let query_parser = QueryParser::for_index(&self.index, vec![self.keywords_field]);
            let query_parser = QueryParser::new(
                self.schema.clone(),
                vec![self.keywords_field],
                self.tokenizer.clone(),
            );
            let filter_keywords = &kw_query
                .iter()
                .filter(|k| !k.contains("*"))
                .map(String::as_str)
                .collect::<Vec<&str>>()
                .join(" AND ");

            let query = query_parser.parse_query(filter_keywords).unwrap();
            queries.push((Occur::Must, query));
        }

        self.add_filters(&mut queries, &params)?;

        let results = self.query_for_result(queries, params.limit, params.offset, searcher)?;

        update_search_keyword(start.elapsed().as_secs_f64());
        Ok(results)
    }

    /// Search for specific document name.
    ///
    /// Result all blocks for that document.
    #[instrument(skip(self))]
    pub async fn search_by_document(
        &self,
        name: &str,
        limit: &usize,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        // Document filter
        let term = Term::from_field_text(self.doc_name_field, name);
        queries.push((
            Occur::Must,
            Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
        ));

        let results = self.query_for_result(queries, limit.clone(), 0, searcher)?;
        update_search_document(start.elapsed().as_secs_f64());
        Ok(results)
    }

    /// Search a specific block by id.
    ///
    /// Result none or the block with the id
    #[instrument(skip(self))]
    pub async fn search_by_id(&self, id: &str, searcher: &Searcher) -> Result<Vec<SearchResult>> {
        let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        // Id filter
        let term = Term::from_field_text(self.id_field, id);
        queries.push((
            Occur::Must,
            Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
        ));

        let results = self.query_for_result(queries, 99, 0, searcher)?;
        update_search_id(start.elapsed().as_secs_f64());
        Ok(results)
    }

    /// Serarch for id's with OR phrase
    /// Search will find all ids inside the vec.
    ///
    /// Additional filter are appended [ add_filters() ]
    ///
    /// Result: all blocks with the id nad math the conditional filter
    async fn search_ids_with_filter(
        &self,
        keys: &Vec<&str>,
        params: &SearchParams,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        //let start = std::time::Instant::now();
        let mut queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        let query_parser = QueryParser::new(
            self.schema.clone(),
            vec![self.id_field],
            self.tokenizer.clone(),
        );
        // Id query - or match
        let collect_keys = keys.join(" OR ");
        let query = query_parser.parse_query(collect_keys.as_str()).unwrap();
        queries.push((Occur::Must, query));

        self.add_filters(&mut queries, &params)?;

        let results = self.query_for_result(queries, params.limit, params.offset, searcher)?;

        Ok(results)
    }

    /// This add filter for queries if a value for a parameter is given
    /// Filter for:
    /// * lang: Language
    /// * name: Document name
    /// * id: specific id
    /// * tree: value of a tree `/root/second`
    /// * min_date or max_date as timestamp
    fn add_filters(
        &self,
        queries: &mut Vec<(Occur, Box<dyn Query>)>,
        params: &SearchParams,
    ) -> Result<()> {
        // Language filter
        if let Some(lang) = &params.lang {
            let term = Term::from_field_text(self.lang_field, lang);
            queries.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        // Document name filter
        if let Some(doc_name) = &params.name {
            let term = Term::from_field_text(self.doc_name_field, doc_name);
            queries.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        // Id filter
        if let Some(id) = &params.id {
            //println!("{id}");
            let term = Term::from_field_text(self.id_field, id);
            queries.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        // Tree query - prefix match on tree path
        if let Some(tree_query) = &params.tree {
            let facet = Facet::from(tree_query);
            let facet_term = Term::from_facet(self.tree_field, &facet);

            queries.push((
                Occur::Must,
                Box::new(TermQuery::new(facet_term, IndexRecordOption::Basic)),
            ));
        }

        // Date range filter
        if params.min_date.is_some() || params.max_date.is_some() {
            let min = params
                .min_date
                .map(|d| d.timestamp_millis())
                .unwrap_or(i64::MIN);
            let max = params
                .max_date
                .map(|d| d.timestamp_millis())
                .unwrap_or(i64::MAX);

            let range_query = RangeQuery::new(
                Bound::Included(Term::from_field_i64(self.date_field, min)),
                Bound::Excluded(Term::from_field_i64(self.date_field, max)),
            );
            queries.push((Occur::Must, Box::new(range_query)));
        }

        Ok(())
    }

    fn query_for_result(
        &self,
        queries: Vec<(Occur, Box<dyn Query + 'static>)>,
        limit: usize,
        offset: usize,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let final_query: Box<dyn Query> = if queries.is_empty() {
            Box::new(AllQuery)
        } else if queries.len() == 1 {
            queries.into_iter().next().unwrap().1
        } else {
            Box::new(BooleanQuery::new(queries))
        };

        let top_docs = searcher.search(
            &final_query,
            &TopDocs::with_limit(limit)
                .and_offset(offset)
                .order_by_score(),
        )?;

        self.collect_results(top_docs, searcher)
    }

    fn collect_results(
        &self,
        top_docs: Vec<(f32, tantivy::DocAddress)>,
        searcher: &Searcher,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let name = doc
                .get_first(self.doc_name_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let date = doc
                .get_first(self.date_field)
                .and_then(|v| v.as_i64())
                .map(|ts| DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now))
                .unwrap_or_else(Utc::now);

            let lang = doc
                .get_first(self.lang_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // facet delivers a string where the deliter are byte 0
            // they dont use their own function of
            // schema/facet
            // This function is the inverse of Facet::from(&str).
            // pub fn to_path_string(&self) -> String
            let org_tree: Vec<u8> = doc
                .get_first(self.tree_field)
                .and_then(|v| v.as_facet())
                .unwrap_or("")
                .as_bytes()
                .into_iter()
                .map(|&byte| if byte == 0 { "/".as_bytes()[0] } else { byte })
                .collect();

            let tree = format!("/{}", String::from_utf8(org_tree).unwrap_or("".to_string()));

            let text = doc
                //.get_first(self.text_field)
                .get_first(
                    self.text_fields.get(lang.as_str()).unwrap().clone(), //self.schema
                )
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let keywords = doc
                .get_all(self.keywords_field)
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>();

            results.push(SearchResult {
                id,
                name,
                date,
                lang,
                tree,
                text,
                score,
                keywords,
            });
        }

        Ok(results)
    }
}
fn query_char_replacer(query: &str) -> String {
    query
        .chars()
        .filter(|&c| match c {
            '*' => false,
            '(' => false,
            ')' => false,
            ':' => false,
            _ => true,
        })
        .collect()
}
