use crate::{deserialize_option_pubkey, deserialize_pubkey, serialize_pubkey};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

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
pub struct QueryDecodedAccounts {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRegisteredTypes {
    #[serde(default)]
    pub program_name: Option<String>,
    #[serde(deserialize_with = "deserialize_option_pubkey")]
    #[serde(default)]
    pub program: Option<Pubkey>,
    #[serde(default)]
    pub discriminant: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredType {
    pub program_name: String,
    #[serde(serialize_with = "serialize_pubkey")]
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub program: Pubkey,
    pub account_discriminant: String,
    pub account_type: serde_json::Value,
}
