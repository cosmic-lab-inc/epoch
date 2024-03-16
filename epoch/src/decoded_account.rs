use crate::account::EpochAccount;
use borsh::{BorshDeserialize, BorshSerialize};
use decoder::Decoder;
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DecodedEpochAccount {
    pub account: EpochAccount,
    pub decoded: Decoder,
}

#[derive(Deserialize, Serialize)]
pub struct JsonEpochAccount {
    pub account: EpochAccount,
    pub decoded: serde_json::Value,
}
