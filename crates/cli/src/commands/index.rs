// tecton-cli/src/commands/index.rs
use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Args;
//use indicatif::{ProgressBar, ProgressStyle};
//use tracing::{info, warn};
use tecton_core::{TectonIndex, config::CoreConfig};

#[derive(Debug, Args, PartialEq, Eq, Clone)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
pub struct IndexArgs {
    /// Text content to index
    #[arg(short, long)]
    pub text: String,

    /// Document name
    #[arg(short = 'n', long = "name")]
    pub document_name: String,

    /// Language code (e.g., en, de, fr)
    #[arg(short, long, default_value = "en")]
    pub language: String,

    /// Tree path (e.g., /chapter1/section2)
    #[arg(long, default_value = "")]
    pub tree: String,

    /// Date in ISO 8601 format (default: now)
    #[arg(short, long)]
    pub date: Option<String>,

    /// Comma-separated keywords (e.g. "first","txt")
    #[arg(short, long, value_delimiter = ',')]
    pub keywords: Vec<String>,

    /// Custom ULID (auto-generated if not provided)
    #[arg(long)]
    pub id: Option<String>,

    /// Commit immediately after indexing
    #[arg(long, default_value = "true")]
    pub commit: bool,
}

pub async fn execute(config: CoreConfig, args: IndexArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;

    let date = if let Some(date_str) = args.date {
        DateTime::parse_from_rfc3339(&date_str)?.with_timezone(&Utc)
    } else {
        Utc::now()
    };

    /*let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}"));
    pb.set_message("Indexing text block...");
    pb.enable_steady_tick(100);*/

    let id = index.add_text_block(
        args.id,
        args.document_name,
        Some(date),
        args.language,
        args.tree,
        args.text,
        // None, // vector
        args.keywords,
    )?;

    if args.commit {
        //   pb.set_message("Committing changes...");
        index.commit()?;
    }

    // pb.finish_with_message("Done!");

    println!("Indexed document with ID: {}", id);

    Ok(())
}
