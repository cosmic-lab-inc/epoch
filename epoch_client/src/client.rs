use std::str::FromStr;

use anchor_lang::{Discriminator, Owner};
use common::init_logger;
use decoder::drift_cpi::{PerpMarket, SpotBalanceType, SpotMarket};
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::account::Account;

use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use anchor_lang::Id;
use borsh::BorshDeserialize;
use common_utils::prelude::anchor_spl::associated_token;
use common_utils::prelude::anchor_spl::associated_token::get_associated_token_address_with_program_id;
use common_utils::prelude::{
    AccountWithRemaining, DynSigner, InstructionWithSigners, RpcClient, RpcClientExt, Token2022,
};
use log::{error, info};
use player_profile::client::AddProfileKey;
use player_profile::instructions::create_profile_ix;
use player_profile::state::{Profile, ProfileKey, ProfilePermissions};
use profile_vault::{create_vault_authority_ix, ProfileVaultPermissions, VaultAuthority};
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use solana_client::rpc_config::RpcRequestAirdropConfig;
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use common::{
    AccountHasher, AuthenticateSignature, EpochAccount, EpochProfile, EpochUser, HashTrait,
    QueryAccountId, QueryAccounts, QueryDecodedAccounts, QueryRegisteredTypes, RegisteredType,
    RequestAirdrop, RequestChallenge, VaultBalance,
};
use decoder::decoded_account::{DecodedEpochAccount, JsonEpochAccount};
use warden::{ToRedisKey, Warden};

pub const EPOCH_API_KEY_HEADER: &str = "epoch_api_key";
pub const EPOCH_API_URL: &str = "https://api.epoch.fm";
pub const EPOCH_MINT: &str = "EPCHJ3JhGrx2y9NKR5BsmCLwBpFxFheMHDZsmn59BwAi";
pub const EPOCH_PROTOCOL: &str = "EPCH4ot3VAbB6nfiy7mdZYuk9C8WyjuAkEhyLyhZshCU";

pub struct EpochClient {
    pub signer: Keypair,
    pub rpc: RpcClient,
    pub client: Client,
    pub epoch_api: String,
}

impl EpochClient {
    pub fn new(signer: Keypair, rpc_url: String, epoch_api: Option<String>) -> Self {
        let epoch_api = epoch_api.unwrap_or_else(|| EPOCH_API_URL.to_string());
        Self {
            signer,
            rpc: RpcClient::new(rpc_url),
            client: Client::new(),
            epoch_api,
        }
    }

    pub fn read_keypair_from_env(env_key: &str) -> anyhow::Result<Keypair> {
        Warden::read_keypair_from_env(env_key)
    }

    //
    //
    // RPC utilities
    //
    //

    pub async fn profiles_for_key(
        client: &RpcClient,
        auth: Pubkey,
        search: Option<ProfileKey>,
    ) -> anyhow::Result<Vec<AccountWithRemaining<Profile, Vec<ProfileKey>>>> {
        let profiles = client
            .get_wrapped_program_accounts::<Profile, Vec<ProfileKey>>()
            .await?;
        // filter first ProfileKey with key == auth
        let profiles_for_auth: Vec<_> = profiles
            .into_iter()
            .filter_map(|p| match p.remaining.first() {
                Some(key) => {
                    match key.key == auth
                        && key.scope == player_profile::ID
                        && key.permissions == ProfilePermissions::AUTH.bits().to_le_bytes()
                    {
                        true => Some(p),
                        false => None,
                    }
                }
                None => None,
            })
            .collect();
        // if some search key then find all profiles with some remaining key == search
        Ok(match search {
            Some(search_key) => profiles_for_auth
                .into_iter()
                .filter(|p| {
                    p.remaining.iter().any(|k| {
                        k.key == search_key.key
                            && k.scope == search_key.scope
                            && k.permissions == search_key.permissions
                    })
                })
                .collect::<Vec<_>>(),
            None => profiles_for_auth,
        })
    }

