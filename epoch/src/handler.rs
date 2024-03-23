use crate::{
    account::EpochAccount,
    decoded_account::{DecodedEpochAccount, JsonEpochAccount},
    errors::{EpochError, EpochResult},
};
use actix_web::web::{BytesMut, Payload};
use common::types::query::*;
use decoder::program_decoder::ProgramDecoder;
use futures::StreamExt;
use gcs::bq::*;
use log::*;
use std::sync::Arc;
use warden::Warden;

const MAX_SIZE: usize = 262_144; // max payload size is 256k

pub const EPOCH_API_KEY_HEADER: &str = "epoch_api_key";

pub struct EpochHandler {
    pub client: BigQueryClient,
    pub decoder: Arc<ProgramDecoder>,
    pub warden: Warden,
}

impl EpochHandler {
    pub fn new(client: BigQueryClient, redis_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            decoder: Arc::new(ProgramDecoder::new()?),
            warden: Warden::new(redis_url)?,
        })
    }

    async fn checked_payload(&self, mut payload: Payload) -> EpochResult<BytesMut> {
        let mut body = BytesMut::new();
        while let Some(chunk) = payload.next().await {
            let chunk = chunk?;
            if (body.len() + chunk.len()) > MAX_SIZE {
                return Err(EpochError::Overflow);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    //
    //
    // Interact with Redis to validate hashed API key
    // and attempt to debit EPOCH tokens from user's vault
    //
    //

    pub async fn register_user(
        &self,
        payload: Payload,
        api_key: Option<String>,
    ) -> EpochResult<String> {
        match api_key {
            None => Err(EpochError::Anyhow(anyhow::anyhow!("API key required"))),
            Some(api_key) => {
                let body = self.checked_payload(payload).await?;
                let query = serde_json::from_slice::<EpochVault>(&body)?;

                Ok(self.warden.register_user(api_key, query.epoch_vault)?)
            }
        }
    }

    //
    //
    // Interact with Google BigQuery
    //
    //

    pub async fn account_id(&self, payload: Payload) -> EpochResult<Option<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountId>(&body)?;
        Ok(match self.client.account_id(&query).await? {
            None => None,
            Some(acct) => EpochAccount::try_from(acct).ok(),
        })
    }

    pub async fn accounts(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccounts>(&body)?;
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
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryDecodedAccounts>(&body)?;
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
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryDecodedAccounts>(&body)?;
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
                let body = self.checked_payload(payload).await?;
                let query = serde_json::from_slice::<QueryRegisteredTypes>(&body)?;

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
