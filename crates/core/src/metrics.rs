use const_format::concatcp;
// ttecton-core/src/metrics.rs
use metrics::{
    Unit, counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Duration;

pub fn setup_metrics_recorder() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    let recorder_handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .unwrap()
        .install_recorder()
        .unwrap();

    let upkeep_handle = recorder_handle.clone();
    register_metrics();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            upkeep_handle.run_upkeep();
        }
    });

    recorder_handle
}

pub const TB_INDEX_NAME: &'static str = "tecton";
//const TB_INDEX_PREFIX: &'static str = "tecton";

pub const LANGUAGE: &'static str = "lang";

const COMMIT_DURATION_SEC: &'static str = concatcp!(TB_INDEX_NAME, "_commit_duration_seconds");
const COMPACT_DURATION_SEC: &'static str = concatcp!(TB_INDEX_NAME, "_compact_duration_seconds");

const DOCUMENTS_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_documents_total");
const DOCUMENTS_UPDATED_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_documents_updated_total");
const DOCUMENT_SIZE_BYTES: &'static str = concatcp!(TB_INDEX_NAME, "_document_size_bytes");
const DOCUMENT_ADDED_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_documents_added_total");
const ERRORS_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_errors_total");
const INDEX_SIZE_BYTES: &'static str = concatcp!(TB_INDEX_NAME, "_index_size_bytes");
const SEARCH_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_total");
const SEARCH_STERN_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_stern_total");
const SEARCH_TREE_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_tree_total");
const SEARCH_KEYWORD_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_keyword_total");
const SEARCH_ID_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_id_total");
const SEARCH_DOCUMENT_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_document_total");
const SEARCH_VECTOR_TOTOAL: &'static str = concatcp!(TB_INDEX_NAME, "_search_vector_total");

const SEARCH_DURATION_SEC: &'static str = concatcp!(TB_INDEX_NAME, "_search_duration_seconds");
const SEARCH_STERN_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_search_stern_duration_seconds");
const SEARCH_TREE_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_search_tree_duration_seconds");
const SEARCH_KEY_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_search_keyword_duration_seconds");
const SEARCH_ID_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_search_id_duration_seconds");
const SEARCH_DOCUMENT_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_search_document_duration_seconds");
const SEARCH_VECTOR_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_search_vector_duration_seconds");

const HTTP_REQUESTS_TOTAL: &'static str = concatcp!(TB_INDEX_NAME, "_http_requests_total");
const HTTP_REQUESTS_DURATION_SEC: &'static str =
    concatcp!(TB_INDEX_NAME, "_http_requests_duration_seconds");

fn register_metrics() {
    // Counters

    describe_counter!(
        DOCUMENT_ADDED_TOTOAL,
        "Total number of documents added to the index."
    );
    describe_counter!(
        DOCUMENTS_UPDATED_TOTOAL,
        "Total number of updated documents to the index."
    );
    describe_counter!(SEARCH_TOTOAL, "Total number of search requests.");
    describe_counter!(
        SEARCH_STERN_TOTOAL,
        "Totoal number of stern search requests."
    );

    describe_counter!(SEARCH_TREE_TOTOAL, "Total number of tree search requests.");
    describe_counter!(
        SEARCH_KEYWORD_TOTOAL,
        "Total number of keyword search requests."
    );

    describe_counter!(SEARCH_ID_TOTOAL, "Total number of id search requests.");
    describe_counter!(
        SEARCH_DOCUMENT_TOTOAL,
        "Total number of document search requests."
    );

    describe_counter!(
        SEARCH_VECTOR_TOTOAL,
        "Total number of vector search requests."
    );

    describe_counter!(ERRORS_TOTOAL, "Total number of errors.");

    describe_counter!(HTTP_REQUESTS_TOTAL, "Total number of http requests.");

    // Histograms
    describe_histogram!(
        DOCUMENT_SIZE_BYTES,
        Unit::Bytes,
        "Size of indexed documents in bytes."
    );
    describe_histogram!(
        COMMIT_DURATION_SEC,
        Unit::Seconds,
        "Time spent committing index changes."
    );
    describe_histogram!(
        COMPACT_DURATION_SEC,
        Unit::Seconds,
        "Time spent compacting index."
    );
    describe_histogram!(
        SEARCH_DURATION_SEC,
        Unit::Seconds,
        "Fulltext search duration."
    );
    describe_histogram!(
        SEARCH_STERN_DURATION_SEC,
        Unit::Seconds,
        "Stern search duration."
    );
    describe_histogram!(
        SEARCH_TREE_DURATION_SEC,
        Unit::Seconds,
        "Tree search duration."
    );
    describe_histogram!(
        SEARCH_KEY_DURATION_SEC,
        Unit::Seconds,
        "Keyword search duration."
    );

    describe_histogram!(SEARCH_ID_DURATION_SEC, Unit::Seconds, "Id search duration.");
    describe_histogram!(
        SEARCH_DOCUMENT_DURATION_SEC,
        Unit::Seconds,
        "Document search duration."
    );
    describe_histogram!(
        SEARCH_VECTOR_DURATION_SEC,
        Unit::Seconds,
        "Vector search duration."
    );

    describe_histogram!(
        HTTP_REQUESTS_DURATION_SEC,
        Unit::Seconds,
        "Http requests duration."
    );

    // Gauges
    describe_gauge!(DOCUMENTS_TOTOAL, "Current number of documents in index.");
    describe_gauge!(INDEX_SIZE_BYTES, Unit::Bytes, "Size of index on disk.");
}

