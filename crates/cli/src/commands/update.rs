// ttecton-cli/src/commands/update.rs
use anyhow::Result;
use clap::Args;
//use tracing::info;
use tecton_core::{TectonIndex, config::CoreConfig};

#[derive(Args, PartialEq, Eq, Clone)]
pub struct UpdateKeywordsArgs {
    /// Document ULID
    #[arg(short, long)]
    pub id: String,

    /// Keywords to add/replace (comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    pub keywords: Vec<String>,

    /// Action: add (append) or replace
    #[arg(short, long, default_value = "add", value_enum)]
    pub action: KeywordAction,

    /// Commit immediately
    #[arg(long, default_value = "true")]
    pub commit: bool,
}

#[derive(clap::ValueEnum, PartialEq, Eq, Clone, Debug)]
pub enum KeywordAction {
    Add,
    Replace,
}

pub async fn execute(config: CoreConfig, args: UpdateKeywordsArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;

    let replace = matches!(args.action, KeywordAction::Replace);

    index.update_keywords(&args.id, args.keywords, replace)?;

    if args.commit {
        index.commit()?;
    }

    println!("Updated keywords for document: {}", args.id);

    Ok(())
}
