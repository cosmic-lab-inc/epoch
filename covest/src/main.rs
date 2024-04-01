use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use log::*;
use solana_client::rpc_config::RpcRequestAirdropConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use epoch_client::{drift_cpi, program_helpers, EpochClient};
use epoch_client::{init_logger, DecodedEpochAccount, Decoder, QueryDecodedAccounts};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    dotenv::dotenv().ok();
    let rpc_url = "http://localhost:8899".to_string();
    // let signer = Keypair::new();
    let signer = EpochClient::read_keypair_from_env("COVEST")?;
    let key = signer.pubkey();
    let client = Arc::new(EpochClient::new(
        signer,
        rpc_url,
        Some("http://localhost:3333".to_string()),
    ));

    client
        .rpc
        .request_airdrop_with_config(
            &key,
            LAMPORTS_PER_SOL,
            RpcRequestAirdropConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
        )
        .await?;

    client.reset_user().await?;
    let epoch_user = client.connect().await?;
    info!("Epoch user: {:#?}", epoch_user);

    client
        .epoch_airdrop(&epoch_user.api_key, epoch_user.vault)
        .await?;

    let max = client.highest_slot().await?;
    info!("highest slot: {}", max);
    let min = client.lowest_slot().await?;
    info!("lowest slot: {}", min);

    let pre_fetch = Instant::now();
    let mut users = client
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: None,
                slot: Some(max),
                owner: drift_cpi::ID,
                discriminant: "User".to_string(),
                limit: 200_000,
                offset: 0,
            },
        )
        .await?;
    info!(
        "Time to fetch user accounts: {}s",
        pre_fetch.elapsed().as_millis() as f64 / 1000.0
    );

    let pre_sort = Instant::now();
    // sort where highest settled_perp_pnl is first index
    users.sort_by(|a, b| {
        let a = if let Decoder::Drift(drift_cpi::AccountType::User(user)) = &a.decoded {
            user.settled_perp_pnl
        } else {
            0
        };
        let b = if let Decoder::Drift(drift_cpi::AccountType::User(user)) = &b.decoded {
            user.settled_perp_pnl
        } else {
            0
        };
        b.cmp(&a)
    });
    info!(
        "Sorted for top user in {}s",
        pre_sort.elapsed().as_millis() as f64 / 1000.0
    );

    let pre_past_states = Instant::now();
    let top_dog = users
        .first()
        .ok_or(anyhow::anyhow!("No accounts returned"))?
        .clone();
    let top_dog_key = Pubkey::from_str(&top_dog.key)?;
    let mut user_states: Vec<DecodedEpochAccount> = client
        .clone()
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: Some(top_dog_key),
                slot: None,
                owner: drift_cpi::ID,
                discriminant: "User".to_string(),
                limit: 50_000_000,
                offset: 0,
            },
        )
        .await?;
    // sort by highest slot
    user_states.sort_by(|a, b| b.slot.cmp(&a.slot));
    info!(
        "Time to fetch past states: {}s",
        pre_past_states.elapsed().as_millis() as f64 / 1000.0
    );

    info!("States: {}", user_states.len());
    let highest_slot = user_states.first().unwrap().slot;
    let lowest_slot = user_states.last().unwrap().slot;
    let diff = highest_slot - lowest_slot;
    let diff_days = diff / 216_000;
    info!(
        "User history, slots {} - {}, ~days: {}",
        highest_slot, lowest_slot, diff_days
    );

    // filter out duplicates with the same settled_perp_pnl value
    let mut updates: HashMap<i64, DecodedEpochAccount> = HashMap::new();

    for state in user_states.into_iter() {
        if let Decoder::Drift(drift_cpi::AccountType::User(user)) = state.decoded {
            let existing_value = updates.get(&user.settled_perp_pnl);
            if existing_value.is_none() {
                updates.insert(user.settled_perp_pnl, state);
            }
        }
    }

    let mut states: Vec<DecodedEpochAccount> = updates.into_values().collect();
    // sort by slot with highest first
    states.sort_by(|a, b| b.slot.cmp(&a.slot));

    for state in states {
        if let Decoder::Drift(drift_cpi::AccountType::User(user)) = state.decoded {
            info!(
                "slot: {}, pnl: {}",
                state.slot,
                user.settled_perp_pnl as f64 / program_helpers::QUOTE_PRECISION as f64
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn epoch_demo() -> anyhow::Result<()> {
    init_logger();
    dotenv::dotenv().ok();

    // load a keypair from env as a buffer (e.g. [1,2,3,4,5,5,...])
    // make sure this has devnet SOL, if not you can get some here: https://faucet.solana.com/
    let signer = EpochClient::read_keypair_from_env("WALLET")?;
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = Arc::new(EpochClient::new(signer, rpc_url, None));

    // deletes your user from the Epoch database in case you want to start fresh with the same keypair
    client.reset_user().await?;
    client.reset_user().await?;
    // automatically sign up or log in
    // if a new user, this will create a Profile account and Token account on-chain,
    // hence the need for devnet SOL.
    let epoch_user = client.connect().await?;
    println!("{:#?}", epoch_user);

    // on devnet you can airdrop yourself EPOCH tokens to pay for API requests
    client
        .epoch_airdrop(&epoch_user.api_key, epoch_user.vault)
        .await?;

    // finds highest slot (latest point in time) that the Epoch database has stored
    let max = client.highest_slot().await?;
    println!("highest slot: {}", max);
    // finds highest slot (earliest point in time) that the Epoch database has stored
    let min = client.lowest_slot().await?;
    println!("lowest slot: {}", min);

    let pre_fetch = Instant::now();
    let users_to_fetch = 200_000;
    let mut users = client
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: None,                        // Do not filter for a specific account address
                slot: Some(max), // Only fetch accounts at this slot (point in time)
                owner: drift_cpi::ID, // Accounts belong to the Drift program
                discriminant: "User".to_string(), // Only fetch accounts with discriminant "User"
                limit: users_to_fetch, // There are about 150,000 user accounts on Drift, so this is plenty for a specific slot
                offset: 0, // This is used for pagination. If the limit you need is >1M you can use this to fetch in chunks
            },
        )
        .await?;
    println!(
        "Time to fetch user {} accounts: {}s",
        users_to_fetch,
        pre_fetch.elapsed().as_millis() as f64 / 1000.0
    );

    let pre_sort = Instant::now();
    // sort where highest settled_perp_pnl is first index
    users.sort_by(|a, b| {
        let a = if let Decoder::Drift(drift_cpi::AccountType::User(user)) = &a.decoded {
            user.settled_perp_pnl
        } else {
            0
        };
        let b = if let Decoder::Drift(drift_cpi::AccountType::User(user)) = &b.decoded {
            user.settled_perp_pnl
        } else {
            0
        };
        b.cmp(&a)
    });
    println!(
        "Sorted for user with highest total profit in {}s",
        pre_sort.elapsed().as_millis() as f64 / 1000.0
    );

    let pre_past_states = Instant::now();
    let top_dog = users
        .first()
        .ok_or(anyhow::anyhow!("No accounts returned"))?
        .clone();

    // get the most profitable user (the top dog) at every slot possible (up to 50M accounts)
    let top_dog_key = Pubkey::from_str(&top_dog.key)?;
    let mut user_states: Vec<DecodedEpochAccount> = client
        .clone()
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: Some(top_dog_key), // Filters specifically for the top dog's account
                slot: None, // Slot doesn't matter here, we want all slots to reconstruct the user's history
                owner: drift_cpi::ID, // Account belongs to the Drift program
                discriminant: "User".to_string(), // Account is a "User" account
                limit: 50_000_000, // 50M accounts at unique slots is roughly 9 months of history (78M slots per year) assuming the database has it
                offset: 0, // This is used for pagination. If the limit you need is >1M you can use this to fetch in chunks
            },
        )
        .await?;

    // sort by highest slot
    user_states.sort_by(|a, b| b.slot.cmp(&a.slot));
    println!(
        "Time to fetch top dog's history: {}s",
        pre_past_states.elapsed().as_millis() as f64 / 1000.0
    );

    let highest_slot = user_states.first().unwrap().slot;
    let lowest_slot = user_states.last().unwrap().slot;
    let diff_days = highest_slot - lowest_slot / 216_000;
    info!(
        "User history, slots {} - {}, ~days: {}",
        highest_slot, lowest_slot, diff_days
    );

    // filter out duplicates with the same settled_perp_pnl value
    let mut updates: HashMap<i64, DecodedEpochAccount> = HashMap::new();

    for state in user_states.into_iter() {
        if let Decoder::Drift(drift_cpi::AccountType::User(user)) = state.decoded {
            let existing_value = updates.get(&user.settled_perp_pnl);
            if existing_value.is_none() {
                updates.insert(user.settled_perp_pnl, state);
            }
        }
    }

    let mut states: Vec<DecodedEpochAccount> = updates.into_values().collect();
    // sort by highest slot (latest point in time) first
    states.sort_by(|a, b| b.slot.cmp(&a.slot));

    for state in states {
        if let Decoder::Drift(drift_cpi::AccountType::User(user)) = state.decoded {
            println!(
                "slot: {}, USDC profit: {}",
                state.slot,
                user.settled_perp_pnl as f64 / program_helpers::QUOTE_PRECISION as f64
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn epoch_drift_markets_test() -> anyhow::Result<()> {
    init_logger();
    dotenv::dotenv().ok();
    let rpc_url = "http://localhost:8899".to_string();

    let signer = EpochClient::read_keypair_from_env("COVEST")?;
    let key = signer.pubkey();
    let client = Arc::new(EpochClient::new(
        signer,
        rpc_url,
        Some("http://api.epoch.fm".to_string()),
    ));

    client
        .rpc
        .request_airdrop_with_config(
            &key,
            LAMPORTS_PER_SOL,
            RpcRequestAirdropConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
        )
        .await?;

    client.reset_user().await?;
    let epoch_user = client.connect().await?;
    info!("Epoch user: {:#?}", epoch_user);

    client
        .epoch_airdrop(&epoch_user.api_key, epoch_user.vault)
        .await?;

    let max = client.highest_slot().await?;
    info!("highest slot: {}", max);

    let perp_markets = client
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: None,
                slot: Some(max),
                owner: drift_cpi::ID,
                discriminant: "PerpMarket".to_string(),
                limit: 200_000,
                offset: 0,
            },
        )
        .await?;
    for acct in perp_markets {
        if let Decoder::Drift(drift_cpi::AccountType::PerpMarket(market)) = acct.decoded {
            info!(
                "perp: {}, index: {}, spot index: {}",
                program_helpers::Drift::decode_name(&market.name),
                market.market_index,
                market.quote_spot_market_index
            );
        }
    }

    let spot_markets = client
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: None,
                slot: Some(max),
                owner: drift_cpi::ID,
                discriminant: "SpotMarket".to_string(),
                limit: 200_000,
                offset: 0,
            },
        )
        .await?;
    for acct in spot_markets {
        if let Decoder::Drift(drift_cpi::AccountType::SpotMarket(market)) = acct.decoded {
            info!(
                "spot: {}, index: {}",
                program_helpers::Drift::decode_name(&market.name),
                market.market_index
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn solana_drift_markets_test() -> anyhow::Result<()> {
    init_logger();
    dotenv::dotenv().ok();
    let rpc_url = "https://rpc.hellomoon.io/57dbc69d-7e66-4454-b33e-fa6a4b46170f".to_string();
    let signer = EpochClient::read_keypair_from_env("COVEST")?;
    let client = Arc::new(EpochClient::new(
        signer,
        rpc_url,
        Some("http://localhost:3333".to_string()),
    ));

    let perp_markets = program_helpers::Drift::perp_markets(&client.rpc).await?;
    for market in perp_markets {
        info!(
            "perp: {}, index: {}, spot index: {}",
            program_helpers::Drift::decode_name(&market.name),
            market.market_index,
            market.quote_spot_market_index
        );
    }

    let spot_markets = program_helpers::Drift::spot_markets(&client.rpc).await?;
    info!("Spot markets: {}", spot_markets.len());
    for market in spot_markets {
        info!(
            "spot: {}, index: {}",
            program_helpers::Drift::decode_name(&market.name),
            market.market_index
        );
    }

    Ok(())
}
