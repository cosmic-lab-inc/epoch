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
use tokio::spawn;

use common::{Data, Plot};
use epoch_client::{drift_cpi, Env, EpochClient, program_helpers, shorten_address, trunc};
use epoch_client::{DecodedEpochAccount, Decoder, init_logger, QueryDecodedAccounts};
use trader::*;

mod trader;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  init_logger();
  dotenv::dotenv().ok();
  let rpc_url = "http://localhost:8899".to_string();
  let signer = EpochClient::read_keypair_from_env("WALLET")?;
  let key = signer.pubkey();
  let client = Arc::new(EpochClient::new(signer, rpc_url, Env::Dev));

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

  // client.reset_user().await?;
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
  let users = client
    .borsh_decoded_accounts(
      &epoch_user.api_key,
      QueryDecodedAccounts {
        owner: drift_cpi::ID,
        discriminant: "User".to_string(),
        limit: Some(200_000),
        ..Default::default()
      },
    )
    .await?;
  info!(
      "Time to fetch {} user accounts: {}s",
      &users.len(),
      pre_fetch.elapsed().as_millis() as f64 / 1000.0
  );

  // filter out users with same key
  let unique_users: HashMap<String, DecodedEpochAccount> = users
    .into_iter()
    .map(|user| (user.key.clone(), user))
    .collect();
  let mut users: Vec<DecodedEpochAccount> = unique_users.into_values().collect();
  info!("Unique users: {}", users.len());

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
  let users = users
    .into_iter()
    .take(10)
    .collect::<Vec<DecodedEpochAccount>>();

  let pre_past_states = Instant::now();

  let join_handles = users.into_iter().map(|user| {
    let client = client.clone();
    let api_key = epoch_user.api_key.clone();
    spawn(async move {
      let user_states: Vec<DecodedEpochAccount> = client
        .borsh_decoded_accounts(
          &api_key,
          QueryDecodedAccounts {
            key: Some(Pubkey::from_str(&user.key)?),
            owner: drift_cpi::ID,
            discriminant: "User".to_string(),
            limit: Some(100_000),
            ..Default::default()
          },
        )
        .await?;
      debug!(
          "Fetched {} states for user: {}",
          user_states.len(),
          shorten_address(&Pubkey::from_str(&user.key)?)
      );
      // filter out duplicates with the same settled_perp_pnl value
      let mut updates: HashMap<i64, Data> = HashMap::new();
      for state in user_states.into_iter() {
        if let Decoder::Drift(drift_cpi::AccountType::User(user)) = state.decoded {
          let existing_value = updates.get(&user.settled_perp_pnl);
          if existing_value.is_none() {
            updates.insert(
              user.settled_perp_pnl,
              Data {
                x: state.slot as i64,
                y: trunc!(
                    user.settled_perp_pnl as f64/ program_helpers::QUOTE_PRECISION as f64,
                    2
                ),
              },
            );
          }
        }
      }
      let mut data: Vec<Data> = updates.into_values().collect();
      // sort with highest slot first
      data.sort_by(|a, b| b.x.cmp(&a.x));

      Result::<_, anyhow::Error>::Ok(Trader {
        key: Pubkey::from_str(&user.key)?,
        data,
      })
    })
  });
  let mut traders: Vec<Trader> = futures_util::future::join_all(join_handles)
    .await
    .into_iter()
    .flatten()
    .flatten()
    .collect();
  info!("Unfiltered traders: {}", traders.len());
  traders.retain(|trader| trader.avg_trade() > 0.0 && trader.worst_trade() > -30.0);
  info!("Filtered traders: {}", traders.len());

  info!(
      "Time to fetch and sort traders: {}s",
      pre_past_states.elapsed().as_millis() as f64 / 1000.0
  );

  for trader in traders {
    info!(
        "{}, avg: {}%, worst: {}%, total: {}%",
        trader.key,
        trader.avg_trade(),
        trader.worst_trade(),
        trader.total_pct_pnl()
    );

    Plot::plot(
      vec![trader.data],
      &format!("{}/{}.png", env!("CARGO_MANIFEST_DIR"), trader.key),
      &format!("{} Trade History", trader.key),
      "PnL",
    )?;
  }

  Ok(())
}

#[tokio::test]
async fn epoch_drift_markets_test() -> anyhow::Result<()> {
  init_logger();
  dotenv::dotenv().ok();
  let rpc_url = "http://localhost:8899".to_string();

  let signer = EpochClient::read_keypair_from_env("WALLET")?;
  let key = signer.pubkey();
  let client = Arc::new(EpochClient::new(signer, rpc_url, Env::Dev));

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
        slot: Some(max),
        owner: drift_cpi::ID,
        discriminant: "PerpMarket".to_string(),
        limit: Some(200_000),
        offset: None,
        key: None,
        min_slot: None,
        max_slot: None,
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
        slot: Some(max),
        owner: drift_cpi::ID,
        discriminant: "SpotMarket".to_string(),
        limit: Some(200_000),
        offset: None,
        key: None,
        min_slot: None,
        max_slot: None,
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
  let signer = EpochClient::read_keypair_from_env("WALLET")?;
  let client = Arc::new(EpochClient::new(signer, rpc_url, Env::Dev));

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
