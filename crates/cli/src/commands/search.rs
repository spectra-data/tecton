// tecton-cli/src/commands/search.rs
use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
//use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;
//use tracing::info;
use tecton_core::{TectonIndex, config::CoreConfig, search::SearchParams};

#[derive(Args, PartialEq, Eq, Clone)]
pub struct SearchArgs {
    #[command(subcommand)]
    pub command: SearchCommand,
}

#[derive(Subcommand, PartialEq, Eq, Clone)]
pub enum SearchCommand {
    /// Stemming search
    Stem(StemArgs),

    /// Fulltext search
    Fulltext(FulltextArgs),

    /// Vector search
    Vector(VectorArgs),

    /// Tree/hierarchy search
    Tree(TreeArgs),

    /// Keyword search
    Keyword(KeywordArgs),

    /// Search by document name
    Document(DocumentArgs),

    /// Search by document name
    Id(IdArgs),
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct StemArgs {
    /// Search query
    #[arg(short, long)]
    pub query: String,

    /// Language for tokenizer and index
    #[arg(short, long)]
    pub language: String,

    /// Filter by document name
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    /// Minimum date (ISO 8601)
    #[arg(long)]
    pub min_date: Option<String>,

    /// Maximum date (ISO 8601)
    #[arg(long)]
    pub max_date: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Offset for pagination
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Output format: json, table, text
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,

    /// Filter by Id
    #[arg(long)]
    pub id: Option<String>,

    /// Filter by tree
    #[arg(long)]
    pub tree: Option<String>,

    /// Filter by tree
    #[arg(long)]
    pub keys: Option<Vec<String>>,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct FulltextArgs {
    /// Search query
    #[arg(short, long)]
    pub query: Option<String>,

    /// Language for tokenizer and index
    #[arg(short, long)]
    pub language: Option<String>,

    /// Filter by document name
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    /// Minimum date (ISO 8601)
    #[arg(long)]
    pub min_date: Option<String>,

    /// Maximum date (ISO 8601)
    #[arg(long)]
    pub max_date: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Offset for pagination
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Output format: json, table, text
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,

    /// Filter by Id
    #[arg(long)]
    pub id: Option<String>,

    /// Filter by tree
    #[arg(long)]
    pub tree: Option<String>,

    /// Filter by tree
    #[arg(long)]
    pub keys: Option<Vec<String>>,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct VectorArgs {
    /// Text to search for (will be embedded)
    #[arg(short, long)]
    pub query: String,

    /// Language for tokenizer and index
    #[arg(short, long)]
    pub language: String,

    /// Filter by document name
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    /// Minimum date (ISO 8601)
    #[arg(long)]
    pub min_date: Option<String>,

    /// Maximum results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Offset for pagination
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,

    /// Filter by Id
    #[arg(long)]
    pub id: Option<String>,

    /// Filter by tree
    #[arg(long)]
    pub tree: Option<String>,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct TreeArgs {
    /// Tree path prefix to search
    #[arg(short, long)]
    pub tree: String,

    /// Filter by language
    #[arg(short, long)]
    pub language: Option<String>,

    /// Filter by document name
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    /// Minimum date (ISO 8601)
    #[arg(long)]
    pub min_date: Option<String>,

    /// Maximum results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct KeywordArgs {
    /// Keywords to search for
    #[arg(short, long)]
    pub keys: Vec<String>,

    /// Filter by language
    #[arg(short, long)]
    pub language: Option<String>,

    /// Filter by document name
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    /// Minimum date (ISO 8601)
    #[arg(long)]
    pub min_date: Option<String>,

    /// Maximum results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,

    /// Filter by tree
    #[arg(long)]
    pub tree: Option<String>,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct DocumentArgs {
    /// Document name to search
    #[arg(short = 'n', long = "name")]
    pub document_name: String,

    /// Maximum results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct IdArgs {
    /// Document id to search
    #[arg(long = "id")]
    pub id: String,

    // /// Maximum results
    //#[arg(long, default_value = "10")]
    //pub limit: usize,
    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: OutputFormat,
}

#[derive(clap::ValueEnum, PartialEq, Eq, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Table,
    Text,
    CSV,
}

pub async fn execute(config: CoreConfig, args: SearchArgs) -> Result<()> {
    match args.command {
        SearchCommand::Stem(args) => execute_stem(config, args).await,
        SearchCommand::Fulltext(args) => execute_fulltext(config, args).await,
        SearchCommand::Vector(args) => execute_vector(config, args).await,
        SearchCommand::Tree(args) => execute_tree(config, args).await,
        SearchCommand::Keyword(args) => execute_keywords(config, args).await,
        SearchCommand::Document(args) => execute_document(config, args).await,
        SearchCommand::Id(args) => execute_id(config, args).await,
    }
}

fn parse_date(date_str: Option<String>) -> Result<Option<DateTime<Utc>>> {
    Ok(match date_str {
        Some(s) => Some(DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc)),
        None => None,
    })
}

fn build_params(
    query: Option<String>,
    language: Option<String>,
    name: Option<String>,
    min_date: Option<String>,
    max_date: Option<String>,
    limit: usize,
    offset: usize,
    id: Option<String>,
    tree: Option<String>,
    keys: Option<Vec<String>>,
) -> Result<SearchParams> {
    Ok(SearchParams {
        query_text: query,
        lang: language,
        name: name,
        min_date: parse_date(min_date)?,
        max_date: parse_date(max_date)?,
        limit: limit.min(100),
        offset,
        id,
        tree,
        keys,
    })
}

pub async fn execute_stem(config: CoreConfig, args: StemArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;
    let params = SearchParams {
        query_text: Some(args.query),
        lang: Some(args.language),
        name: args.name,
        min_date: parse_date(args.min_date)?,
        max_date: parse_date(args.max_date)?,
        limit: args.limit.min(100),
        offset: args.offset,
        id: args.id,
        tree: args.tree,
        keys: args.keys,
    };

    let start = Instant::now();
    let results = index.search_stern(&params).await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

pub async fn execute_fulltext(config: CoreConfig, args: FulltextArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;
    let params = build_params(
        args.query,
        args.language,
        args.name,
        args.min_date,
        args.max_date,
        args.limit,
        args.offset,
        args.id,
        args.tree,
        args.keys,
    )?;

    let start = Instant::now();
    let results = index.search_fulltext(&params).await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

pub async fn execute_vector(config: CoreConfig, args: VectorArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;
    let params = SearchParams {
        query_text: Some(args.query.clone()),
        lang: Some(args.language),
        name: args.name,
        min_date: parse_date(args.min_date)?,
        max_date: None,
        limit: args.limit.min(100),
        offset: args.offset.min(0),
        id: args.id,
        tree: args.tree,
        keys: None,
    };

    // Placeholder vector - in reality would use embedding model

    let start = Instant::now();
    let results = index.search_vector(&params).await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

pub async fn execute_tree(config: CoreConfig, args: TreeArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;
    let params = build_params(
        //Some(args.query),
        None,
        args.language,
        args.name,
        args.min_date,
        None,
        args.limit,
        0,
        None,
        Some(args.tree),
        None,
    )?;

    let start = Instant::now();
    let results = index.search_tree(&params).await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

pub async fn execute_keywords(config: CoreConfig, args: KeywordArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;
    let params = build_params(
        None,
        args.language,
        args.name,
        args.min_date,
        None,
        args.limit,
        0,
        None,
        None,
        Some(args.keys),
    )?;

    let start = Instant::now();
    let results = index.search_keywords(&params).await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

pub async fn execute_document(config: CoreConfig, args: DocumentArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;

    let start = Instant::now();
    let results = index
        .search_by_document(&args.document_name, &args.limit.min(100))
        .await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

pub async fn execute_id(config: CoreConfig, args: IdArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;

    let start = Instant::now();
    let results = index.search_by_id(&args.id).await?;
    let elapsed = start.elapsed();

    print_results(&results, args.format, elapsed)?;
    Ok(())
}

fn print_results(
    results: &[tecton_core::search::SearchResult],
    format: OutputFormat,
    elapsed: std::time::Duration,
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(results)?);
        }
        OutputFormat::Table => {
            if results.is_empty() {
                println!("No results found (took {:.2?})", elapsed);
                return Ok(());
            }

            use comfy_table::{
                Attribute,
                Cell, //Color,
                Table,
            };
            let mut table = Table::new();
            table.set_header(vec![
                Cell::new("#").add_attribute(Attribute::Bold),
                Cell::new("ID").add_attribute(Attribute::Bold),
                Cell::new("Document").add_attribute(Attribute::Bold),
                Cell::new("Tree").add_attribute(Attribute::Bold),
                Cell::new("Score").add_attribute(Attribute::Bold),
                Cell::new("Text Preview").add_attribute(Attribute::Bold),
            ]);

            for (i, result) in results.iter().enumerate() {
                let preview = if result.text.len() > 100 {
                    format!("{}...", &result.text[..100])
                } else {
                    result.text.clone()
                };

                table.add_row(vec![
                    Cell::new(i + 1),
                    Cell::new(&result.id[..result.id.len().min(26)]),
                    Cell::new(&result.name),
                    Cell::new(&result.tree),
                    Cell::new(format!("{:.3}", result.score)),
                    Cell::new(preview),
                ]);
            }

            println!("{}", table);
            println!("\nFound {} results in {:.2?}", results.len(), elapsed);
        }
        OutputFormat::Text => {
            for (i, result) in results.iter().enumerate() {
                println!("=== Result {} ===", i + 1);
                println!("ID: {}", result.id);
                println!("Document: {}", result.name);
                println!("Date: {}", result.date.format("%Y-%m-%d %H:%M:%S"));
                println!("Language: {}", result.lang);
                println!("Tree: {}", result.tree);
                println!("Score: {:.3}", result.score);
                println!("Keywords: {}", result.keywords.join(", "));
                println!("Text: {}", result.text);
                println!();
            }
            println!("Found {} results in {:.2?}", results.len(), elapsed);
        }
        OutputFormat::CSV => {
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(b';')
                .quote_style(csv::QuoteStyle::NonNumeric)
                .from_writer(std::io::stdout());
            wtr.write_record([
                "ID", "Document", "Language", "Tree", "Score", "Keywords", "Text",
            ])?;
            // amnual filed are required for keyword reason
            for result in results.iter() {
                wtr.write_record([
                    result.id.to_string(),
                    result.date.format("%Y-%m-%d %H:%M:%S").to_string(),
                    result.lang.to_string(),
                    result.tree.to_string(),
                    result.score.to_string(),
                    result.keywords.join(" "),
                    result.text.to_string(),
                ])?;
            }

            wtr.flush()?;
        }
    }
    Ok(())
}
