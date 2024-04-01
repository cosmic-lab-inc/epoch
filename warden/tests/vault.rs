use anchor_lang::Id;
use anchor_lang::solana_program::system_instruction::create_account;
use anchor_spl::associated_token;
use anyhow::Result;
use common_utils::prelude::*;
use common_utils::prelude::anchor_spl::associated_token::get_associated_token_address_with_program_id;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022::extension::{ExtensionType, StateWithExtensions};
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022::instruction::initialize_mint2;
use common_utils::prelude::solana_client::rpc_config::{RpcRequestAirdropConfig, RpcSendTransactionConfig};
use player_profile::{
    client::AddProfileKey, instructions::create_profile_ix, state::ProfilePermissions,
};
use player_profile::state::{Profile, ProfileKey};
use profile_vault::{
    close_vault_ix, create_vault_authority_ix, drain_vault_ix, ProfileVaultPermissions,
    VaultAuthority,
};
use solana_sdk::bs58;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::transaction::Transaction;

use warden::{RedisClient, Warden, WardenError};

// Examples of more token extension tests and code
// https://github.com/solana-labs/solana-program-library/blob/c38a1b5/token/program-2022-test/tests

#[tokio::test]
async fn create_vault_authority_test() -> Result<()> {
    let client = get_client();
    let [funder, key, create_vault_key, vault_seed] = client.create_funded_keys().await?;

    let profile_key = Keypair::new();
    let ixs = [
        create_profile_ix(
            &profile_key,
            [
                AddProfileKey::new(&key, player_profile::ID, -1, ProfilePermissions::AUTH),
                AddProfileKey::new(
                    &create_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
                ),
            ],
            1,
        ),
        create_vault_authority_ix(
            profile_key.pubkey(),
            1,
            &create_vault_key,
            vault_seed.pubkey(),
        ),
    ];
    client.build_send_and_check(ixs, &funder).await?;

    let (vault_authority_key, vault_authority_bump) =
        VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());
    let vault_authority_account = client
        .get_parsed_account::<VaultAuthority>(vault_authority_key)
        .await?;

    assert_eq!(
        vault_authority_account.header,
        VaultAuthority {
            version: 0,
            profile: profile_key.pubkey(),
            vault_seed: vault_seed.pubkey(),
            vault_bump: vault_authority_bump,
        }
    );

    Ok(())
}

#[tokio::test]
async fn token_2022_test() -> Result<()> {
    let client = get_client();
    let [funder, vault_seed] = client.create_funded_keys().await?;

    let profile_key = Keypair::new();

    let (vault_authority_key, _) =
        VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());

    let mint_authority = Keypair::new();
    let fee_authority = Keypair::new();
    let mint = Keypair::new();
    let decimals = 2;

    let cfg = CreateMint2022Config {
        funder: Keypair::from_bytes(&funder.to_bytes())?,
        mint: Keypair::from_bytes(&mint.to_bytes())?,
        mint_authority: Keypair::from_bytes(&mint_authority.to_bytes())?,
        freeze_authority: None,
        fee_authority: Some(fee_authority),
        fee_basis_points: 1500,
        decimals,
    };

    client
        .create_mint_2022_with_config(&funder as &DynSigner, cfg)
        .await?;
    println!("Created mint 2022: {}", mint.pubkey());

    let vault = client
        .create_token_2022_account(&funder, &mint.pubkey(), &vault_authority_key)
        .await?;
    println!(
        "Created token 2022 account for vault authority: {}",
        vault.pubkey()
    );

    client
        .mint_to_token_2022_account(
            &funder,
            &mint.pubkey(),
            vault.pubkey(),
            10000,
            &mint_authority,
        )
        .await?;
    println!("Minted tokens to vault authority");

    let token_info = client.get_token_2022_account_info(&vault.pubkey()).await?;
    println!("Token amount left over: {}", token_info.amount);
    assert_eq!(token_info.amount, 10000);

    Ok(())
}

