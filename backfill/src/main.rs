mod config;
mod errors;
mod logger;

use archive_stream::{shorten_address, stream_archived_accounts, AccountCallback, ArchiveAccount};
use async_trait::async_trait;
use clap::Parser;
use config::*;
use errors::*;
use gcs::bucket::*;
use log::*;
use logger::*;
use postgres_client::{DbAccount, PostgresClient};
use solana_sdk::pubkey::Pubkey;
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
    dotenv::dotenv().ok();
    init_logger()?;
    let args: Args = Args::parse();
    info!("Starting with args: {:?}", args);

    let backfill_config = BackfillConfig::read_backfill_config(&args.config_file_path)?;

    let bucket = backfill_config.gcs_bucket;
    let gcs_file = backfill_config.gcs_local_file;
    let metas: Vec<SnapshotMeta> = tokio::spawn(async move {
        info!("Fetching snapshots from GCS, this usually takes 60-90s");
        let metas = match gcs_file {
            None => get_snapshot_metas(&bucket).await,
            Some(path) => get_snapshot_metas_from_local(&path).await,
        }?;
        Result::<_, anyhow::Error>::Ok(metas)
    })
    .await??;

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
    let backfill = Box::leak(Box::new(BackfillHandler {
        programs: backfill_config.programs,
        client: PostgresClient::new_from_url(db_url).await?,
    }));

    match stream_archived_accounts(source, backfill).await {
        Ok(_) => {
            info!("Done!");
            Result::<_, anyhow::Error>::Ok(())
        }
        Err(e) => {
            error!("Error: {}", e);
            Result::<_, anyhow::Error>::Err(e)
        }
    }
}

struct BackfillHandler {
    programs: Vec<Pubkey>,
    client: PostgresClient,
}

#[async_trait]
impl AccountCallback for BackfillHandler {
    async fn callback(&self, account: ArchiveAccount) -> anyhow::Result<()> {
        if self.programs.contains(&account.owner) {
            info!(
                "key: {}, slot: {}, owner: {}",
                &account.key,
                account.slot,
                shorten_address(&account.owner)
            );
            // send to postgres
            let db_account = DbAccount::try_from(account)?;
            self.client.account_upsert(&db_account).await?;
        }
        Ok(())
    }
}
