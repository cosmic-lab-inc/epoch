use crate::{redis::redis_client::RedisClient, scrambler::Scrambler, HasherTrait, ToRedisKey};
use anchor_lang::Id;
use common_utils::prelude::*;
use log::{error, info};
use profile_vault::{drain_vault_ix, VaultAuthority};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub const EPOCH_MINT: &str = "EPCHJ3JhGrx2y9NKR5BsmCLwBpFxFheMHDZsmn59BwAi";
pub const EPOCH_PROTOCOL: &str = "EPCH4ot3VAbB6nfiy7mdZYuk9C8WyjuAkEhyLyhZshCU";
pub const EPOCH_MINT_DECIMALS: u8 = 2;
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

pub struct DebitVaultConfig<T: ToRedisKey> {
    pub api_key: T,
    pub profile: Pubkey,
}

/// Warden handles storing and validating user credentials in Redis.
///
/// Redis stores a hashed API key as the key and the user's Profile pubkey as the value.
///
/// The user's Profile is used to derive the PDA of the vault authority of the Epoch vault
/// using [`VaultAuthority::find_program_address`] which is turn gets the PDA of the associated token account
/// that holds the Epoch tokens by using a PDA of the owner (vault authority PDA) and the Epoch mint.
pub struct Warden {
    pub redis: RedisClient,
}

impl Warden {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            redis: RedisClient::new(redis_url)?,
        })
    }

    pub fn find_epoch_vault(owner: &Pubkey) -> anyhow::Result<Pubkey> {
        let seeds: &[&[u8]] = &[
            &owner.to_bytes(),
            &Token2022::id().to_bytes(),
            &Pubkey::from_str(EPOCH_MINT)?.to_bytes(),
        ];

        let (epoch_vault, bump) =
            Pubkey::find_program_address(seeds, &Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID)?);
        Ok(epoch_vault)
    }

    pub async fn debit_vault<T: ToRedisKey>(
        &self,
        api_key: T,
        vault_auth_key: Pubkey, // todo: user sends in API request (connected wallet address is profile/vault auth)
        drain_vault_key_index: Option<u16>,
        epoch_protocol_signer: &DynSigner<'static>, // todo: load Keypair from env for epoch_protocol.json
        debit_amount: u64,                          // todo: credit system
    ) -> anyhow::Result<()> {
        let drain_vault_key_index = drain_vault_key_index.unwrap_or(2);
        let profile = self.read_user(&api_key)?;

        let mint = Pubkey::from_str(EPOCH_MINT)?;
        let (vault_auth, _) = VaultAuthority::find_program_address(&profile, &mint);
        let epoch_vault = Warden::find_epoch_vault(&vault_auth)?;

        let epoch_protocol = Pubkey::from_str(EPOCH_PROTOCOL)?;
        let protocol_vault = Warden::find_epoch_vault(&epoch_protocol)?;

        let ix = drain_vault_ix(
            profile,
            drain_vault_key_index,
            epoch_protocol_signer,
            mint,
            epoch_vault,
            vault_auth_key,
            protocol_vault, // todo: init token account for EPOCH_PROTOCOL on server startup
            debit_amount,
            EPOCH_MINT_DECIMALS,
        );

        Ok(())
    }

    /// Hash the api key and check against the hashed key in Redis.
    pub fn read_user<T: ToRedisKey>(&self, api_key: &T) -> anyhow::Result<Pubkey> {
        let hashed_key = Scrambler::new().hash(api_key);
        match self.redis.get(hashed_key)? {
            None => {
                error!("API key not recognized");
                Err(anyhow::anyhow!("API key not recognized"))
            }
            Some(epoch_vault) => Ok(Pubkey::from_str(&epoch_vault)?),
        }
    }

    /// Update a user's Profile under the hashed API key.
    /// This will error if the API key is already registered.
    /// Returns the new value in Redis.
    pub fn create_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        epoch_vault: Pubkey,
    ) -> anyhow::Result<String> {
        let hashed_key = Scrambler::new().hash(api_key);

        let existing_value = self.redis.get(hashed_key)?;
        match existing_value {
            Some(value) => {
                error!("API key already registered for: {}", value);
                Err(anyhow::anyhow!("API key already registered"))
            }
            None => {
                let res = self
                    .redis
                    .upsert(hashed_key, Some(epoch_vault.to_string()))?;
                match res {
                    None => {
                        error!("Error registering user, upserted as None");
                        Err(anyhow::anyhow!("Error registering user, upserted as None"))
                    }
                    Some(epoch_token_acct) => {
                        info!("Registered user: {}", epoch_token_acct);
                        Ok(epoch_token_acct)
                    }
                }
            }
        }
    }

    /// Update a user's Profile under the hashed API key.
    /// Warning: This will overwrite the pubkey if the API key is already registered.
    /// For new users, use [`create_user`] instead.
    /// Returns the new value in Redis.
    pub fn update_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        epoch_vault: Pubkey,
    ) -> anyhow::Result<String> {
        let hashed_key = Scrambler::new().hash(api_key);
        let user_profile = self
            .redis
            .upsert(hashed_key, Some(epoch_vault.to_string()))?;
        match user_profile {
            None => {
                error!("Error registering user, upserted as None");
                Err(anyhow::anyhow!("Error registering user, upserted as None"))
            }
            Some(user_profile) => {
                info!("Registered user: {}", user_profile);
                Ok(user_profile)
            }
        }
    }

    /// Delete the Redis key-value pair for the hashed API key.
    pub fn delete_user<T: ToRedisKey>(&self, api_key: &T, profile: Pubkey) -> anyhow::Result<()> {
        let hashed_key = Scrambler::new().hash(api_key);

        let user_profile = self.redis.get(hashed_key)?;
        match user_profile {
            Some(value) => match profile.to_string() == value {
                true => {
                    let res = self.redis.upsert(hashed_key, None)?;
                    info!("Deleted user: {:?}", res);
                    Ok(())
                }
                false => {
                    error!("Failed to delete API key, as it does not match the registered account");
                    Err(anyhow::anyhow!("API key does not match registered account",))
                }
            },
            None => {
                error!("Failed to delete API key, as it is not registered");
                Err(anyhow::anyhow!("API key not registered"))
            }
        }
    }
}
