use crate::{hasher::Hasher, redis::redis_client::RedisClient};
use log::{error, info};
use solana_sdk::pubkey::Pubkey;

/// Redis stores a hashed API key as the key-value pair to the user's Epoch token account.
/// The value is the Epoch token account, which is a PDA of the vault authority of the token account and the Epoch token mint.
/// The Epoch token account is used in favor of the user's Profile so that we avoid RPC lookups,
/// since the vault authority is a PDA of the user's Profile, which the user's wallet owns,
/// which requires an RPC call to find the Profile for a user's wallet.
pub struct Warden {
    pub redis: RedisClient,
}

impl Warden {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            redis: RedisClient::new(redis_url)?,
        })
    }

    /// Hash the api key and check against the hashed key in Redis.
    pub fn validate_api_key(&self, api_key: String) -> anyhow::Result<Pubkey> {
        let hashed_key = Hasher::hash(api_key.as_bytes())?;
        let epoch_token_acct = self.redis.get(hashed_key)?;
        match epoch_token_acct {
            None => {
                error!("API key not recognized");
                Err(anyhow::Error::msg("API key not recognized"))
            }
            Some(epoch_token_acct) => Ok(Pubkey::new_from_array(
                epoch_token_acct.as_bytes().try_into()?,
            )),
        }
    }

    /// Update a user's Epoch token account under the hashed API key.
    /// This will error if the API key is already registered.
    pub fn register_user(
        &self,
        api_key: String,
        epoch_token_acct: Pubkey,
    ) -> anyhow::Result<String> {
        let hashed_key = Hasher::hash(api_key.as_bytes())?;

        let existing_value = self.redis.get(hashed_key.clone())?;
        match existing_value {
            Some(value) => {
                error!("API key already registered for: {}", value);
                return Err(anyhow::Error::msg("API key already registered"));
            }
            None => {
                let res = self
                    .redis
                    .upsert(hashed_key, Some(epoch_token_acct.to_string()))?;
                match res {
                    None => {
                        error!("Error registering user, upserted as None");
                        panic!("Error registering user, upserted as None")
                    }
                    Some(epoch_token_acct) => {
                        info!("Registered user: {}", epoch_token_acct);
                        Ok(epoch_token_acct)
                    }
                }
            }
        }
    }

    /// Update a user's Epoch token account under the hashed API key.
    /// Warning: This will overwrite the pubkey if the API key is already registered.
    /// For new users, use [`register_user`] instead.
    pub fn update_user(&self, api_key: String, epoch_token_acct: Pubkey) -> anyhow::Result<String> {
        let hashed_key = Hasher::hash(api_key.as_bytes())?;
        let res = self
            .redis
            .upsert(hashed_key, Some(epoch_token_acct.to_string()))?;
        match res {
            None => {
                error!("Error registering user, upserted as None");
                panic!("Error registering user, upserted as None")
            }
            Some(epoch_token_acct) => {
                info!("Registered user: {}", epoch_token_acct);
                Ok(epoch_token_acct)
            }
        }
    }

    /// Delete the Redis key-value pair for the hashed API key.
    pub fn delete_user(&self, api_key: String, epoch_token_acct: Pubkey) -> anyhow::Result<()> {
        let hashed_key = Hasher::hash(api_key.as_bytes())?;

        let existing_value = self.redis.get(hashed_key.clone())?;
        match existing_value {
            Some(value) => match epoch_token_acct.to_string() == value {
                true => {
                    let res = self.redis.upsert(hashed_key, None)?;
                    info!("Deleted user: {:?}", res);
                    Ok(())
                }
                false => {
                    error!("Failed to delete API key, as it does not match the registered account");
                    Err(anyhow::Error::msg(
                        "API key does not match registered account",
                    ))
                }
            },
            None => {
                error!("Failed to delete API key, as it is not registered");
                Err(anyhow::Error::msg("API key not registered"))
            }
        }
    }
}
