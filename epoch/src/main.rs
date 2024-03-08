mod config;
mod errors;
mod logger;
mod utils;

use archive_stream::archive::ArchiveAccount;
use archive_stream::archiver::ArchiveCallback;
use archive_stream::stream_archived_accounts;
use clap::Parser;
use config::*;
use errors::*;
use gcs::bucket::*;
use log::*;
use logger::*;
use spacetime_client::SpacetimeClient;
use std::path::PathBuf;
use utils::*;

#[derive(Parser, Debug)]
struct Args {
    /// Path to backfill.yaml config.
    /// Should deserialize into BackfillConfig
    #[arg(long, env, default_value = "backfill.yaml")]
    config_file_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger()?;
    let args: Args = Args::parse();
    info!("Starting with args: {:?}", args);

    let backfill_config = BackfillConfig::read_backfill_config(&args.config_file_path)?;

    let spacetime = SpacetimeClient::new();

    spacetime.run();

    Ok(())
}
