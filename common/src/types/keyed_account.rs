use crate::serde::serialize_pubkey;
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Serialize)]
pub struct KeyedAccount<T: Serialize> {
    #[serde(serialize_with = "serialize_pubkey")]
    pub key: Pubkey,
    pub account: T,
}
