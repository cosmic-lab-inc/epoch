use serde::Deserializer;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Custom deserialization function for converting a String to a Pubkey
pub fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginate {
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountId {
    pub id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsKey {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub key: Pubkey,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsOwner {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub owner: Pubkey,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsSlot {
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsKeyOwner {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub key: Pubkey,
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub owner: Pubkey,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsKeySlot {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub key: Pubkey,
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsOwnerSlot {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub owner: Pubkey,
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsKeyOwnerSlot {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub key: Pubkey,
    pub owner: Pubkey,
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}
