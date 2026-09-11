// tecton-cli/src/commands/mod.rs
pub mod admin;
pub mod index;
pub mod search;
pub mod server;
pub mod update;
use clap::{
    // CommandFactory,
    //Parser,
    Subcommand,
};
pub use tecton_core::metrics::TB_INDEX_NAME;
//use core::config::CoreConfig;
//

#[derive(clap::Parser)]
#[command(name = TB_INDEX_NAME)]
#[command(version, about = "Text block indexer and search engine", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Path to config file")]
    pub config: Option<String>,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand, PartialEq, Eq, Clone)]
pub enum Commands {
    /// Index a single text block
    #[command(after_long_help = "\
    EXAMPLES:
      tecton index --text 'Text zum inizieren' --name sample.txt --language de
      tecton index --text 'Text zum inizieren' --name sample.txt --language de --id 01BX5ZZKBKACTAV9WEVGEMMVRZ

    BEHAVIOR:
      Index a text block for a document.")]
    Index(index::IndexArgs),

    /// Search commands
    Search(search::SearchArgs),

    /// Update keywords for a document
    UpdateKeywords(update::UpdateKeywordsArgs),

    /// Administrative commands
    Admin(admin::AdminArgs),

    /// Start web server
    Server(server::ServerArgs),
}
