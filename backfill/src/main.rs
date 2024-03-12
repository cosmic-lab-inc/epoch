mod config;
mod errors;
mod logger;

use archive_stream::{shorten_address, stream_archived_accounts, ArchiveAccount};
use clap::Parser;
use config::*;
use errors::*;
use gcs::bucket::*;
use log::*;
use logger::*;
use postgres_client::{DbAccount, PostgresClient};
use std::path::PathBuf;
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

    let backfill_config = BackfillConfig::read_backfill_config(&args.config_file_path)?;

    let bucket = backfill_config.gcs_bucket;
    let gcs_file = backfill_config.gcs_local_file;
    let metas: Vec<SnapshotMeta> = rt.block_on(async move {
        info!("Fetching snapshots from GCS, this usually takes 60-90s");
        let metas = match gcs_file {
            None => get_snapshot_metas(&bucket).await,
            Some(path) => get_snapshot_metas_from_local(&path).await,
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

    let db_url = std::env::var("DATABASE_URL")?;
    let client = rt.block_on(PostgresClient::new_from_url(db_url))?;

    let (tx, rx) = crossbeam_channel::unbounded::<ArchiveAccount>();

    let programs = Arc::new(backfill_config.programs);
    rt.spawn(async move {
        let programs = programs.clone();
        while let Ok(account) = rx.recv() {
            if programs.contains(&account.owner) {
                let msg = format!(
                    "key: {}, slot: {}, owner: {}",
                    &account.key,
                    account.slot,
                    shorten_address(&account.owner)
                );
                // send to postgres
                let db_account = match DbAccount::try_from(account) {
                    Ok(db_account) => db_account,
                    Err(e) => {
                        error!("Error converting account to DbAccount: {:?}", e);
                        return;
                    }
                };
                match client.account_upsert(&db_account).await {
                    Ok(_row) => {
                        info!("Upserted account: {:?}", msg);
                    }
                    Err(e) => {
                        error!("Error upserting account: {:?}", e);
                    }
                }
            }
        }
    });

    stream_archived_accounts(source, tx)
}
