use borsh::{BorshDeserialize, BorshSerialize};
use decoder::Decoder;
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DecodedEpochAccount {
    pub key: String,
    pub slot: u64,
    pub owner: String,
    pub decoded: Decoder,
}

#[derive(Deserialize, Serialize)]
pub struct JsonEpochAccount {
    pub key: String,
    pub slot: u64,
    pub owner: String,
    pub decoded: serde_json::Value,
}
