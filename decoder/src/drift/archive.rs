use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use std::vec::Vec;

use crate::archive::Archive;
use crate::drift::program::user::{User, UserStats};
use crate::drift::snapshot::DriftTraderSnapshot;
use crate::drift::trader::{Authority, DriftTrader};
use crate::drift::TraderStats;
use anyhow::Result;
use common::trunc;
use common::KeyedAccount;
use futures::stream::{self, StreamExt};
use log::*;
use rayon::prelude::*;
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;

pub const DRIFT_PROGRAM_ID: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";

pub struct DriftArchive {
    pub program_id: Pubkey,
    pub connection: RpcClient,
    pub archive: Archive,
}

impl DriftArchive {
    pub fn new(program_id: Pubkey, connection: RpcClient) -> Result<Self> {
        let cc = RpcClient::new_with_commitment(connection.url(), connection.commitment());
        Ok(Self {
            program_id,
            connection,
            archive: Archive::new(program_id, cc),
        })
    }

    #[allow(dead_code)]
    fn idl_json() -> Result<String> {
        let idl_path = format!("{}{}", env::current_dir()?.display(), "/src/drift/idl.json");
        let path = Path::new(&idl_path);
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                info!("Failed to open Drift idl.json: {:?}", e);
                return Err(e.into());
            }
        };
        let mut idl_json = String::new();
        file.read_to_string(&mut idl_json)?;
        Ok(idl_json)
    }

    pub async fn user_accounts(&self) -> Result<Vec<(Pubkey, Account)>> {
        let discrim = Archive::account_discriminator("User");
        let memcmp = Memcmp::new_base58_encoded(0, discrim.to_vec().as_slice());

        let filters = vec![RpcFilterType::Memcmp(memcmp)];

        let account_config = RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..Default::default()
        };

        let accounts = self
            .connection
            .get_program_accounts_with_config(
                &self.program_id,
                RpcProgramAccountsConfig {
                    filters: Some(filters),
                    account_config,
                    ..Default::default()
                },
            )
            .await?;
        Ok(accounts)
    }

    pub fn decode_name(name: &[u8; 32]) -> String {
        String::from_utf8(name.to_vec()).unwrap().trim().to_string()
    }

    pub fn user_stats_pda(&self, user_authority: &Pubkey) -> Pubkey {
        let seeds: &[&[u8]] = &[b"user_stats", &user_authority.to_bytes()[..]];
        Pubkey::find_program_address(seeds, &self.program_id).0
    }

    pub async fn user_stats(
        &self,
        user_auths: &[&Authority],
    ) -> Result<Vec<KeyedAccount<UserStats>>> {
        let pdas = user_auths
            .iter()
            .map(|k| self.user_stats_pda(k))
            .collect::<Vec<Pubkey>>();

        let account_infos = self.archive.get_chunked_account_infos(&pdas).await?;
        let user_stats: Vec<KeyedAccount<UserStats>> = account_infos
            .into_iter()
            .filter_map(|keyed_account| {
                let result = Archive::deserialize_account::<UserStats>(
                    keyed_account.account.data.as_slice(),
                )
                .copied();
                match result {
                    Err(_) => None,
                    Ok(account) => Some(KeyedAccount {
                        key: keyed_account.key,
                        account,
                    }),
                }
            })
            .collect();

        Ok(user_stats)
    }

    /// Gets all Drift users, sorts by highest ROI (perp pnl / deposits), and takes top 1,000 users.
    /// Fetches those 1,000 users' [`UserStats`] accounts to derive "PnL to volume ratio",
    /// and filters out users who have not traded in the last 30 days.
    /// Since one authority can have many User accounts, we map all User accounts to each authority and return.
    pub async fn top_traders(&self) -> Result<HashMap<Authority, DriftTrader>> {
        let start = Instant::now();
        let user_accounts: Vec<(Pubkey, Account)> = self.user_accounts().await?;
        let end = Instant::now();
        info!(
            "Fetched Drift {} users in {}s",
            &user_accounts.len(),
            trunc!(end.duration_since(start).as_secs_f64(), 2)
        );

        // chunk user_accounts into 1000 accounts per chunk
        let chunked_accounts: Vec<&[(Pubkey, Account)]> =
            user_accounts.par_chunks(20_000).collect();

        // par iter over chunked accounts
        let mut deser_users: Vec<KeyedAccount<User>> = chunked_accounts
            .into_par_iter()
            .map(|chunk| {
                chunk.par_iter().filter_map(|u| {
                    match Archive::deserialize_account::<User>(u.1.data.as_slice()) {
                        Ok(user) => match user.idle {
                            false => Some(KeyedAccount {
                                key: u.0,
                                account: *user,
                            }),
                            true => None,
                        },
                        Err(_) => None,
                    }
                })
            })
            .flatten()
            .collect::<Vec<KeyedAccount<User>>>();
        // sort where highest roi is first index
        deser_users.par_sort_by(|a, b| b.account.roi().partial_cmp(&a.account.roi()).unwrap());

        // filter 10_000 highest roi users before fetching UserStats from RPC, 150k accounts is too much...
        let top_users: Vec<KeyedAccount<User>> = deser_users.into_iter().take(1_000).collect();

        // map all User accounts to each authority
        let mut user_auths = HashMap::<Authority, Vec<KeyedAccount<User>>>::new();
        top_users
            .into_iter()
            .for_each(|u| match user_auths.get_mut(&u.account.authority) {
                Some(users) => {
                    users.push(u);
                }
                None => {
                    user_auths.insert(u.account.authority, vec![u]);
                }
            });

        // get UserStats account for each authority
        let auths = user_auths.keys().collect::<Vec<&Authority>>();
        let user_stats = self.user_stats(auths.as_slice()).await?;

        // UserStat account is PDA of authority pubkey, so there's only ever 1:1.
        // There is never a case when traders HashMap has an existing entry that needs to be updated.
        // Therefore, insert (which overwrites) is safe.
        let mut traders = HashMap::<Authority, DriftTrader>::new();
        user_stats
            .into_iter()
            // filter traders who have traded in the last 30 days
            .filter(|us| us.account.taker_volume_30d > 0 && us.account.maker_volume_30d > 0)
            .for_each(|us| {
                let users: Vec<KeyedAccount<User>> =
                    user_auths.remove(&us.account.authority).unwrap_or_default();
                let key = us.account.authority;
                let trader = DriftTrader {
                    authority: us.account.authority,
                    user_stats: us,
                    users,
                };
                traders.insert(key, trader);
            });
        Ok(traders)
    }

    /// Top perp traders, sorted by ROI as a ratio of settled perp pnl to total deposits.
    pub async fn top_traders_by_roi(&self) -> Result<Vec<DriftTrader>> {
        let traders_map = self.top_traders().await?;
        let mut traders = traders_map.into_values().collect::<Vec<DriftTrader>>();
        traders.sort_by(|a, b| b.roi().partial_cmp(&a.roi()).unwrap());
        Ok(traders)
    }
    /// Formatted into [`TraderStats`] struct for easy display and less memory usage.
    pub async fn top_trader_stats_by_roi(&self) -> Result<Vec<TraderStats>> {
        let best_traders = self.top_traders_by_roi().await?;
        let mut trader_stats: Vec<TraderStats> =
            best_traders.into_iter().map(TraderStats::from).collect();
        trader_stats.sort_by(|a, b| b.roi.partial_cmp(&a.roi).unwrap());
        Ok(trader_stats)
    }

    /// todo: RPC doesn't support get account at historical slot. Need to do snapshot/rocksdb parsing...
    /// `snapshot_slot_period` is slots between snapshots, default is 1 day = 216_000 slots.
    /// `num_snapshots` defaults to 30 snapshots (30 days).
    pub async fn trader_history(
        &self,
        trader: &DriftTrader,
        snapshot_slot_period: Option<u64>,
        num_snapshots: Option<u64>,
    ) -> Result<Vec<DriftTraderSnapshot>> {
        let snapshot_slot_period = snapshot_slot_period.unwrap_or(216_000);
        let num_snapshots = num_snapshots.unwrap_or(30);
        let current_slot = self.connection.get_slot().await?;
        let results: Vec<DriftTraderSnapshot> =
            stream::iter((0..num_snapshots).collect::<Vec<u64>>())
                .then(|i| async move {
                    let slot = current_slot - (i * snapshot_slot_period);
                    let user_addresses =
                        trader.users.iter().map(|u| u.key).collect::<Vec<Pubkey>>();
                    info!("fetch at {}", slot);
                    let user_accounts: Vec<KeyedAccount<User>> = self
                        .connection
                        .get_multiple_accounts_with_config(
                            &user_addresses,
                            RpcAccountInfoConfig {
                                min_context_slot: Some(slot),
                                ..Default::default()
                            },
                        )
                        .await?
                        .value
                        .into_iter()
                        .enumerate()
                        .filter_map(|(index, acc)| match acc {
                            None => None,
                            Some(account) => match Archive::deserialize_account(&account.data) {
                                Err(_) => None,
                                Ok(user) => Some(KeyedAccount {
                                    key: user_addresses[index],
                                    account: *user,
                                }),
                            },
                        })
                        .collect();

                    let user_stats = KeyedAccount {
                        key: trader.user_stats.key,
                        account: *Archive::deserialize_account::<UserStats>(
                            &self
                                .connection
                                .get_account(&trader.user_stats.key)
                                .await?
                                .data,
                        )?,
                    };
                    let snapshot = DriftTraderSnapshot {
                        slot,
                        trader: DriftTrader {
                            authority: trader.authority,
                            user_stats,
                            users: user_accounts,
                        },
                    };
                    Result::<_, anyhow::Error>::Ok(snapshot)
                })
                .filter_map(|x| async { x.ok() })
                .collect()
                .await;
        Ok(results)
    }

    pub async fn trader_history_stats(
        &self,
        trader: &DriftTrader,
        snapshot_slot_period: Option<u64>,
        num_snapshots: Option<u64>,
    ) -> Result<Vec<TraderStats>> {
        let snapshots = self
            .trader_history(trader, snapshot_slot_period, num_snapshots)
            .await?;
        let stats: Vec<TraderStats> = snapshots.into_iter().map(TraderStats::from).collect();
        Ok(stats)
    }
}
