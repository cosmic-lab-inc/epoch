use anyhow::Result;
use bytemuck::checked::try_from_bytes;
use bytemuck::CheckedBitPattern;
use common::rpc::Chunk;
use common::transaction::{SignatureInfo, TrxData};
use common::KeyedAccount;
use futures_util::future::try_join_all;
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_sdk::account::Account;
use solana_sdk::hash::hash;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::fmt::Debug;
use std::mem::size_of;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;

pub struct Archive {
    pub program_id: Pubkey,
    pub connection: RpcClient,
}

impl Archive {
    pub fn new(program_id: Pubkey, connection: RpcClient) -> Self {
        Self {
            program_id,
            connection,
        }
    }

    pub fn deserialize_account<T: CheckedBitPattern + Debug>(account_buffer: &[u8]) -> Result<&T> {
        try_from_bytes(&account_buffer[8..][..size_of::<T>()]).map_err(Into::into)
    }

    /// Derives the account discriminator form the account name as Anchor does.
    pub fn account_discriminator(name: &str) -> [u8; 8] {
        let mut discriminator = [0u8; 8];
        let hashed = hash(format!("account:{}", name).as_bytes()).to_bytes();
        discriminator.copy_from_slice(&hashed[..8]);
        discriminator
    }

    pub async fn get_chunked_signatures_for_address(
        &self,
        key: Option<&Pubkey>,
        limit: Option<usize>,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let limit = limit.unwrap_or(1000);
        let key = key.unwrap_or(&self.program_id);

        if limit <= 1000 {
            let config = GetConfirmedSignaturesForAddress2Config {
                limit: Some(limit),
                ..Default::default()
            };
            // by default this fetches last 1000 signatures
            let res = self
                .connection
                .get_signatures_for_address_with_config(key, config)
                .await?;
            Ok(res)
        } else {
            let mut chunks: Vec<Chunk> = Vec::new();
            let mut eat_limit = limit;
            let chunk_size = 1000;
            while eat_limit > 0 {
                let start = limit - eat_limit;
                let end = std::cmp::min(start + chunk_size, limit);
                eat_limit -= &chunk_size;
                chunks.push(Chunk { start, end });
            }

            let mut sigs: Vec<RpcConfirmedTransactionStatusWithSignature> =
                Vec::with_capacity(limit);

            // zeroth index is handled differently
            let zeroth = &chunks[0];
            let zeroth_cfg = GetConfirmedSignaturesForAddress2Config {
                limit: Some(zeroth.end - zeroth.start),
                ..Default::default()
            };
            let sigs_for_zeroth_chunk = self
                .connection
                .get_signatures_for_address_with_config(&self.program_id, zeroth_cfg)
                .await?;
            let mut border_sig: RpcConfirmedTransactionStatusWithSignature =
                sigs_for_zeroth_chunk[sigs_for_zeroth_chunk.len() - 1].clone();
            sigs.extend(sigs_for_zeroth_chunk);

            // iterate everything after zeroth index
            let after_zeroth = &chunks[1..chunks.len() - 1];
            for chunk in after_zeroth {
                let cfg = GetConfirmedSignaturesForAddress2Config {
                    limit: Some(chunk.end - chunk.start),
                    before: Some(Signature::from_str(&border_sig.signature)?),
                    ..Default::default()
                };
                let sigs_for_chunk = self
                    .connection
                    .get_signatures_for_address_with_config(&self.program_id, cfg)
                    .await?;
                border_sig = sigs_for_chunk[sigs_for_chunk.len() - 1].clone();
                sigs.extend(sigs_for_chunk);
            }

            Ok(sigs)
        }
    }

    pub async fn transaction_archive(
        &self,
        key: Option<&Pubkey>,
        limit: Option<usize>,
    ) -> Result<Vec<TrxData>> {
        let key = key.unwrap_or(&self.program_id);
        let sigs = self
            .get_chunked_signatures_for_address(Some(key), limit)
            .await?;

        let mut txs = Vec::<TrxData>::new();
        let opts = RpcTransactionConfig {
            max_supported_transaction_version: Some(0),
            ..Default::default()
        };
        for sig in sigs {
            let tx_info = self
                .connection
                .get_transaction_with_config(&Signature::from_str(&sig.signature)?, opts)
                .await?;

            let decoded_tx = tx_info.transaction.transaction.decode();
            if let Some(decoded_tx) = decoded_tx {
                let signature = Signature::from_str(&sig.signature)?;
                let tx = decoded_tx.into_legacy_transaction();
                if let Some(tx) = &tx {
                    if let Some(signer) = tx.message.account_keys.first() {
                        let trx_data = TrxData {
                            tx: tx.clone(),
                            signature,
                            signer: *signer,
                            slot: tx_info.slot,
                            block_time: tx_info.block_time.unwrap_or(0),
                        };
                        txs.push(trx_data);
                    }
                }
            }
        }
        Ok(txs)
    }

    pub async fn get_chunked_account_infos(
        &self,
        keys: &[Pubkey],
    ) -> Result<Vec<KeyedAccount<Account>>> {
        // get_multiple_accounts max Pubkeys is 100
        let chunk_size = 100;

        if keys.len() <= chunk_size {
            let pre_filter = self.connection.get_multiple_accounts(keys).await?;
            let infos = pre_filter
                .into_iter()
                .enumerate()
                .filter_map(|(index, acc)| {
                    acc.map(|acc| KeyedAccount {
                        key: keys[index],
                        account: acc,
                    })
                })
                .collect::<Vec<KeyedAccount<Account>>>();
            Ok(infos)
        } else {
            let chunks = keys.chunks(chunk_size).collect::<Vec<&[Pubkey]>>();
            let infos = try_join_all(chunks.into_iter().enumerate().map(
                move |(_index, chunk)| async move {
                    let accs = self
                        .connection
                        .get_multiple_accounts(chunk)
                        .await?
                        .into_iter()
                        .enumerate()
                        .filter_map(|(index, acc)| {
                            acc.map(|acc| KeyedAccount {
                                key: chunk[index],
                                account: acc,
                            })
                        });
                    Result::<_, anyhow::Error>::Ok(accs)
                },
            ))
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<KeyedAccount<Account>>>();

            Ok(infos)
        }
    }

    pub fn time_since_signature(
        ctx: RpcConfirmedTransactionStatusWithSignature,
    ) -> Result<SignatureInfo> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let block_time = ctx.block_time.unwrap();
        let elapsed = now.as_secs() as i64 - block_time;

        let days = elapsed / 86_400;
        let hours = (elapsed % 86_400) / 3600;
        let minutes = (elapsed % 3600) / 60;
        let seconds = elapsed % 60;
        let formatted_age = format!("{}d {}h {}m {}s", days, hours, minutes, seconds);
        info!("Oldest sig: {},\nage: {}", ctx.signature, formatted_age);

        Ok(SignatureInfo {
            ctx,
            unix_seconds_since: elapsed,
            formatted_time_since: formatted_age,
        })
    }
}