#[tokio::test]
async fn drain_vault_test() -> Result<()> {
    let client = get_client();
    let [funder, key, create_vault_key, drain_vault_key, vault_seed] =
        client.create_funded_keys().await?;

    let profile_key = Keypair::new();

    let (vault_authority_key, vault_authority_bump) =
        VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());

    let mint_authority = Keypair::new();
    let fee_authority = Keypair::new();
    let mint = Keypair::new();
    let decimals = 2;

    let cfg = CreateMint2022Config {
        funder: Keypair::from_bytes(&funder.to_bytes())?,
        mint: Keypair::from_bytes(&mint.to_bytes())?,
        mint_authority: Keypair::from_bytes(&mint_authority.to_bytes())?,
        freeze_authority: None,
        fee_authority: Some(fee_authority),
        fee_basis_points: 1000,
        decimals,
    };

    client
        .create_mint_2022_with_config(&funder as &DynSigner, cfg)
        .await?;

    let vault = client
        .create_token_2022_account(&funder, &mint.pubkey(), &vault_authority_key)
        .await?;

    let funder_tokens = client
        .create_token_2022_account(&funder, &mint.pubkey(), &funder.pubkey())
        .await?;

    client
        .mint_to_token_2022_account(
            &funder,
            &mint.pubkey(),
            vault.pubkey(),
            10000,
            &mint_authority,
        )
        .await?;

    let ixs = [
        create_profile_ix(
            &profile_key,
            [
                AddProfileKey::new(&key, player_profile::ID, -1, ProfilePermissions::AUTH),
                AddProfileKey::new(
                    &create_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
                ),
                AddProfileKey::new(
                    &drain_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::DRAIN_VAULT,
                ),
            ],
            1,
        ),
        create_vault_authority_ix(
            profile_key.pubkey(),
            1,
            &create_vault_key,
            vault_seed.pubkey(),
        ),
        drain_vault_ix(
            profile_key.pubkey(),
            2,
            &drain_vault_key,
            mint.pubkey(),
            vault.pubkey(),
            vault_authority_key,
            funder_tokens.pubkey(),
            10000,
            decimals,
        ),
    ];
    client.build_send_and_check(ixs, &funder).await?;

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
                key: key.pubkey(),
                scope: player_profile::ID,
                expire_time: -1,
                permissions: ProfilePermissions::AUTH.bits().to_le_bytes(),
            },
            ProfileKey {
                key: create_vault_key.pubkey(),
                scope: profile_vault::ID,
                expire_time: -1,
                permissions: ProfileVaultPermissions::CREATE_VAULT_AUTHORITY
                    .bits()
                    .to_le_bytes(),
            },
            ProfileKey {
                key: drain_vault_key.pubkey(),
                scope: profile_vault::ID,
                expire_time: -1,
                permissions: ProfileVaultPermissions::DRAIN_VAULT.bits().to_le_bytes(),
            },
        ]
    );

    let vault_authority_account = client
        .get_parsed_account::<VaultAuthority>(vault_authority_key)
        .await?;

    assert_eq!(
        vault_authority_account.header,
        VaultAuthority {
            version: 0,
            profile: profile_key.pubkey(),
            vault_seed: vault_seed.pubkey(),
            vault_bump: vault_authority_bump,
        }
    );

    let vault_token_info = client.get_token_2022_account_info(&vault.pubkey()).await?;
    println!("Vault tokens left over: {}", vault_token_info.amount);
    assert_eq!(vault_token_info.amount, 0);

    let funder_token_info = client
        .get_token_2022_account_info(&funder_tokens.pubkey())
        .await?;
    println!("Funder tokens: {}", funder_token_info.amount);
    assert_eq!(funder_token_info.amount, 9000);

    Ok(())
}