    pub async fn epoch_profile(
        &self,
    ) -> anyhow::Result<Option<AccountWithRemaining<Profile, Vec<ProfileKey>>>> {
        let profiles = EpochClient::profiles_for_key(
            &self.rpc,
            self.signer.pubkey(),
            Some(ProfileKey {
                key: Pubkey::from_str(EPOCH_PROTOCOL)?,
                scope: profile_vault::ID,
                permissions: ProfileVaultPermissions::DRAIN_VAULT.bits().to_le_bytes(),
                expire_time: -1,
            }),
        )
        .await?;
        Ok(profiles.into_iter().next())
    }

    pub async fn create_profile(&self) -> anyhow::Result<Keypair> {
        let profile_key = Keypair::new();
        let epoch_protocol = Pubkey::from_str(EPOCH_PROTOCOL)?;
        let mint = Pubkey::from_str(EPOCH_MINT)?;
        let vault_auth = Self::vault_auth(&profile_key.pubkey())?;

        let ixs = [
            InstructionWithSigners::build(|_| {
                (
                    associated_token::instruction::create_associated_token_account_idempotent(
                        &self.signer.pubkey(),
                        &vault_auth,
                        &mint,
                        &Token2022::id(),
                    ),
                    vec![],
                )
            }),
            create_profile_ix(
                &profile_key,
                [
                    AddProfileKey::new(
                        &self.signer,
                        player_profile::ID,
                        -1,
                        ProfilePermissions::AUTH,
                    ),
                    AddProfileKey::new(
                        &self.signer,
                        profile_vault::ID,
                        -1,
                        ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
                    ),
                    AddProfileKey::new(
                        epoch_protocol,
                        profile_vault::ID,
                        -1,
                        ProfileVaultPermissions::DRAIN_VAULT,
                    ),
                ],
                1,
            ),
            create_vault_authority_ix(profile_key.pubkey(), 1, &self.signer as &DynSigner, mint),
        ];
        let create_profile_sig = self
            .rpc
            .build_send_and_check(ixs, &self.signer as &DynSigner)
            .await?;
        info!("Create profile signature: {:?}", create_profile_sig);
        Ok(profile_key)
    }

    pub fn vault_auth(profile: &Pubkey) -> anyhow::Result<Pubkey> {
        let mint = Pubkey::from_str(EPOCH_MINT)?;
        let (vault_auth, _) = VaultAuthority::find_program_address(profile, &mint);
        Ok(vault_auth)
    }

    pub fn vault(profile: &Pubkey) -> anyhow::Result<Pubkey> {
        let mint = Pubkey::from_str(EPOCH_MINT)?;
        let vault_auth = Self::vault_auth(profile)?;
        let epoch_vault =
            get_associated_token_address_with_program_id(&vault_auth, &mint, &Token2022::id());
        Ok(epoch_vault)
    }

    /// UNIX timestamp (in seconds) of the given slot.
    pub async fn get_slot_timestamp(
        &self,
        slot: u64,
        mainnet_rpc: String,
    ) -> anyhow::Result<Option<i64>> {
        let rpc_client = RpcClient::new(mainnet_rpc);
        let block = match rpc_client.get_block(slot).await {
            Ok(block) => block,
            Err(_) => return Ok(None),
        };
        Ok(block.block_time)
    }

    pub async fn get_program_accounts<A: BorshDeserialize + Owner + Discriminator>(
        &self,
    ) -> anyhow::Result<Vec<A>> {
        let memcmp = Memcmp::new_base58_encoded(0, A::discriminator().to_vec().as_slice());
        let filters = vec![RpcFilterType::Memcmp(memcmp)];

        let account_config = RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        };

