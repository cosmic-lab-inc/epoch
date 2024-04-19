use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use clap::Parser;
use log::*;

use common::{ArchiveAccount, init_logger};
use config::*;
use gcs::bucket::*;
use snapshot::stream_archived_accounts;
use timescale_client::{TimescaleAccount, TimescaleClient};

mod config;
mod errors;

#[derive(Parser, Debug)]
struct Args {
  /// Path to backfill.yaml config.
  /// Should deserialize into BackfillConfig
  #[arg(long, env, default_value = "backfill.yaml")]
  config_file_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
  dotenv::dotenv().ok();
  init_logger();
  let args: Args = Args::parse();
  info!("🏗️ Backfill snapshots with args: {:?}", args);

  let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)
    .enable_all()
    .build()?;

  let backfill_config = BackfillConfig::read_config(&args.config_file_path)?;

  let mut timescale = rt.block_on(async move {
    TimescaleClient::new_from_url(backfill_config.timescale_db).await
  })?;

  let bucket = backfill_config.gcs_bucket;
  let metas: Vec<SnapshotMeta> = rt.block_on(async move {
    let metas = match &backfill_config.gcs_local_file {
      Some(path) => match Path::new(path).exists() {
        false => get_snapshot_metas(GcsObjectsSource::Url(bucket)).await,
        true => get_snapshot_metas(GcsObjectsSource::Path(path.clone())).await,
      },
      None => get_snapshot_metas(GcsObjectsSource::Url(bucket)).await,
    }?;
    Result::<_, anyhow::Error>::Ok(metas)
  })?;
  info!(
      "GCS snapshots found: {} - {}",
      metas.first().unwrap().snapshot.slot,
      metas.last().unwrap().snapshot.slot
  );

  // slice SnapshotMetas to range config wants to backfill
  let start = backfill_config.backfill_start_date;
  let end = backfill_config.backfill_end_date;
  let metas: Vec<_> = metas
    .into_iter()
    .filter(|m| m.snapshot.time_created >= start && m.snapshot.time_created <= end)
    .collect();
  info!(
      "Snapshot date range to backfill: {} - {}",
      metas.first().unwrap().datetime(),
      metas.last().unwrap().datetime()
  );

  let (tx, rx) = crossbeam_channel::unbounded::<ArchiveAccount>();
  let programs = Arc::new(backfill_config.programs);
  rt.spawn(async move {
    let programs = programs.clone();

    const BUFFER_SIZE: usize = 500;
    let mut buffer = Vec::new();

    while let Ok(account) = rx.recv() {
      if programs.contains(&account.owner) {
        // send to Timescale
        let timescale_acct = TimescaleAccount::new(account);

        if buffer.len() == BUFFER_SIZE {
          match timescale.upsert_accounts(std::mem::take(&mut buffer)).await {
            Ok(_) => {
              debug!("Upserted {} accounts", BUFFER_SIZE);
            }
            Err(e) => {
              error!("{:?}", e);
            }
          }
        } else {
          buffer.push(timescale_acct);
        }
      }
    }
  });

  // backfill from the most recent date to the oldest
  let sender = Arc::new(tx);
  for meta in metas.into_iter().rev() {
    let source = meta.snapshot.url.clone();
    info!("Backfilling snapshot: {:#?}", &meta.datetime());
    match stream_archived_accounts(source, sender.clone()) {
      Ok(_) => {
        info!(
            "Done snapshot {} for slot {}",
            &meta.datetime(),
            &meta.snapshot.slot
        );
      }
      Err(e) => {
        error!("Error backfilling snapshot: {}", e);
      }
    }
  }
  Ok(())
}
