use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize};
use solana_sdk::pubkey::Pubkey;

// Custom deserialization function for converting a String to a Pubkey
pub fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
                                  where
                                    D: Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_pubkey<S>(key: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
                           where
                             S: serde::Serializer,
{
    serializer.serialize_str(&key.to_string())
}