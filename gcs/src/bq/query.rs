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
    pub key: Option<Pubkey>,
    pub slot: Option<u64>,
    pub owner: Option<Pubkey>,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountType {
    pub key: Option<Pubkey>,
    pub slot: Option<u64>,
    pub owner: Pubkey,
    pub discriminant: String,
    pub limit: u64,
    pub offset: u64,
}
