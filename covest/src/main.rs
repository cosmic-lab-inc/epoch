use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use log::*;
use plotters::prelude::*;
use solana_client::rpc_config::RpcRequestAirdropConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use tokio::spawn;

use epoch_client::{drift_cpi, program_helpers, shorten_address, trunc, Env, EpochClient};
use epoch_client::{init_logger, DecodedEpochAccount, Decoder, QueryDecodedAccounts};
use plot::*;
use trader::*;

mod plot;
mod trader;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    dotenv::dotenv().ok();
    let rpc_url = "http://localhost:8899".to_string();
    let signer = EpochClient::read_keypair_from_env("COVEST")?;
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
    // let user_join_handles = (0..300).map(|i| {
    //     let client = client.clone();
    //     let api_key = epoch_user.api_key.clone();
    //     std::thread::sleep(std::time::Duration::from_millis(100)); // prevents rate limiting
    //     spawn(async move {
    //         let users = client
    //             .borsh_decoded_accounts(
    //                 &api_key,
    //                 QueryDecodedAccounts {
    //                     slot: Some(252_600_000 + i * 500),
    //                     owner: drift_cpi::ID,
    //                     discriminant: "User".to_string(),
    //                     limit: Some(20_000_000),
    //                     ..Default::default()
    //                 },
    //             )
    //             .await?;
    //         Result::<_, anyhow::Error>::Ok(users)
    //     })
    // });
    // let users: Vec<DecodedEpochAccount> = futures_util::future::join_all(user_join_handles)
    //     .await
    //     .into_iter()
    //     .flatten()
    //     .flatten()
    //     .flatten()
    //     .collect();

    let users = client
        .borsh_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                min_slot: Some(253_000_000),
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
        .take(1000)
        .collect::<Vec<DecodedEpochAccount>>();

    let pre_past_states = Instant::now();

    let join_handles = users.into_iter().map(|user| {
        let client = client.clone();
        let api_key = epoch_user.api_key.clone();
        std::thread::sleep(std::time::Duration::from_millis(100)); // prevents rate limiting
        spawn(async move {
            let user_states: Vec<DecodedEpochAccount> = client
                .borsh_decoded_accounts(
                    &api_key,
                    QueryDecodedAccounts {
                        key: Some(Pubkey::from_str(&user.key)?),
                        owner: drift_cpi::ID,
                        discriminant: "User".to_string(),
                        limit: Some(50_000_000),
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
                                x: state.slot,
                                y: trunc!(
                                    user.settled_perp_pnl as f64
                                        / program_helpers::QUOTE_PRECISION as f64,
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
    traders.retain(|trader| trader.avg_trade() > 0.0 && trader.worst_trade() > -50.0);
    info!("Filtered traders: {}", traders.len());

    info!(
        "Time to fetch and sort traders: {}s",
        pre_past_states.elapsed().as_millis() as f64 / 1000.0
    );

    // find lowest x value in all traders
    let mut min_x = u64::MAX;
    let mut max_x = 0;
    let mut min_y = f64::MAX;
    let mut max_y = 0.0;
    for trader in traders.iter() {
        for data in trader.data.iter() {
            if data.x < min_x {
                min_x = data.x;
            }
            if data.x > max_x {
                max_x = data.x;
            }
            if data.y < min_y {
                min_y = data.y;
            }
            if data.y > max_y {
                max_y = data.y;
            }
        }
    }

    for trader in traders {
        info!(
            "{}, avg: {}%, worst: {}%, total: {}%",
            trader.key,
            trader.avg_trade(),
            trader.worst_trade(),
            trader.total_pct_pnl()
        );

        let out_file = &format!("{}/{}.png", env!("CARGO_MANIFEST_DIR"), trader.key);
        let root = BitMapBackend::new(out_file, (2048, 1024)).into_drawing_area();
        root.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&root)
            .margin_top(20)
            .margin_bottom(20)
            .margin_left(30)
            .margin_right(30)
            .set_all_label_area_size(130)
            .caption(
                format!("{} Trade History", trader.key),
                ("sans-serif", 40.0).into_font(),
            )
            .build_cartesian_2d(min_x..max_x, min_y..max_y)?;
        chart
            .configure_mesh()
            .light_line_style(WHITE)
            .label_style(("sans-serif", 30, &BLACK).into_text_style(&root))
            .x_desc("Slot")
            .y_desc("PnL")
            .draw()?;

        // get color from colors array
        let color = Plot::random_color();

        let series = &trader.data;
        chart.draw_series(
            LineSeries::new(
                series.iter().map(|data| (data.x, data.y)),
                ShapeStyle {
                    color,
                    filled: true,
                    stroke_width: 2,
                },
            )
            .point_size(3),
        )?;

        root.present()?;
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
    let signer = EpochClient::read_keypair_from_env("COVEST")?;
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