pub fn update_documents_total(num_docs: u64) {
    gauge!(DOCUMENTS_TOTOAL).set(num_docs as f64);
}

pub fn compact_duration(durration_sec: f64) {
    histogram!(COMPACT_DURATION_SEC).record(durration_sec);
}

pub fn documents_added(num_docs: u64) {
    gauge!(DOCUMENT_ADDED_TOTOAL).set(num_docs as f64);
}
pub fn update_index_metrics(num_docs: u64, index_size_bytes: u64) {
    gauge!(DOCUMENT_ADDED_TOTOAL).set(num_docs as f64);
    gauge!(INDEX_SIZE_BYTES).set(index_size_bytes as f64);
}

pub fn update_document_insert(text_len_bytes: f64, lang: &str) {
    let labels = [(LANGUAGE, lang.to_owned())];
    counter!(DOCUMENT_ADDED_TOTOAL, &labels).increment(1);
    histogram!(DOCUMENT_SIZE_BYTES, &labels).record(text_len_bytes);
}

pub fn update_document_update() {
    counter!(DOCUMENTS_UPDATED_TOTOAL).increment(1);
}

pub fn record_error(error_type: &str) {
    counter!(ERRORS_TOTOAL, "type" => error_type.to_string()).increment(1);
}

pub fn update_search(sec: f64) {
    histogram!(SEARCH_DURATION_SEC).record(sec);
    counter!(SEARCH_TOTOAL).increment(1);
}

pub fn update_search_stemming(sec: f64, lang: &str) {
    let labels = [(LANGUAGE, lang.to_owned())];
    histogram!(SEARCH_STERN_DURATION_SEC, &labels).record(sec);
    counter!(SEARCH_STERN_TOTOAL, &labels).increment(1);
}

pub fn update_search_tree(sec: f64) {
    histogram!(SEARCH_TREE_DURATION_SEC).record(sec);
    counter!(SEARCH_TREE_TOTOAL).increment(1);
}

pub fn update_search_keyword(sec: f64) {
    histogram!(SEARCH_KEY_DURATION_SEC).record(sec);
    counter!(SEARCH_KEYWORD_TOTOAL).increment(1);
}

pub fn update_search_id(sec: f64) {
    histogram!(SEARCH_ID_DURATION_SEC).record(sec);
    counter!(SEARCH_ID_TOTOAL).increment(1);
}

pub fn update_search_document(sec: f64) {
    histogram!(SEARCH_DOCUMENT_DURATION_SEC).record(sec);
    counter!(SEARCH_DOCUMENT_TOTOAL).increment(1);
}

pub fn update_search_vector(sec: f64, lang: &str) {
    let labels = [(LANGUAGE, lang.to_owned())];
    histogram!(SEARCH_VECTOR_DURATION_SEC, &labels).record(sec);
    counter!(SEARCH_VECTOR_TOTOAL).increment(1);
}

pub fn commit_duration(sec: f64) {
    histogram!(COMMIT_DURATION_SEC).record(sec);
}

pub fn http_requests_global(labels: Vec<(String, String)>, latency: f64) {
    let l = labels.clone();
    histogram!(HTTP_REQUESTS_DURATION_SEC, &l).record(latency.clone());
    counter!(HTTP_REQUESTS_TOTAL, &l).increment(1);
}
