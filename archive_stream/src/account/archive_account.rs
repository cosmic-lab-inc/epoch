// use serde::Deserializer;
// use std::str::FromStr;
use crate::{AccountHasher, HashTrait};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveAccount {
    // #[serde(serialize_with = "serialize_pubkey", deserialize_with = "deserialize_pubkey")]
    pub key: Pubkey,
    /// historical snapshot slot at which this state existed
    pub slot: u64,
    /// lamports in the account
    pub lamports: u64,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: u64,
    /// data held in this account
    pub data: Vec<u8>,
}

impl ArchiveAccount {
    pub fn discrim(&self) -> Option<[u8; 8]> {
        if self.data.len() < 8 {
            return None;
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&self.data[..8]);
        Some(arr)
    }

    pub fn hash(&self) -> u64 {
        let mut hasher = AccountHasher::new();
        hasher.hash_account(self)
    }
}

// Custom deserialization function for converting a String to a Pubkey
// pub fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
//                                   where
//                                     D: Deserializer<'de>,
// {
//     let s = String::deserialize(deserializer)?;
//     Pubkey::from_str(&s).map_err(serde::de::Error::custom)
// }
// 
// pub fn serialize_pubkey<S>(key: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
//                            where
//                              S: serde::Serializer,
// {
//     serializer.serialize_str(&key.to_string())
// }