        let keyed_accounts = self
            .rpc
            .get_program_accounts_with_config(
                &A::owner(),
                RpcProgramAccountsConfig {
                    filters: Some(filters),
                    account_config,
                    ..Default::default()
                },
            )
            .await?;
        let markets: Vec<A> = keyed_accounts
            .into_iter()
            .flat_map(|(k, a)| A::deserialize(&mut a.data.as_slice()))
            .collect();
        Ok(markets)
    }

    //
    //
    // Build requests and deserialize responses
    //
    //

    fn build_headers<T: ToRedisKey>(api_key: &T) -> anyhow::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_str("application/json")?,
        );
        headers.insert(
            HeaderName::from_str(EPOCH_API_KEY_HEADER)?,
            reqwest::header::HeaderValue::from_str(&api_key.to_redis_key())?,
        );
        Ok(headers)
    }

    async fn parse_response<T: DeserializeOwned>(res: reqwest::Response) -> anyhow::Result<T> {
        match res.status() {
            StatusCode::OK => Ok(res.json::<T>().await?),
            _ => {
                let error_msg = res.text().await?;
                error!("Failed to parse Epoch response: {:?}", error_msg);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    async fn parse_borsh_response<T: BorshDeserialize>(
        res: reqwest::Response,
    ) -> anyhow::Result<T> {
        match res.status() {
            StatusCode::OK => Ok(T::deserialize(&mut res.bytes().await?.as_ref())?),
            _ => {
                let error_msg = res.text().await?;
                error!("Failed to parse Epoch response: {:?}", error_msg);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    //
    //
    // Connect (log in or sign up)
    //
    //

    pub async fn verify_wallet(&self) -> anyhow::Result<String> {
        let challenge_res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "challenge"))
            .json(&RequestChallenge {
                key: self.signer.pubkey(),
            })
            .send()
            .await?;
        let msg_to_sign = Self::parse_response::<String>(challenge_res).await?;
        // utf8 encode the message
        let msg_bytes = msg_to_sign.as_bytes();
        let secret = self.signer.to_bytes();
        let sig_bytes = nacl::sign::sign(msg_bytes, &secret)
            .map_err(|e| anyhow::anyhow!("Failed to sign message: {:?}", e))?;
        let signature = bs58::encode(sig_bytes).into_string();

        let authenticate_res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "authenticate"))
            .json(&AuthenticateSignature {
                key: self.signer.pubkey(),
                signature,
            })
            .send()
            .await?;
        match Self::parse_response::<Option<String>>(authenticate_res).await? {
            Some(api_key) => Ok(api_key),
            None => {
                error!("Failed to verify wallet");
                Err(anyhow::anyhow!("Failed to verify wallet"))
            }
        }
    }

    pub async fn reset_user(&self) -> anyhow::Result<()> {
        let api_key = self.verify_wallet().await?;
        if let Some(profile) = self.read_user(&api_key).await? {
            self.delete_user(&api_key, profile).await?;
        }
        Ok(())
    }

    pub async fn connect(&self) -> anyhow::Result<EpochUser> {
        let api_key = self.verify_wallet().await?;
        let existing_profile = self.epoch_profile().await?;
        info!("Existing profile: {:?}", existing_profile);
        let existing_user = self.read_user(&api_key).await?;
        info!("Existing user: {:?}", existing_user);

        Ok(match (existing_profile, existing_user) {
            // no user ane no profile -> create profile and create user
            (None, None) => {
                let new_profile = self.create_profile().await?;
                self.create_user(&api_key, new_profile.pubkey()).await?
            }
            // profile and no user -> create user
            (Some(profile), None) => self.create_user(&api_key, profile.key).await?,
            // no profile and user -> create profile
            (None, Some(_user)) => {
                let new_profile = self.create_profile().await?;
                self.update_user(&api_key, new_profile.pubkey()).await?
            }
            // profile and user -> do nothing
            (Some(_profile), Some(_user)) => self.epoch_user(&api_key).await?.ok_or(
                anyhow::anyhow!("Failed to find user after verifying wallet"),
            )?,
        })
    }

    //
    //
    // CRUD ops for user
    //
    //

    pub async fn epoch_airdrop<T: ToRedisKey>(
        &self,
        api_key: &T,
        key: Pubkey,
    ) -> anyhow::Result<()> {
        let _ = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "airdrop"))
            .headers(Self::build_headers(api_key)?)
            .json(&RequestAirdrop { key })
            .send()
            .await?;
        Ok(())
    }

    pub async fn user_balance<T: ToRedisKey>(&self, api_key: &T) -> anyhow::Result<VaultBalance> {
        let res = self
            .client
            .get(format!("{}/{}", &self.epoch_api, "user-balance"))
            .headers(Self::build_headers(api_key)?)
            .send()
            .await?;
        Self::parse_response::<VaultBalance>(res).await
    }

    pub async fn epoch_user<T: ToRedisKey>(
        &self,
        api_key: &T,
    ) -> anyhow::Result<Option<EpochUser>> {
        let res = self
            .client
            .get(format!("{}/{}", &self.epoch_api, "read-user"))
            .headers(Self::build_headers(api_key)?)
            .send()
            .await?;
        let profile: Option<Pubkey> = Self::parse_response::<Option<String>>(res)
            .await?
            .map(|p| Pubkey::from_str(&p))
            .transpose()?;
        info!("get profile: {:?}", profile);

        Ok(match profile {
            None => None,
            Some(profile) => {
                let vault = Self::vault(&profile)?;
                info!("get vault: {:?}", &vault);
                let epoch_user = EpochUser {
                    profile,
                    api_key: api_key.to_redis_key(),
                    vault,
                    balance: self.user_balance(api_key).await?,
                };
                Some(epoch_user)
            }
        })
    }

    pub async fn read_user<T: ToRedisKey>(&self, api_key: &T) -> anyhow::Result<Option<Pubkey>> {
        let res = self
            .client
            .get(format!("{}/{}", &self.epoch_api, "read-user"))
            .headers(Self::build_headers(api_key)?)
            .send()
            .await?;
        let profile: Option<Pubkey> = Self::parse_response::<Option<String>>(res)
            .await?
            .map(|p| Pubkey::from_str(&p))
            .transpose()?;
        Ok(profile)
    }

    pub async fn create_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        profile: Pubkey,
    ) -> anyhow::Result<EpochUser> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "create-user"))
            .headers(Self::build_headers(api_key)?)
            .json(&EpochProfile { profile })
            .send()
            .await?;
        let profile = Pubkey::from_str(&Self::parse_response::<String>(res).await?)?;
        let epoch_user = EpochUser {
            profile,
            api_key: api_key.to_redis_key(),
            vault: Self::vault(&profile)?,
            balance: self.user_balance(api_key).await?,
        };
        Ok(epoch_user)
    }

    pub async fn update_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        profile: Pubkey,
    ) -> anyhow::Result<EpochUser> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "update-user"))
            .headers(Self::build_headers(api_key)?)
            .json(&EpochProfile { profile })
            .send()
            .await?;
        let profile = Pubkey::from_str(&Self::parse_response::<String>(res).await?)?;
        let epoch_user = EpochUser {
            profile,
            api_key: api_key.to_redis_key(),
            vault: Self::vault(&profile)?,
            balance: self.user_balance(api_key).await?,
        };
        Ok(epoch_user)
    }

    pub async fn delete_user<T: ToRedisKey>(
        &self,
        api_key: &T,
        profile: Pubkey,
    ) -> anyhow::Result<String> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "delete-user"))
            .headers(Self::build_headers(api_key)?)
            .json(&EpochProfile { profile })
            .send()
            .await?;
        Self::parse_response::<String>(res).await
    }

    //
    //
    // Interact with Google BigQuery
    //
    //

    pub async fn account_id<T: ToRedisKey>(
        &self,
        api_key: &T,
        key: &Pubkey,
        slot: u64,
    ) -> anyhow::Result<Option<EpochAccount>> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "account-id"))
            .headers(Self::build_headers(api_key)?)
            .json(&QueryAccountId {
                id: AccountHasher::new().hash_id(key, slot),
            })
            .send()
            .await?;
        Self::parse_response::<Option<EpochAccount>>(res).await
    }

    pub async fn accounts<T: ToRedisKey>(
        &self,
        api_key: &T,
        query: QueryAccounts,
    ) -> anyhow::Result<Vec<EpochAccount>> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "accounts"))
            .headers(Self::build_headers(api_key)?)
            .json(&query)
            .send()
            .await?;
        Self::parse_response::<Vec<EpochAccount>>(res).await
    }

    pub async fn borsh_decoded_accounts<T: ToRedisKey>(
        &self,
        api_key: &T,
        query: QueryDecodedAccounts,
    ) -> anyhow::Result<Vec<DecodedEpochAccount>> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "borsh-decoded-accounts"))
            .headers(Self::build_headers(api_key)?)
            .json(&query)
            .send()
            .await?;
        Self::parse_borsh_response::<Vec<DecodedEpochAccount>>(res).await
    }

    pub async fn json_decoded_accounts<T: ToRedisKey>(
        &self,
        api_key: &T,
        query: QueryDecodedAccounts,
    ) -> anyhow::Result<Vec<JsonEpochAccount>> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "decoded-accounts"))
            .headers(Self::build_headers(api_key)?)
            .json(&query)
            .send()
            .await?;
        Self::parse_response::<Vec<JsonEpochAccount>>(res).await
    }

    pub async fn registered_types<T: ToRedisKey>(
        &self,
        api_key: &T,
    ) -> anyhow::Result<Vec<RegisteredType>> {
        let res = self
            .client
            .get(format!("{}/{}", &self.epoch_api, "registered-types"))
            .headers(Self::build_headers(api_key)?)
            .send()
            .await?;
        Self::parse_response::<Vec<RegisteredType>>(res).await
    }

    pub async fn filtered_registered_types<T: ToRedisKey>(
        &self,
        api_key: &T,
        query: QueryRegisteredTypes,
    ) -> anyhow::Result<Vec<RegisteredType>> {
        let res = self
            .client
            .post(format!("{}/{}", &self.epoch_api, "registered-types"))
            .headers(Self::build_headers(api_key)?)
            .json(&query)
            .send()
            .await?;
        Self::parse_response::<Vec<RegisteredType>>(res).await
    }

    pub async fn highest_slot(&self) -> anyhow::Result<u64> {
        let res = self
            .client
            .get(format!("{}/{}", &self.epoch_api, "highest-slot"))
            .send()
            .await?;
        Self::parse_response::<u64>(res).await
    }

    pub async fn lowest_slot(&self) -> anyhow::Result<u64> {
        let res = self
            .client
            .get(format!("{}/{}", &self.epoch_api, "lowest-slot"))
            .send()
            .await?;
        Self::parse_response::<u64>(res).await
    }
}

