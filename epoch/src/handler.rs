use crate::account::EpochAccount;
use crate::errors::{EpochError, EpochResult};
use actix_web::web::{BytesMut, Payload};
use futures::StreamExt;
use gcs::bq::*;

const MAX_SIZE: usize = 262_144; // max payload size is 256k

pub struct EpochHandler {
    pub client: BigQueryClient,
}

impl EpochHandler {
    pub fn new(client: BigQueryClient) -> Self {
        Self { client }
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
        let query = serde_json::from_slice::<Paginate>(&body)?;
        Ok(self
            .client
            .accounts(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_key(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsKey>(&body)?;
        Ok(self
            .client
            .accounts_key(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_owner(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsOwner>(&body)?;
        Ok(self
            .client
            .accounts_owner(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_slot(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsSlot>(&body)?;
        Ok(self
            .client
            .accounts_slot(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_key_owner(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsKeyOwner>(&body)?;
        Ok(self
            .client
            .accounts_key_owner(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_key_slot(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsKeySlot>(&body)?;
        Ok(self
            .client
            .accounts_key_slot(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_owner_slot(&self, payload: Payload) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsOwnerSlot>(&body)?;
        Ok(self
            .client
            .accounts_owner_slot(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }

    pub async fn accounts_key_owner_slot(
        &self,
        payload: Payload,
    ) -> EpochResult<Vec<EpochAccount>> {
        let body = self.checked_payload(payload).await?;
        let query = serde_json::from_slice::<QueryAccountsKeyOwnerSlot>(&body)?;
        Ok(self
            .client
            .accounts_key_owner_slot(&query)
            .await?
            .into_iter()
            .filter_map(|a| EpochAccount::try_from(a).ok())
            .collect::<Vec<EpochAccount>>())
    }
}
