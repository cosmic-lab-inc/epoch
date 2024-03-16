use borsh::{BorshSerialize, BorshDeserialize};
use crate::account::EpochAccount;
use decoder::Decoder;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DecodedEpochAccount {
    pub account: EpochAccount,
    pub decoded: Decoder,
}