#[tokio::test]
#[should_panic(expected = "Vault should be closed")]
async fn close_vault_test() {
    let client = get_client();
    let [funder, key, create_vault_key, close_vault_key, vault_seed] =
        client.create_funded_keys().await.unwrap();

    let profile_key = Keypair::new();

    let (vault_authority_key, _vault_authority_bump) =
        VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());

    let mint_authority = Keypair::new();
    let fee_authority = Keypair::new();
    let mint = Keypair::new();
    let decimals = 2;

    let cfg = CreateMint2022Config {
        funder: Keypair::from_bytes(&funder.to_bytes()).unwrap(),
        mint: Keypair::from_bytes(&mint.to_bytes()).unwrap(),
        mint_authority: Keypair::from_bytes(&mint_authority.to_bytes()).unwrap(),
        freeze_authority: None,
        fee_authority: Some(fee_authority),
        fee_basis_points: 1500,
        decimals,
    };

    client
        .create_mint_2022_with_config(&funder as &DynSigner, cfg)
        .await
        .unwrap();

    let vault = client
        .create_token_2022_account(&funder as &DynSigner, &mint.pubkey(), &vault_authority_key)
        .await
        .unwrap();

    let funder_tokens = client
        .create_token_2022_account(&funder, &mint.pubkey(), &funder.pubkey())
        .await
        .unwrap();

    client
        .mint_to_token_2022_account(
            &funder,
            &mint.pubkey(),
            vault.pubkey(),
            100,
            &mint_authority,
        )
        .await
        .expect("Failed to mint tokens");

    let ixs = [
        create_profile_ix(
            &profile_key,
            [
                AddProfileKey::new(&key, player_profile::ID, -1, ProfilePermissions::AUTH),
                AddProfileKey::new(
                    &create_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
                ),
                AddProfileKey::new(
                    &close_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::CLOSE_VAULT,
                ),
            ],
            1,
        ),
        create_vault_authority_ix(
            profile_key.pubkey(),
            1,
            &create_vault_key,
            vault_seed.pubkey(),
        ),
        close_vault_ix(
            profile_key.pubkey(),
            2,
            &close_vault_key,
            mint.pubkey(),
            vault.pubkey(),
            vault_authority_key,
            funder_tokens.pubkey(),
            funder.pubkey(),
            decimals,
        ),
    ];
    client.build_send_and_check(ixs, &funder).await.unwrap();

    client
        .get_token_2022_account_info(&vault.pubkey())
        .await
        .expect("Vault should be closed");
}

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
    println!("profile: {}", profile_key.pubkey());
    let (vault_auth, vault_bump) =
        VaultAuthority::find_program_address(&profile_key.pubkey(), &mint.pubkey());

    let epoch_vault =
        get_associated_token_address_with_program_id(&vault_auth, &mint.pubkey(), &Token2022::id());
    let protocol_vault = get_associated_token_address_with_program_id(
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
        fee_basis_points: 1000,
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
    let create_profile_sig = client
        .build_send_and_check(create_profile_ixs, &user)
        .await?;
    println!(
        "Profile {} created: {}",
        profile_key.pubkey(),
        create_profile_sig.0
    );

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
    let warden = Warden::new(&redis_url, rpc_url, false)?;
    let api_key = "warden_test_api_key".to_string();
    let user_profile = warden.upsert_user(api_key.clone(), profile_key.pubkey())?;
    assert_eq!(user_profile, profile_key.pubkey());

    let user_vault_before = match warden.user_balance(api_key.clone()).await {
        Ok(balance) => Ok(balance),
        Err(e) => {
            eprintln!("Error reading vault {} with error: {:?}", epoch_vault, e);
            Err(e)
        }
    }?;
    println!("User vault before: {:?}", user_vault_before);

    // pretend user made an API request and attempt to debit their vault.
    let debit_sig = warden
        .debit_vault(api_key, &epoch_protocol as &DynSigner, 1_00)
        .await?;
    println!("Debit sig: {}", debit_sig);

    let user_vault_after = Warden::read_epoch_vault(&client, &epoch_vault).await?;
    println!("User vault after: {:?}", user_vault_after);

    let protocol_vault_after = Warden::read_epoch_vault(&client, &protocol_vault).await?;
    println!("Protocol vault after: {:?}", protocol_vault_after);

    Ok(())
}

