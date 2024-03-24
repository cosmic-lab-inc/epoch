use std::sync::Arc;

use actix_web::web::{BytesMut, Payload};
use common_utils::prelude::{DynSigner, RpcClientToken2022Ext};
use futures::StreamExt;
use log::*;
use serde::de::DeserializeOwned;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

use common::types::query::*;
use decoder::program_decoder::ProgramDecoder;
use gcs::bq::*;
use warden::{ToRedisKey, Warden, EPOCH_MINT_DECIMALS};

use crate::{
    account::EpochAccount,
    decoded_account::{DecodedEpochAccount, JsonEpochAccount},
    errors::{EpochError, EpochResult},
};

const MAX_SIZE: usize = 262_144; // max payload size is 256k

pub const EPOCH_API_KEY_HEADER: &str = "epoch_api_key";

pub struct EpochHandler {
    pub client: BigQueryClient,
    pub decoder: Arc<ProgramDecoder>,
    pub warden: Warden,
    pub epoch_protocol_signer: Keypair,
}

impl EpochHandler {
    pub fn new(client: BigQueryClient, redis_url: &str, rpc_url: String) -> anyhow::Result<Self> {
        let epoch_protocol_signer = Warden::read_keypair_from_env("EPOCH_PROTOCOL")?;
        Ok(Self {
            client,
            decoder: Arc::new(ProgramDecoder::new()?),
            warden: Warden::new(redis_url, rpc_url)?,
            epoch_protocol_signer,
        })
    }

