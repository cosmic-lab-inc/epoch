use anyhow::Result;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::account::Account;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use common::init_logger;
use decoder::user::User;
use decoder::{Archive, DriftArchive, TraderStats, DRIFT_PROGRAM_ID};

fn devnet_connection() -> RpcClient {
    RpcClient::new_with_commitment(
        "https://devnet.helius-rpc.com/?api-key=0b810c4e-acb6-49a3-b2cd-90e671480ca8".to_string(),
        // "https://radial-frequent-silence.solana-devnet.quiknode.pro/6254b51f5853c6699156c8bcafea3ff72085a05d/".to_string(),
        CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        },
    )
}

fn mainnet_connection() -> RpcClient {
    RpcClient::new_with_commitment(
        "https://mainnet.helius-rpc.com/?api-key=0b810c4e-acb6-49a3-b2cd-90e671480ca8".to_string(),
        // "https://rpc.hellomoon.io/57dbc69d-7e66-4454-b33e-fa6a4b46170f".to_string(),
        CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        },
    )
}

fn public_mainnet_connection() -> RpcClient {
    RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com".to_string(),
        CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        },
    )
}

#[allow(dead_code)]
fn devnet_drift() -> Result<DriftArchive> {
    DriftArchive::new(Pubkey::from_str(DRIFT_PROGRAM_ID)?, devnet_connection())
}

#[allow(dead_code)]
fn drift() -> Result<DriftArchive> {
    DriftArchive::new(Pubkey::from_str(DRIFT_PROGRAM_ID)?, mainnet_connection())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_user_accounts() -> Result<()> {
    init_logger();
    let drift = devnet_drift()?;
    let users: Vec<(Pubkey, Account)> = drift.user_accounts().await?;
    info!("users: {:?}", users.len());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_deserialize_user_account() -> Result<()> {
    init_logger();
    let drift = devnet_drift()?;
    let users: Vec<(Pubkey, Account)> = drift.user_accounts().await?;
    let first = users.first().unwrap();
    let user = Archive::deserialize_account::<User>(&first.1.data)?;
    info!("deser user: {:#?}", user);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_trader_stats_by_roi() -> Result<()> {
    init_logger();
    let drift = drift()?;
    let best_traders = drift.top_trader_stats_by_roi().await?;
    let list = best_traders
        .into_iter()
        .take(10)
        .collect::<Vec<TraderStats>>();
    // write to json
    let file = std::fs::File::create("top_traders_by_roi.json")?;
    serde_json::to_writer_pretty(file, &list)?;
    info!("Wrote to top_traders_by_roi.json");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_top_trader_history() -> Result<()> {
    init_logger();
    let drift = drift()?;
    let best_traders = drift.top_traders_by_roi().await?;
    let best = best_traders
        .first()
        .ok_or_else(|| anyhow::anyhow!("No traders"))?;
    let _ = drift.trader_history(best, None, None).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_top_trader_history_stats() -> Result<()> {
    init_logger();
    let drift = devnet_drift()?;
    let best_traders = drift.top_traders_by_roi().await?;
    let best = best_traders
        .first()
        .ok_or_else(|| anyhow::anyhow!("No traders"))?;
    let stats = drift
        .trader_history_stats(best, Some(216_000 * 2), Some(90))
        .await?;
    let file = std::fs::File::create("top_trader_history.json")?;
    serde_json::to_writer_pretty(file, &stats)?;
    info!("Wrote to top_trader_history.json");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_historical_signatures() -> Result<()> {
    init_logger();
    let drift = devnet_drift()?;
    let key = Pubkey::from_str("3RKsm6oNoWWuQLZ6ue59AZTmrtq6KgSjCLBgb5kUv25a")?;

    let mut sigs = drift
        .archive
        .get_chunked_signatures_for_address(Some(&key), Some(100_000))
        .await?;
    // sort by block_time where highest is first
    sigs.sort_by(|a, b| b.block_time.cmp(&a.block_time));
    let oldest_sig = sigs.last().unwrap();

    let info = Archive::time_since_signature(oldest_sig.clone())?;
    info!(
        "oldest sig: {}, age: {:#?}",
        info.ctx.signature, info.formatted_time_since
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_historical_account_state() -> Result<()> {
    init_logger();
    let conn = public_mainnet_connection();
    let key = Pubkey::from_str(DRIFT_PROGRAM_ID)?;

    let curr_slot = conn.get_slot().await?;
    let past_slot = curr_slot - 216_000 * 10;
    let cfg = RpcAccountInfoConfig {
        min_context_slot: Some(past_slot),
        commitment: Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
        ..Default::default()
    };

    let res = conn.get_account_with_config(&key, cfg).await?;
    let ctx = res.context.slot;
    info!(
        "curr slot: {}, past slot: {}, account slot: {}",
        curr_slot, past_slot, ctx
    );

    Ok(())
}
