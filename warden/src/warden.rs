use crate::{redis::redis_client::RedisClient, scrambler::Scrambler, HasherTrait, ToRedisKey};
use anchor_lang::Id;
use anchor_spl::associated_token;
use common_utils::prelude::*;
use log::{error, info};
use player_profile::client::{find_key_in_profile};
use player_profile::state::{Profile, ProfileKey};
use profile_vault::{
    drain_vault_ix, ProfileVaultPermissions, VaultAuthority,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::bs58;

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
}

impl Warden {
    pub fn new(redis_url: &str, rpc_url: String) -> anyhow::Result<Self> {
        Ok(Self {
            redis: RedisClient::new(redis_url)?,
            client: RpcClient::new(rpc_url),
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

    pub fn find_epoch_vault(owner: &Pubkey) -> anyhow::Result<Pubkey> {
        let epoch_vault = associated_token::get_associated_token_address_with_program_id(
            owner,
            &Pubkey::from_str(EPOCH_MINT)?,
            &Token2022::id(),
        );
        Ok(epoch_vault)
    }
    
    pub async fn debit_vault<T: ToRedisKey>(
        &self,
        api_key: &T,
        epoch_protocol_signer: &DynSigner<'static>,
        debit_amount: u64,
    ) -> anyhow::Result<String> {
        let profile = self.read_user(api_key)?;

        let profile_account = self
            .client
            .get_wrapped_account::<Profile, Vec<ProfileKey>>(profile)
            .await?;

        let drain_vault_key_index = find_key_in_profile(
            profile_account.remaining,
            epoch_protocol_signer.pubkey(),
            [profile_vault::ID],
            ProfileVaultPermissions::DRAIN_VAULT,
        ).ok_or(
            anyhow::anyhow!(
                "Profile {} missing drain vault key",
                profile
            )
        )?;

        let mint = Pubkey::from_str(EPOCH_MINT)?;
        let (vault_auth, _) = VaultAuthority::find_program_address(&profile, &mint);
        let epoch_vault = Warden::find_epoch_vault(&vault_auth)?;

        assert_eq!(epoch_protocol_signer.pubkey(), Pubkey::from_str(EPOCH_PROTOCOL)?);
        let protocol_vault = Warden::find_epoch_vault(&epoch_protocol_signer.pubkey())?;

        let ix = drain_vault_ix(
            profile,
            drain_vault_key_index,
            epoch_protocol_signer,
            mint,
            epoch_vault,
            vault_auth,
            protocol_vault, // todo: init token account for EPOCH_PROTOCOL on server startup
            debit_amount,
            EPOCH_MINT_DECIMALS,
        );
        match self.client
          .build_send_and_check(
              [ix],
              epoch_protocol_signer,
          )
          .await {
              Ok((sig, _slot)) => {
                  Ok(bs58::encode(sig).into_string())
              },
              Err(e) => {
                  Err(anyhow::anyhow!("Error debiting vault for Profile {} with error: {}", profile, e))
          }
        }
    }

    /// Hash the api key and check against the hashed key in Redis.
    pub fn read_user<T: ToRedisKey>(&self, api_key: &T) -> anyhow::Result<Pubkey> {
        let hashed_key = Scrambler::new().hash(api_key);
        match self.redis.get(hashed_key)? {
            None => {
                error!("API key not recognized");
                Err(anyhow::anyhow!("API key not recognized"))
            }
            Some(user_profile) => Ok(Pubkey::from_str(&user_profile)?),
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



#[cfg(tests)]
mod tests {
    use crate::{redis::redis_client::RedisClient, scrambler::Scrambler, HasherTrait, ToRedisKey};
    use anchor_lang::Id;
    use anchor_spl::associated_token;
    use common_utils::prelude::solana_client::rpc_config::RpcRequestAirdropConfig;
    use common_utils::prelude::*;
    use log::{error, info};
    use player_profile::client::{AddProfileKey, find_key_in_profile};
    use player_profile::instructions::create_profile_ix;
    use player_profile::state::{Profile, ProfileKey, ProfilePermissions};
    use profile_vault::{
        create_vault_authority_ix, drain_vault_ix, ProfileVaultPermissions, VaultAuthority,
    };
    use solana_sdk::commitment_config::CommitmentConfig;
    use solana_sdk::native_token::LAMPORTS_PER_SOL;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;
    
    #[tokio::test]
    async fn test_debit_epoch_vault() -> anyhow::Result<()> {
        dotenv::dotenv().ok();
    
        let client = get_client();
        let [user] = client.create_funded_keys().await?;
    
        let epoch_protocol = Warden::read_keypair_from_env("EPOCH_PROTOCOL")?;
        let mint = Warden::read_keypair_from_env("EPOCH_MINT")?;
        let decimals = 2;
        println!("mint: {}", mint.pubkey());
        println!("protocol: {}", epoch_protocol.pubkey());
    
        client
            .request_airdrop_with_config(
                &epoch_protocol.pubkey(),
                LAMPORTS_PER_SOL,
                RpcRequestAirdropConfig {
                    commitment: Some(CommitmentConfig::confirmed()),
                    ..Default::default()
                },
            )
            .await?;
    
        let profile_key = Keypair::new();
        let (vault_auth, vault_bump) =
            VaultAuthority::find_program_address(&profile_key.pubkey(), &mint.pubkey());
    
        let epoch_vault = associated_token::get_associated_token_address_with_program_id(
            &vault_auth,
            &mint.pubkey(),
            &Token2022::id(),
        );
        let protocol_vault = associated_token::get_associated_token_address_with_program_id(
            &epoch_protocol.pubkey(),
            &mint.pubkey(),
            &Token2022::id(),
        );
    
        let cfg = CreateMint2022Config {
            funder: Keypair::from_bytes(&user.to_bytes())?,
            mint: Keypair::from_bytes(&mint.to_bytes())?,
            mint_authority: Keypair::from_bytes(&epoch_protocol.to_bytes())?,
            freeze_authority: None,
            fee_authority: Some(Keypair::from_bytes(&epoch_protocol.to_bytes())?),
            fee_basis_points: 1500,
            decimals,
        };
    
        match client
            .create_mint_2022_with_config(&user as &DynSigner, cfg)
            .await
        {
            Err(e) => {
                // if error contains "already in use" then ignore
                if e.to_string().contains("already in use") {
                    println!("Mint already initialized");
                    Ok(())
                } else {
                    Err(anyhow::Error::from(e))
                }
            }
            Ok(_res) => Ok(()),
        }?;
    
        let create_epoch_vault_ix = InstructionWithSigners::build(|_| {
            (
                associated_token::instruction::create_associated_token_account_idempotent(
                    &user.pubkey(),
                    &vault_auth,
                    &mint.pubkey(),
                    &Token2022::id(),
                ),
                vec![],
            )
        });
        // protocol vault
        let create_protocol_vault_ix = InstructionWithSigners::build(|_| {
            (
                associated_token::instruction::create_associated_token_account_idempotent(
                    &user.pubkey(),
                    &epoch_protocol.pubkey(),
                    &mint.pubkey(),
                    &Token2022::id(),
                ),
                vec![],
            )
        });
        client
            .build_send_and_check(
                [create_epoch_vault_ix, create_protocol_vault_ix],
                &user as &DynSigner,
            )
            .await?;
    
        println!("Epoch vault: {}", epoch_vault);
        println!("Protocol vault: {}", protocol_vault);
        client
            .mint_to_token_2022_account(&user, &mint.pubkey(), epoch_vault, 10000, &epoch_protocol)
            .await?;
    
        let create_profile_ixs = [
            create_profile_ix(
                &profile_key,
                [
                    AddProfileKey::new(&user, player_profile::ID, -1, ProfilePermissions::AUTH),
                    AddProfileKey::new(
                        &user,
                        profile_vault::ID,
                        -1,
                        ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
                    ),
                    AddProfileKey::new(
                        &epoch_protocol,
                        profile_vault::ID,
                        -1,
                        ProfileVaultPermissions::DRAIN_VAULT,
                    ),
                ],
                1,
            ),
            create_vault_authority_ix(profile_key.pubkey(), 1, &user, mint.pubkey()),
        ];
        client
            .build_send_and_check(create_profile_ixs, &user)
            .await?;
    
        // validate profile created correctly
        let profile_account = client
            .get_wrapped_account::<Profile, Vec<ProfileKey>>(profile_key.pubkey())
            .await?;
        assert_eq!(
            profile_account.header,
            Profile {
                version: 0,
                auth_key_count: 1,
                key_threshold: 1,
                next_seq_id: 0,
                created_at: profile_account.header.created_at,
            }
        );
        assert_eq!(
            profile_account.remaining,
            vec![
                ProfileKey {
                    key: user.pubkey(),
                    scope: player_profile::ID,
                    expire_time: -1,
                    permissions: ProfilePermissions::AUTH.bits().to_le_bytes(),
                },
                ProfileKey {
                    key: user.pubkey(),
                    scope: profile_vault::ID,
                    expire_time: -1,
                    permissions: ProfileVaultPermissions::CREATE_VAULT_AUTHORITY
                        .bits()
                        .to_le_bytes(),
                },
                ProfileKey {
                    key: epoch_protocol.pubkey(),
                    scope: profile_vault::ID,
                    expire_time: -1,
                    permissions: ProfileVaultPermissions::DRAIN_VAULT.bits().to_le_bytes(),
                },
            ]
        );
        let vault_authority_account = client
            .get_parsed_account::<VaultAuthority>(vault_auth)
            .await?;
        assert_eq!(
            vault_authority_account.header,
            VaultAuthority {
                version: 0,
                profile: profile_key.pubkey(),
                vault_seed: mint.pubkey(),
                vault_bump,
            }
        );
    
        // create Epoch user in Redis
        let redis_url = RedisClient::fmt_redis_url(
            "default",
            "IJD4LqEHEk3mjoMxvcXDvDIKSUyNUSDD",
            "redis-17359.c284.us-east1-2.gce.cloud.redislabs.com",
            17359,
        );
        let rpc_url = "http://localhost:8899".to_string();
        let warden = Warden::new(&redis_url, rpc_url)?;
        let api_key = "warden_test_api_key".to_string();
        let user_profile = warden.upsert_user(&api_key, profile_key.pubkey())?;
        assert_eq!(user_profile, profile_key.pubkey());
    
        // pretend user made an API request and attempt to debit their vault.
        client
            .build_send_and_check(
                [drain_vault_ix(
                    profile_key.pubkey(),
                    2,
                    &epoch_protocol,
                    mint.pubkey(),
                    epoch_vault,
                    vault_auth,
                    protocol_vault,
                    10000,
                    decimals,
                )],
                &user,
            )
            .await?;
    
        // validate vault states
        let vault_token_info = client.get_token_2022_account_info(&epoch_vault).await?;
        println!("Epoch vault: {}", vault_token_info.amount);
        assert_eq!(vault_token_info.amount, 0);
    
        let funder_token_info = client.get_token_2022_account_info(&protocol_vault).await?;
        println!("Funder vault: {}", funder_token_info.amount);
        assert_eq!(funder_token_info.amount, 8500);
    
        Ok(())
    }
    
}