#[tokio::test]
async fn ata_token_ext() -> anyhow::Result<()> {
    let client = RpcClient::new("http://localhost:8899".to_string());
    let funder = Keypair::new();
    let owner = Keypair::new();
    let mint = Keypair::new();
    let decimals = 2;
    let fee_auth = Keypair::new();
    let mint_auth = Keypair::new();
    let freeze_auth = Keypair::new();
    let fee_basis_points = 10_00; // 10%

    let airdrop_funder_sig = client
        .request_airdrop_with_config(
            &funder.pubkey(),
            100_000_000_000,
            RpcRequestAirdropConfig {
                ..Default::default()
            },
        )
        .await?;
    println!("Airdrop funder: {}", airdrop_funder_sig);

    let airdrop_owner_sig = client
        .request_airdrop_with_config(
            &owner.pubkey(),
            100_000_000_000,
            RpcRequestAirdropConfig {
                ..Default::default()
            },
        )
        .await?;
    println!("Airdrop owner: {}", airdrop_owner_sig);

    // create mint with transfer fee extension
    let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
        ExtensionType::TransferFeeConfig,
    ])?;
    let rent = Rent::default().minimum_balance(space);
    // create system account for mint
    let create_mint_acct_ix = create_account(
        &funder.pubkey(),
        &mint.pubkey(),
        rent,
        space as u64,
        &spl_token_2022::ID,
    );
    // define transfer fee as part of mint
    let transfer_fee_cfg_ix = initialize_transfer_fee_config(
        &spl_token_2022::ID,
        &mint.pubkey(),
        Some(&fee_auth.pubkey()),
        Some(&fee_auth.pubkey()),
        fee_basis_points,
        fee_basis_points as u64,
    )?;

    // Initialize the mint
    let init_mint_ix = initialize_mint2(
        &spl_token_2022::ID,
        &mint.pubkey(),
        &mint_auth.pubkey(),
        Some(&freeze_auth.pubkey()),
        decimals,
    )?;
    let bh = client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_acct_ix, transfer_fee_cfg_ix, init_mint_ix],
        Some(&funder.pubkey()),
        &[&mint, &funder],
        bh,
    );
    let create_mint_sig = client
        .send_transaction_with_config(
            &tx,
            RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                ..Default::default()
            },
        )
        .await?;
    println!("Mint with transfer fee created: {}", create_mint_sig);

    // create ATA
    let create_ata_ix = associated_token::instruction::create_associated_token_account_idempotent(
        &funder.pubkey(),
        &owner.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::ID,
    );
    let bh = client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix],
        Some(&funder.pubkey()),
        &[&funder],
        bh,
    );
    let create_ata_sig = client
        .send_transaction_with_config(
            &tx,
            RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                ..Default::default()
            },
        )
        .await?;
    println!("ATA created: {}", create_ata_sig);

    // print ATA address
    let ata = get_associated_token_address_with_program_id(
        &owner.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::ID,
    );
    println!("ATA: {}", ata);
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // get ATA from RPC
    let info = client
        .get_account_with_commitment(&ata, CommitmentConfig::processed())
        .await?
        .value
        .ok_or(WardenError::TokenAccountNotFound(ata.to_string()))?;

    // errors with InvalidAccountData because ExtensionType is not defined in the program
    let state = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&info.data)?;
    println!("ATA state: {:?}", state.base);

    Ok(())
}

#[tokio::test]
async fn test_profiles_for_key() -> anyhow::Result<()> {
    let client = get_client();
    let [funder, key, create_vault_key, drain_vault_key, vault_seed] =
        client.create_funded_keys().await?;

    let profile_key = Keypair::new();
    let profile_auth = key;

    let ixs = [
        create_profile_ix(
            &profile_key,
            [
                AddProfileKey::new(
                    &profile_auth,
                    player_profile::ID,
                    -1,
                    ProfilePermissions::AUTH,
                ),
                AddProfileKey::new(
                    &create_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
                ),
                AddProfileKey::new(
                    &drain_vault_key,
                    profile_vault::ID,
                    -1,
                    ProfileVaultPermissions::DRAIN_VAULT,
                ),
            ],
            1,
        ),
        create_vault_authority_ix(
            profile_key.pubkey(),
            1,
            &create_vault_key,
            vault_seed.pubkey(),
        ),
    ];
    let (sig, _) = client.build_send_and_check(ixs, &funder).await?;
    let bs58_sig = bs58::encode(sig).into_string();
    println!("Create profile sig: {}", bs58_sig);

    // tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    let profiles = profiles_for_key(
        &client,
        profile_auth.pubkey(),
        Some(ProfileKey {
            key: drain_vault_key.pubkey(),
            scope: profile_vault::ID,
            permissions: ProfileVaultPermissions::DRAIN_VAULT.bits().to_le_bytes(),
            expire_time: -1,
        }),
    )
    .await?;
    println!(
        "Profiles for key {:?}: {:?}",
        profile_auth.pubkey(),
        profiles.len()
    );
    assert!(!profiles.is_empty());

    Ok(())
}

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
