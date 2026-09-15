// tecton-cli/src/main.rs
use crate::{
    commands::{Cli, Commands},
    config::load_config,
};
use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
//use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tecton_core::metrics::TB_INDEX_NAME;

mod commands;
mod config;

#[tokio::main]
async fn main() -> Result<()> {
    //let cli = Cli::parse();

    // Initialize tracing
    /*
    let env_filter = if cli.verbose {
        EnvFilter::new("debug,=trace")
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("error,=error"))
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_level(true),
        )
        .init();
        */
    // Handle shell completions
    if let Some(shell) = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<Shell>().ok())
    {
        clap_complete::generate(
            shell,
            &mut Cli::command(),
            TB_INDEX_NAME,
            &mut std::io::stdout(),
        );
        return Ok(());
    }
    let cli = Cli::parse();
    // Load configuration
    let config_path = cli.config.as_deref().map(std::path::PathBuf::from);
    let config = load_config(config_path)?;

    // Execute command
    match cli.command {
        Commands::Index(args) => commands::index::execute(config, args).await,
        Commands::Search(args) => commands::search::execute(config, args).await,
        Commands::UpdateKeywords(args) => commands::update::execute(config, args).await,
        Commands::Admin(args) => commands::admin::execute(config, args).await,
        Commands::Server(args) => commands::server::execute(config, args).await,
    }
}