    async fn parse_query<T: DeserializeOwned>(&self, mut payload: Payload) -> EpochResult<T> {
        let mut body = BytesMut::new();
        while let Some(chunk) = payload.next().await {
            let chunk = chunk?;
            if (body.len() + chunk.len()) > MAX_SIZE {
                return Err(EpochError::Overflow);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice::<T>(&body)?)
    }

    //
    //
    // Interact with Warden to manage user API key and Epoch vault stored in Redis.
    // If user requests data, validate API key against Redis data,
    // then attempt to debit user's Epoch vault of tokens.
    //
    //

    pub async fn debit_vault<T: ToRedisKey>(
        &self,
        api_key: &T,
        debit_amount: u64,
    ) -> EpochResult<String> {
        Ok(self
            .warden
            .debit_vault(
                api_key,
                &self.epoch_protocol_signer as &DynSigner,
                debit_amount,
            )
            .await?)
    }

    async fn vault_balance<T: ToRedisKey>(&self, api_key: &T) -> EpochResult<f64> {
        // validate vault states
        let vault = self.warden.read_user(api_key)?;
        let vault_token_info = self
            .warden
            .client
            .get_token_2022_account_info(&vault)
            .await
            .map_err(|e| {
                EpochError::Anyhow(anyhow::anyhow!(
                    "Error reading vault token account: {:?}",
                    e
                ))
            })?;
        let factor = 10_f64.powi(EPOCH_MINT_DECIMALS as i32);
        Ok(vault_token_info.amount as f64 / factor)
    }

    pub async fn read_vault(&self, api_key: Option<String>) -> EpochResult<f64> {
        match api_key {
            None => Err(EpochError::Anyhow(anyhow::anyhow!("API key required"))),
            Some(api_key) => {
                let balance = self.vault_balance(&api_key).await?;
                Ok(balance)
            }
        }
    }

    //
    //
    // CRUD ops for user in Redis
    //
    //

    pub async fn read_user(&self, api_key: Option<String>) -> EpochResult<Pubkey> {
        match api_key {
            None => Err(EpochError::Anyhow(anyhow::anyhow!("API key required"))),
            Some(api_key) => Ok(self.warden.read_user(&api_key)?),
        }
    }

    pub async fn create_user(
        &self,
        payload: Payload,
        api_key: Option<String>,
    ) -> EpochResult<Pubkey> {
        match api_key {
            None => Err(EpochError::Anyhow(anyhow::anyhow!("API key required"))),
            Some(api_key) => {
                let query = self.parse_query::<EpochVault>(payload).await?;
                Ok(self.warden.create_user(&api_key, query.epoch_vault)?)
            }
        }
    }

    pub async fn update_user(
        &self,
        payload: Payload,
        api_key: Option<String>,
    ) -> EpochResult<Pubkey> {
        match api_key {
            None => Err(EpochError::Anyhow(anyhow::anyhow!("API key required"))),
            Some(api_key) => {
                let query = self.parse_query::<EpochVault>(payload).await?;
                Ok(self.warden.upsert_user(&api_key, query.epoch_vault)?)
            }
        }
    }

    pub async fn delete_user(&self, payload: Payload, api_key: Option<String>) -> EpochResult<()> {
        match api_key {
            None => Err(EpochError::Anyhow(anyhow::anyhow!("API key required"))),
            Some(api_key) => {
                let query = self.parse_query::<EpochVault>(payload).await?;
                Ok(self.warden.delete_user(&api_key, query.epoch_vault)?)
            }
        }
    }

    //
    //
    // Interact with Google BigQuery
    //
    //

    pub async fn account_id(&self, payload: Payload) -> EpochResult<Option<EpochAccount>> {
        let query = self.parse_query::<QueryAccountId>(payload).await?;
        Ok(match self.client.account_id(&query).await? {
            None => None,
            Some(acct) => EpochAccount::try_from(acct).ok(),
        })
    }

    pub async fn accounts(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let query = self.parse_query::<QueryAccounts>(payload).await?;
        Ok(self
            .client
            .accounts(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn borsh_decoded_accounts(
        &self,
        payload: Payload,
    ) -> anyhow::Result<Vec<DecodedEpochAccount>> {
        let query = self.parse_query::<QueryDecodedAccounts>(payload).await?;
        let archive_accts = self.client.account_type(&query).await?;

        // TODO: par iter by wrapping ProgramDecoder in Arc
        let decoded_accts: Vec<DecodedEpochAccount> = archive_accts
            .into_iter()
            .flat_map(|a| match EpochAccount::try_from(a) {
                Err(e) => {
                    error!("Error converting ArchiveAccount to EpochAccount: {:?}", e);
                    Err(EpochError::Anyhow(e))?
                }
                Ok(account) => {
                    let name = match self
                        .decoder
                        .discrim_to_name(&query.owner, &account.data[..8].try_into()?)
                    {
                        Some(discrim) => Result::<_, anyhow::Error>::Ok(discrim),
                        None => Err(EpochError::Anyhow(anyhow::anyhow!("Invalid discriminant")))?,
                    }?;
                    let decoded =
                        match self
                            .decoder
                            .borsh_decode_account(&query.owner, &name, &account.data)
                        {
                            Ok(decoded) => decoded,
                            Err(e) => {
                                error!("Error decoding account: {:?}", e);
                                Err(EpochError::Anyhow(e))?
                            }
                        };
                    Result::<_, anyhow::Error>::Ok(DecodedEpochAccount {
                        key: account.key,
                        slot: account.slot,
                        owner: account.owner,
                        decoded,
                    })
                }
            })
            .collect();
        Ok(decoded_accts)
    }

    pub async fn json_decoded_accounts(
        &self,
        payload: Payload,
    ) -> anyhow::Result<Vec<JsonEpochAccount>> {
        let query = self.parse_query::<QueryDecodedAccounts>(payload).await?;
        let archive_accts = self.client.account_type(&query).await?;

        // TODO: par iter by making EpochAccount try from reference. Data must be borrowed Cow (use BytesWrapper)
        let mut decoded_accts: Vec<JsonEpochAccount> = archive_accts
            .into_iter()
            .flat_map(|a| match EpochAccount::try_from(a) {
                Err(e) => {
                    error!("Error converting ArchiveAccount to EpochAccount: {:?}", e);
                    Err(EpochError::Anyhow(e))?
                }
                Ok(account) => {
                    let name = match self
                        .decoder
                        .discrim_to_name(&query.owner, &account.data[..8].try_into()?)
                    {
                        Some(discrim) => Result::<_, anyhow::Error>::Ok(discrim),
                        None => Err(EpochError::Anyhow(anyhow::anyhow!("Invalid discriminant")))?,
                    }?;
                    let decoded = match self.decoder.json_decode_account(
                        &query.owner,
                        &name,
                        &mut account.data.as_slice(),
                    ) {
                        Ok(decoded) => decoded,
                        Err(e) => {
                            error!("Error decoding account: {:?}", e);
                            Err(EpochError::Anyhow(e))?
                        }
                    };
                    Result::<_, anyhow::Error>::Ok(JsonEpochAccount {
                        key: account.key,
                        slot: account.slot,
                        owner: account.owner,
                        decoded,
                    })
                }
            })
            .collect();
        // sort so the highest slot is 0th index
        decoded_accts.sort_by_key(|a| a.slot);
        decoded_accts.reverse();

        Ok(decoded_accts)
    }

    pub async fn registered_types(
        &self,
        payload: Option<Payload>,
    ) -> anyhow::Result<Vec<RegisteredType>> {
        match payload {
            None => self.decoder.registred_types(),
            Some(payload) => {
                let query = self.parse_query::<QueryRegisteredTypes>(payload).await?;
                Ok(self
                    .decoder
                    .registred_types()?
                    .into_iter()
                    .filter_map(|t| {
                        match (&query.program_name, &query.program, &query.discriminant) {
                            (Some(program_name), Some(program), Some(discriminant)) => {
                                if t.program_name.to_lowercase() == *program_name.to_lowercase()
                                    && t.program == *program
                                    && t.account_discriminant.to_lowercase()
                                        == *discriminant.to_lowercase()
                                {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (Some(program_name), Some(program), None) => {
                                if t.program_name.to_lowercase() == *program_name.to_lowercase()
                                    && t.program == *program
                                {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (Some(program_name), None, Some(discriminant)) => {
                                if t.program_name.to_lowercase() == *program_name.to_lowercase()
                                    && t.account_discriminant.to_lowercase()
                                        == *discriminant.to_lowercase()
                                {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (Some(program_name), None, None) => {
                                if t.program_name.to_lowercase() == *program_name.to_lowercase() {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (None, Some(program), Some(discriminant)) => {
                                if t.program == *program
                                    && t.account_discriminant.to_lowercase()
                                        == *discriminant.to_lowercase()
                                {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (None, Some(program), None) => {
                                if t.program == *program {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (None, None, Some(discriminant)) => {
                                if t.account_discriminant.to_lowercase()
                                    == *discriminant.to_lowercase()
                                {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            (None, None, None) => Some(t),
                        }
                    })
                    .collect())
            }
        }
    }
}
