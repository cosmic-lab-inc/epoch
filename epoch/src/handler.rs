use crate::{
    account::EpochAccount,
    decoded_account::{DecodedEpochAccount, JsonEpochAccount},
    errors::{EpochError, EpochResult},
};
use actix_web::web::{BytesMut, Payload};
use decoder::program_decoder::ProgramDecoder;
use futures::StreamExt;
use gcs::bq::*;
use log::{error, info};

const MAX_SIZE: usize = 262_144; // max payload size is 256k

pub struct EpochHandler {
    pub client: BigQueryClient,
    pub decoder: ProgramDecoder,
}

impl EpochHandler {
    pub fn new(client: BigQueryClient) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            decoder: ProgramDecoder::new()?,
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
        let query = serde_json::from_slice::<QueryAccountType>(&body)?;
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
                    Result::<_, anyhow::Error>::Ok(DecodedEpochAccount { account, decoded })
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
        let query = serde_json::from_slice::<QueryAccountType>(&body)?;
        let archive_accts = self.client.account_type(&query).await?;

        // TODO: par iter by wrapping ProgramDecoder in Arc
        let decoded_accts: Vec<JsonEpochAccount> = archive_accts
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
                        &mut account.data[..8].try_into()?,
                    ) {
                        Ok(decoded) => decoded,
                        Err(e) => {
                            error!("Error decoding account: {:?}", e);
                            Err(EpochError::Anyhow(e))?
                        }
                    };
                    Result::<_, anyhow::Error>::Ok(JsonEpochAccount { account, decoded })
                }
            })
            .collect();
        Ok(decoded_accts)
    }
}
