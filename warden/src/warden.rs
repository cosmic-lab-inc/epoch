#![allow(clippy::inconsistent_digit_grouping)]

use anchor_spl::associated_token;
use common::VaultBalance;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::TransferFeeAmount;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022::extension::{
    BaseStateWithExtensions, StateWithExtensions,
};
use common_utils::prelude::*;
use log::{error, info, warn};
use player_profile::client::find_key_in_profile;
use player_profile::state::{Profile, ProfileKey};
use profile_vault::{drain_vault_ix, ProfileVaultPermissions, VaultAuthority};
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::{
    redis::redis_client::RedisClient, scrambler::Scrambler, HasherTrait, ToRedisKey, WardenError,
};

pub const EPOCH_MINT: &str = "EPCHJ3JhGrx2y9NKR5BsmCLwBpFxFheMHDZsmn59BwAi";
pub const EPOCH_PROTOCOL: &str = "EPCH4ot3VAbB6nfiy7mdZYuk9C8WyjuAkEhyLyhZshCU";
pub const EPOCH_MINT_DECIMALS: u8 = 2;

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
    pub client: RpcClient,
    pub is_mainnet: bool,
}

impl Warden {
    pub fn new(redis_url: &str, rpc_url: String, is_mainnet: bool) -> anyhow::Result<Self> {
        Ok(Self {
            redis: RedisClient::new(redis_url)?,
            client: RpcClient::new(rpc_url),
            is_mainnet,
        })
    }

    pub fn read_keypair_from_env(env_var: &str) -> anyhow::Result<Keypair> {
        let raw_mint = std::env::var(env_var)
            .map_err(|e| anyhow::anyhow!("Failed to get {} from env: {}", env_var, e))?;
        let raw: Vec<u8> = raw_mint
            .trim_matches(|c| c == '[' || c == ']') // Remove the square brackets
            .split(',') // Split the string into an iterator of substrings based on the comma
            .filter_map(|s| s.trim().parse().ok()) // Parse each substring to u8, filtering out any errors
            .collect(); // Collect the values into a Vec<u8>
        Ok(Keypair::from_bytes(&raw)?)
    }

    pub async fn airdrop(&self, key: Pubkey) -> anyhow::Result<()> {
        if self.is_mainnet {
            warn!("Airdrop requested on mainnet");
            Ok(())
        } else {
            let epoch_protocol = Warden::read_keypair_from_env("EPOCH_PROTOCOL")?;
            let mint = Warden::read_keypair_from_env("EPOCH_MINT")?;
            Ok(self
                .client
                .mint_to_token_2022_account(
                    &epoch_protocol as &DynSigner,
                    &mint.pubkey(),
                    key,
                    1000_00,
                    &epoch_protocol as &DynSigner,
                )
                .await?)
        }
    }

    pub fn find_epoch_vault(owner: &Pubkey) -> anyhow::Result<Pubkey> {
        let epoch_vault = associated_token::get_associated_token_address_with_program_id(
            owner,
            &Pubkey::from_str(EPOCH_MINT)?,
            &Token2022::id(),
        );
        Ok(epoch_vault)
    }

    pub fn verify_signature(msg: &[u8], sig: &[u8], wallet: Pubkey) -> anyhow::Result<bool> {
        nacl::sign::verify(msg, sig, &wallet.to_bytes())
            .map_err(|e| anyhow::anyhow!("Error verifying signature: {:?}", e))
    }

    /// Convert a UI transfer amount to the real amount by multiplying by the decimals of the mint.
    pub fn to_real_amount(amount: f64) -> u64 {
        let factor = 10u64.pow(EPOCH_MINT_DECIMALS as u32) as f64;
        (amount * factor) as u64
    }

    /// Convert a real transfer amount to the UI amount by dividing by the decimals of the mint.
    pub fn to_uiamount(amount: u64) -> f64 {
        let factor = 10u64.pow(EPOCH_MINT_DECIMALS as u32);
        (amount / factor) as f64
    }

    /// Returns signature of the debit transaction.
    pub async fn debit_vault<T: ToRedisKey>(
        &self,
        api_key: &T,
        epoch_protocol_signer: &DynSigner<'static>,
        debit_amount: u64,
    ) -> anyhow::Result<String> {
        let profile = self
            .read_user(api_key)?
            .ok_or(anyhow::anyhow!("User not found"))?;

        let profile_account = self
            .client
            .get_wrapped_account::<Profile, Vec<ProfileKey>>(profile)
            .await?;

        let drain_vault_key_index = find_key_in_profile(
            profile_account.remaining,
            epoch_protocol_signer.pubkey(),
            [profile_vault::ID],
            ProfileVaultPermissions::DRAIN_VAULT,
        )
        .ok_or(anyhow::anyhow!(
            "Profile {} missing drain vault key",
            profile
        ))?;

        let mint = Pubkey::from_str(EPOCH_MINT)?;
        let (vault_auth, _) = VaultAuthority::find_program_address(&profile, &mint);
        let epoch_vault = Warden::find_epoch_vault(&vault_auth)?;

        assert_eq!(
            epoch_protocol_signer.pubkey(),
            Pubkey::from_str(EPOCH_PROTOCOL)?
        );
        let protocol_vault = Warden::find_epoch_vault(&epoch_protocol_signer.pubkey())?;

        let ix = drain_vault_ix(
            profile,
            drain_vault_key_index,
            epoch_protocol_signer,
            mint,
            epoch_vault,
            vault_auth,
            protocol_vault,
            debit_amount,
            EPOCH_MINT_DECIMALS,
        );
        match self
            .client
            .build_send_and_check([ix], epoch_protocol_signer)
            .await
        {
            Ok((sig, _slot)) => Ok(bs58::encode(sig).into_string()),
            Err(e) => {
                error!(
                    "Error debiting vault for Profile {} and vault {} with error: {}",
                    profile, epoch_vault, e
                );
                Err(anyhow::anyhow!(
                    "Error debiting vault for Profile {} and vault {} with error: {}",
                    profile,
                    epoch_vault,
                    e
                ))
            }
        }
    }

