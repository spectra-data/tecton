// tecton-cli/src/commands/admin.rs
use anyhow::Result;
use clap::{Args, Subcommand};

use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
//use toml::ser::to_string_pretty;
//use std::time::Duration;
//use tracing::info;
use tecton_core::{
    TectonIndex,
    config::{
        CoreConfig, //, IndexConfig
    },
};

use ulid::Ulid;

#[derive(Args, PartialEq, Eq, Clone)]
pub struct AdminArgs {
    #[command(subcommand)]
    pub command: AdminCommand,
}

#[derive(Subcommand, PartialEq, Eq, Clone)]
pub enum AdminCommand {
    /// Create a new empty index
    Create(CreateArgs),

    /// Compact/optimize the index
    Compact(CompactArgs),

    /// Show index statistics
    Stats(StatsArgs),

    /// Delete an document by the matching id
    Delete(DeleteArgs),

    /// Backup index
    Backup(BackupArgs),

    /// Restore index from backup
    Restore(RestoreArgs),

    /// Print a toml config file with default values
    TomlConfig(NoArgs),
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct NoArgs {}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct CreateArgs {
    /// Index path
    #[arg(short, long, default_value = "./index")]
    pub path: PathBuf,

    /// Force overwrite if exists
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct CompactArgs {
    /// Index path (optional, uses config if not provided)
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct StatsArgs {
    /// Index path (optional, uses config if not provided)
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct DeleteArgs {
    /// Document ULID id to delete
    #[arg(long)]
    pub id: String,

    /// Confirm deletion
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct BackupArgs {
    /// Backup output directory
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, PartialEq, Eq, Clone)]
pub struct RestoreArgs {
    /// Backup directory to restore from
    #[arg(short, long)]
    pub input: PathBuf,

    /// Target index path
    #[arg(short, long)]
    pub target: PathBuf,
}

pub async fn execute(config: CoreConfig, args: AdminArgs) -> Result<()> {
    match args.command {
        AdminCommand::Create(args) => execute_create(config, args).await,
        AdminCommand::Compact(args) => execute_compact(config, args).await,
        AdminCommand::Stats(args) => execute_stats(config, args).await,
        AdminCommand::Delete(args) => execute_delete(config, args).await,
        AdminCommand::Backup(args) => execute_backup(config, args).await,
        AdminCommand::Restore(args) => execute_restore(config, args).await,
        AdminCommand::TomlConfig(_args) => execute_sample_config(config).await,
    }
}

async fn execute_create(config: CoreConfig, args: CreateArgs) -> Result<()> {
    if args.path.exists() && !args.force {
        anyhow::bail!(
            "Index already exists at {}. Use --force to overwrite.",
            args.path.display()
        );
    }

    if args.path.exists() {
        std::fs::remove_dir_all(&args.path)?;
    }

    let mut index_config = config.index.clone();
    index_config.index_path = args.path.clone();

    let index = TectonIndex::create(CoreConfig {
        index: index_config,
        ..config
    })?;

    println!("Created new index at: {}", args.path.display());
    println!("Documents: {}", index.num_docs());

    Ok(())
}

async fn execute_compact(config: CoreConfig, args: CompactArgs) -> Result<()> {
    let index = if let Some(path) = args.path {
        let mut idx_config = config.index.clone();
        idx_config.index_path = path;
        TectonIndex::open(CoreConfig {
            index: idx_config,
            ..config
        })?
    } else {
        TectonIndex::new(config)?
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}"));
    pb.set_message("Compacting index...");
    pb.enable_steady_tick(100);

    index.compact()?;

    pb.finish_with_message("Compaction complete!");

    println!("Index compacted. Documents: {}", index.num_docs());

    Ok(())
}

async fn execute_stats(config: CoreConfig, args: StatsArgs) -> Result<()> {
    let index = if let Some(path) = args.path {
        let mut idx_config = config.index.clone();
        idx_config.index_path = path;
        TectonIndex::open(CoreConfig {
            index: idx_config,
            ..config
        })?
    } else {
        TectonIndex::new(config)?
    };

    use comfy_table::{Attribute, Cell, Table};
    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Property").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);

    table.add_row(vec![Cell::new("Documents"), Cell::new(index.num_docs())]);
    table.add_row(vec![
        Cell::new("Index Path"),
        Cell::new(index.config().index_path.display()),
    ]);
    table.add_row(vec![
        Cell::new("Commit Interval"),
        Cell::new(format!("{}ms", index.config().commit_interval_ms)),
    ]);
    table.add_row(vec![
        Cell::new("Max Threads"),
        Cell::new(index.config().max_threads),
    ]);
    table.add_row(vec![
        Cell::new("Vector Index Enabled"),
        Cell::new(index.config().enable_hnsw),
    ]);
    table.add_row(vec![
        Cell::new("Stemming Enabled"),
        Cell::new(index.config().enable_steam),
    ]);

    // Calculate index size
    let index_size =
        calculate_dir_size(&index.config().index_path.join(&index.config().tantivy_dir))?;
    table.add_row(vec![
        Cell::new("Tantivy Index Size"),
        Cell::new(format_bytes(index_size)),
    ]);

    let hnsw_index_size: u64 = if index.config().enable_hnsw {
        // Calculate hsnw index size
        calculate_dir_size(
            &index
                .config()
                .index_path
                .join(&index.config().hnsw.hnsw_dir),
        )?
    } else {
        0
    };
    table.add_row(vec![
        Cell::new("Hnsw Index Size"),
        Cell::new(format_bytes(hnsw_index_size)),
    ]);

    println!("{}", table);

    Ok(())
}

async fn execute_delete(config: CoreConfig, args: DeleteArgs) -> Result<()> {
    if !args.confirm {
        anyhow::bail!("Deletion requires --confirm flag");
    }

    let _ulid = match Ulid::from_string(args.id.as_str()) {
        Ok(u) => u,
        Err(_err) => anyhow::bail!("Invalid ULID"),
    };

    let index = TectonIndex::new(config)?;
    match index.delete_document(args.id.as_str()) {
        Ok(_) => println!("Block with id {} deleted!", args.id),
        Err(e) => anyhow::bail!(e),
    }
    Ok(())
}

async fn execute_backup(config: CoreConfig, args: BackupArgs) -> Result<()> {
    let index = TectonIndex::new(config)?;
    let source = &index.config().index_path;

    std::fs::create_dir_all(&args.output)?;

    // Copy index directory
    copy_dir_all(source, &args.output)?;

    println!("Index backed up to: {}", args.output.display());

    Ok(())
}

async fn execute_restore(_config: CoreConfig, args: RestoreArgs) -> Result<()> {
    if args.target.exists() {
        anyhow::bail!("Target path already exists: {}", args.target.display());
    }

    copy_dir_all(&args.input, &args.target)?;

    println!(
        "Index restored from {} to {}",
        args.input.display(),
        args.target.display()
    );

    Ok(())
}

async fn execute_sample_config(_config: CoreConfig) -> Result<()> {
    // Strictly the hardcoded defaults, zero runtime influence
    let core_defaults = CoreConfig::default();
    println!(
        "# Default tectonx configuration\n{}",
        toml::to_string_pretty(&core_defaults).unwrap()
    );

    Ok(())
}

fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}
