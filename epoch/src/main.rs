mod config;
mod errors;
mod logger;
mod utils;

use clap::Parser;
use config::*;
use errors::*;
use log::*;
use logger::*;
use std::path::PathBuf;

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
    // let args: Args = Args::parse();
    // info!("Starting with args: {:?}", args);
    // let backfill_config = BackfillConfig::read_backfill_config(&args.config_file_path)?;

    Ok(())
}
