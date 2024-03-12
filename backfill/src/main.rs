mod config;
mod errors;
mod logger;

use archive_stream::stream_archived_accounts;
use clap::Parser;
use common::ArchiveAccount;
use config::*;
use gcs::bq::{BigQueryClient, BqAccount};
use gcs::bucket::*;
use log::*;
use logger::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser, Debug)]
struct Args {
    /// Path to backfill.yaml config.
    /// Should deserialize into BackfillConfig
    #[arg(long, env, default_value = "backfill.yaml")]
    config_file_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    init_logger()?;
    let args: Args = Args::parse();
    info!("Starting with args: {:?}", args);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()?;

    let backfill_config = BackfillConfig::read_config(&args.config_file_path)?;

    let client = rt.block_on(async move {
        let client = BigQueryClient::new(Path::new(&backfill_config.gcs_sa_key)).await?;
        Result::<_, anyhow::Error>::Ok(client)
    })?;

    let bucket = backfill_config.gcs_bucket;
    let metas: Vec<SnapshotMeta> = rt.block_on(async move {
        info!("Fetching snapshots from GCS, this usually takes 60-90s");
        let metas = match &backfill_config.gcs_local_file {
            Some(path) => match Path::new(path).exists() {
                false => get_snapshot_metas(&bucket).await,
                true => get_snapshot_metas_from_local(path).await,
            },
            None => get_snapshot_metas(&bucket).await,
        }?;
        Result::<_, anyhow::Error>::Ok(metas)
    })?;
    info!(
        "GCS snapshots found: {} - {}",
        metas.first().unwrap().snapshot.slot,
        metas.last().unwrap().snapshot.slot
    );

    let earliest_snapshot = *backfill_config.slots.iter().min().unwrap();
    // slice metas after earliest snapshot
    let metas: Vec<_> = metas
        .into_iter()
        .filter(|m| m.snapshot.slot >= earliest_snapshot)
        .collect();

    let first = metas.first().unwrap();
    let last = metas.last().unwrap();
    info!(
        "desired snapshot range: {} - {}",
        first.snapshot.slot, last.snapshot.slot
    );

    // get accounts from the earliest snapshot
    let snapshot_meta = metas.first().unwrap().clone();
    let source = snapshot_meta.snapshot.url;

    let (tx, rx) = crossbeam_channel::unbounded::<ArchiveAccount>();

    let programs = Arc::new(backfill_config.programs);
    rt.spawn(async move {
        let programs = programs.clone();

        const BUFFER_SIZE: usize = 500;
        let mut buffer = Vec::new();

        while let Ok(account) = rx.recv() {
            if programs.contains(&account.owner) {
                // send to BigQuery
                let bq_account = match BqAccount::try_from(account) {
                    Ok(db_account) => db_account,
                    Err(e) => {
                        error!("Error converting account to BqAccount: {:?}", e);
                        return;
                    }
                };
                if buffer.len() == BUFFER_SIZE {
                    match client.upsert_accounts(std::mem::take(&mut buffer)).await {
                        Ok(_row) => {
                            info!("Upserted {} accounts", BUFFER_SIZE);
                        }
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                } else {
                    buffer.push(bq_account);
                }
            }
        }
    });

    stream_archived_accounts(source, tx)
}
