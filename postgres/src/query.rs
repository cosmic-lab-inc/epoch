use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginate {
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_key.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsByKey {
    pub key: Pubkey,
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_owner.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsByOwner {
    pub owner: Pubkey,
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_slot.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsBySlot {
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_key_and_owner.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsByKeyAndOwner {
    pub key: Pubkey,
    pub owner: Pubkey,
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_key_and_slot.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsByKeyAndSlot {
    pub key: Pubkey,
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_owner_and_slot.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsByOwnerAndSlot {
    pub owner: Pubkey,
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}

// migrations/accounts_by_key_and_owner_and_slot.sql
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAccountsByKeyAndOwnerAndSlot {
    pub key: Pubkey,
    pub owner: Pubkey,
    pub slot: u64,
    pub limit: u64,
    pub offset: u64,
}