    pub async fn user_balance<T: ToRedisKey>(&self, api_key: &T) -> anyhow::Result<VaultBalance> {
        let profile = self
            .read_user(api_key)?
            .ok_or(WardenError::UserNotFound(api_key.to_redis_key()))?;
        info!("read balance for profile: {}", profile);
        let mint = Pubkey::from_str(EPOCH_MINT)?;
        info!("find vault auth with program: {}", profile_vault::ID);
        let (vault_auth, _) = VaultAuthority::find_program_address(&profile, &mint);
        info!("read balance for vault auth: {}", vault_auth);
        let epoch_vault = Warden::find_epoch_vault(&vault_auth)?;
        info!("read balance for vault: {}", epoch_vault);
        Warden::read_epoch_vault(&self.client, &epoch_vault).await
    }

    pub async fn read_epoch_vault(
        client: &RpcClient,
        vault: &Pubkey,
    ) -> anyhow::Result<VaultBalance> {
        // get ATA from RPC
        let info = client
            .get_account_with_commitment(vault, CommitmentConfig::confirmed())
            .await?
            .value
            .ok_or(WardenError::TokenAccountNotFound(vault.to_string()))?;

        // errors with InvalidAccountData because ExtensionType is not defined in the program
        let state = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&info.data)?;

        let transfer_fee_amount = match state.get_extension::<TransferFeeAmount>() {
            Ok(ext) => Ok(ext),
            Err(e) => {
                error!("Error parsing vault TransferAmount extension: {:?}", e);
                Err(e)
            }
        }?;
        let withheld_amount: u64 = transfer_fee_amount.withheld_amount.into();
        let factor = 10_f64.powi(EPOCH_MINT_DECIMALS as i32);
        let vault_info = VaultBalance {
            amount: state.base.amount,
            ui_amount: state.base.amount as f64 / factor,
            withheld_amount,
            ui_withheld_amount: withheld_amount as f64 / factor,
            decimals: EPOCH_MINT_DECIMALS,
        };
        Ok(vault_info)
    }

    /// Hash the api key and check against the hashed key in Redis.
    pub fn read_user<T: ToRedisKey>(&self, api_key: &T) -> anyhow::Result<Option<Pubkey>> {
        let hashed_key = Scrambler::new().hash(api_key);
        match self.redis.get(hashed_key)? {
            None => {
                warn!("API key not registered");
                Ok(None)
            }
            Some(user_profile) => Ok(Some(Pubkey::from_str(&user_profile)?)),
        }
    }

    /// Update a user's Profile under the hashed API key.
    /// This will error if the API key is already registered.
    /// Returns the new value in Redis.
    pub fn create_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        profile: Pubkey,
    ) -> anyhow::Result<Pubkey> {
        let hashed_key = Scrambler::new().hash(api_key);

        let existing_value = self.redis.get(hashed_key)?;
        match existing_value {
            Some(value) => {
                error!("API key already registered for: {}", value);
                Err(anyhow::anyhow!("API key already registered"))
            }
            None => {
                let res = self.redis.upsert(hashed_key, Some(profile.to_string()))?;
                match res {
                    None => {
                        error!("Error registering user, upserted as None");
                        Err(anyhow::anyhow!("Error registering user, upserted as None"))
                    }
                    Some(user_profile) => {
                        info!("Registered user: {}", user_profile);
                        Ok(Pubkey::from_str(&user_profile)?)
                    }
                }
            }
        }
    }

    /// Update a user's Profile under the hashed API key.
    /// Warning: This will overwrite the pubkey if the API key is already registered.
    /// For new users, use [`create_user`] instead.
    /// Returns the new value in Redis.
    pub fn upsert_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        profile: Pubkey,
    ) -> anyhow::Result<Pubkey> {
        let hashed_key = Scrambler::new().hash(api_key);
        let user_profile = self.redis.upsert(hashed_key, Some(profile.to_string()))?;
        match user_profile {
            None => {
                error!("Error registering user, upserted as None");
                Err(anyhow::anyhow!("Error registering user, upserted as None"))
            }
            Some(user_profile) => {
                info!("Registered user: {}", user_profile);
                Ok(Pubkey::from_str(&user_profile)?)
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