#[tokio::test]
async fn test_epoch_client() -> anyhow::Result<()> {
    let rpc_url = "http://localhost:8899".to_string();
    // let signer = Keypair::new();
    let signer = EpochClient::read_keypair_from_env("COVEST")?;
    let key = signer.pubkey();
    let client = EpochClient::new(signer, rpc_url, Some("http://localhost:3333".to_string()));

    client
        .rpc
        .request_airdrop_with_config(
            &key,
            LAMPORTS_PER_SOL,
            RpcRequestAirdropConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
        )
        .await?;

    let epoch_user = client.connect().await?;
    println!("Epoch user: {:#?}", epoch_user);

    client
        .epoch_airdrop(&epoch_user.api_key, epoch_user.vault)
        .await?;

    let decoded_accounts = client
        .json_decoded_accounts(
            &epoch_user.api_key,
            QueryDecodedAccounts {
                key: Some(Pubkey::from_str(
                    "A8PudbQF6ALzqQLUNzYaenc6jsVE1kGPVJbjMyqixsWv",
                )?),
                slot: None,
                owner: Pubkey::from_str("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH")?,
                discriminant: "User".to_string(),
                limit: 5,
                offset: 0,
            },
        )
        .await?;
    for account in decoded_accounts {
        println!("{:#?}", account);
    }

    Ok(())
}
