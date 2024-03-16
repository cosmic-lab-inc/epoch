use common::{deserialize_option_pubkey, deserialize_pubkey};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

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
pub struct QueryAccounts {
    #[serde(deserialize_with = "deserialize_option_pubkey")]
    #[serde(default)]
    pub key: Option<Pubkey>,
    pub slot: Option<u64>,
    #[serde(deserialize_with = "deserialize_option_pubkey")]
    #[serde(default)]
    pub owner: Option<Pubkey>,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountType {
    #[serde(deserialize_with = "deserialize_option_pubkey")]
    #[serde(default)]
    pub key: Option<Pubkey>,
    pub slot: Option<u64>,
    #[serde(deserialize_with = "deserialize_pubkey")]
    #[serde(default)]
    pub owner: Pubkey,
    pub discriminant: String,
    pub limit: u64,
    pub offset: u64,
}
