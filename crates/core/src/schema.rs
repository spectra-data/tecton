use crate::languages;

use tantivy::schema::*;
use tantivy::tokenizer::*;

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const FIELD_ID: &str = "id";
pub const FIELD_DOCUMENT_NAME: &str = "name";
pub const FIELD_DATE: &str = "date";
pub const FIELD_LANG: &str = "lang";
pub const FIELD_TREE: &str = "tree";
pub const FIELD_TEXT: &str = "text";
pub const FIELD_STEM: &str = "stem";
pub const FIELD_VECTOR: &str = "vector";
pub const FIELD_KEYWORDS: &str = "keywords";

pub fn build_schema(enable_stern: bool, languages: &Vec<String>) -> Schema {
    let mut schema_builder = Schema::builder();

    let string_indexing = TextFieldIndexing::default()
        .set_tokenizer("raw_tokenizer")
        .set_index_option(IndexRecordOption::Basic);
    let string_options = TextOptions::default()
        .set_indexing_options(string_indexing)
        .set_stored();

    // ID - stored, indexed, fast field for lookup
    schema_builder.add_text_field(FIELD_ID, string_options.clone());
    // DocumentName - stored, indexed for filtering
    schema_builder.add_text_field(FIELD_DOCUMENT_NAME, string_options); //STRING | STORED);

    // Date - stored as i64 timestamp, indexed for range queries
    schema_builder.add_i64_field(FIELD_DATE, INDEXED | STORED | FAST);

    // Lang - stored, indexed for filtering
    schema_builder.add_text_field(FIELD_LANG, STRING | STORED);

    schema_builder.add_facet_field(FIELD_TREE, FacetOptions::default().set_stored());

    for lang in languages {
        // Fulltext with stopwords for  language
        schema_builder.add_text_field(
            format!("{}_{}", FIELD_TEXT, lang).as_str(),
            TextOptions::default()
                .set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer(self::get_text_tokenizer_name(lang).as_str()) // Unique tokenizer per field
                        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                )
                .set_stored(),
        );
        // stem (stemming) enabled language text fields
        if enable_stern {
            schema_builder.add_text_field(
                format!("{}_{}", FIELD_STEM, lang).as_str(),
                TextOptions::default().set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer(self::get_stem_tokenizer_name(lang).as_str()) // Unique tokenizer per field
                        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                ),
                //.set_stored(),
            );
        }
    }

    // Keywords - stored, indexed for exact matching
    let keyword_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("keyword_tokenizer")
                .set_index_option(IndexRecordOption::WithFreqs),
        )
        .set_stored();
    schema_builder.add_text_field(FIELD_KEYWORDS, keyword_options);

    let schema = schema_builder.build();

    schema
}

pub fn register_tokenizers(index: &tantivy::Index, enable_stemming: bool, languages: &Vec<String>) {
    let tokenizers = index.tokenizers();

    // Keyword tokenizer - exact match, lowercase
    tokenizers.register(
        "keyword_tokenizer",
        tantivy::tokenizer::SimpleTokenizer::default(),
    );

    tokenizers.register("raw_tokenizer", tantivy::tokenizer::RawTokenizer::default());
    tokenizers.register(
        "facet_tokenizer",
        tantivy::tokenizer::FacetTokenizer::default(),
    );

    // Register language-specific text tokenizer
    register_text_toikenizer(&tokenizers, &languages);

    // Register language-specific stemmers
    if enable_stemming {
        register_stem_toikenizer(&tokenizers, &languages);
    }
    // Default fallback
    tokenizers.register("text_tokenizer", get_default_text_tokenizer());
}

fn register_stem_toikenizer(tokenizers: &TokenizerManager, languages: &Vec<String>) {
    for lang in languages {
        let language = languages::get_language_for_lang(lang);
        let tokenizer = TextAnalyzer::builder(tantivy::tokenizer::SimpleTokenizer::default())
            .filter(LowerCaser)
            //.filter(AsciiFoldingFilter)
            .filter(
                StopWordFilter::new(language)
                    .unwrap_or(StopWordFilter::new(Language::English).unwrap()),
            )
            .filter(Stemmer::new(language))
            .filter(RemoveLongFilter::limit(40))
            .build();

        tokenizers.register(get_stem_tokenizer_name(lang).as_str(), tokenizer);
    }
}

fn register_text_toikenizer(tokenizers: &TokenizerManager, languages: &Vec<String>) {
    for lang in languages {
        let language = languages::get_language_for_lang(lang);
        let tokenizer = TextAnalyzer::builder(tantivy::tokenizer::SimpleTokenizer::default())
            .filter(LowerCaser)
            // .filter(AsciiFoldingFilter)
            .filter(
                StopWordFilter::new(language)
                    .unwrap_or(StopWordFilter::new(Language::English).unwrap()),
            )
            .build();

        tokenizers.register(get_text_tokenizer_name(lang).as_str(), tokenizer);
    }
}

pub fn get_stem_tokenizer_name(lang: &str) -> String {
    if languages::is_language_supported(lang) {
        return format!("{}_stem", lang);
    }
    "en_stem".to_string()
}

pub fn get_text_tokenizer_name(lang: &str) -> String {
    if languages::is_language_supported(lang) {
        return format!("{}_txt", lang);
    }
    "en_txt".to_string()
}

fn get_default_text_tokenizer() -> TextAnalyzer {
    TextAnalyzer::builder(tantivy::tokenizer::SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(AsciiFoldingFilter)
        .filter(StopWordFilter::new(Language::English).unwrap())
        .build()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlockData {
    pub id: String,
    pub name: String,
    pub date: DateTime<Utc>,
    pub lang: String,
    pub tree: String,
    pub text: String,
    pub keywords: Vec<String>,
}
