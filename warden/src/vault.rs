use anchor_spl::token_2022::spl_token_2022;
use anyhow::Result;
use common_utils::prelude::anchor_spl::token_2022::spl_token_2022::extension::{
    ExtensionType, StateWithExtensions,
};
use common_utils::prelude::*;
use player_profile::{
    client::AddProfileKey, instructions::create_profile_ix, state::ProfilePermissions,
};
use profile_vault::{
    close_vault_ix, create_vault_authority_ix, drain_vault_ix, ProfileVaultPermissions,
    VaultAuthority,
};
use solana_sdk::commitment_config::CommitmentConfig;

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

    let account_info = client
        .get_account_with_commitment(&vault.pubkey(), CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or(anyhow::anyhow!("Token account not found"))?;
    let state =
        StateWithExtensions::<spl_token_2022::state::Account>::unpack(&account_info.data).unwrap();
    println!("Token amount left over: {}", state.base.amount);
    assert_eq!(state.base.amount, 10000);

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
        fee_basis_points: 1500,
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

    let token_info = client.get_token_2022_account_info(&vault.pubkey()).await?;

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
    println!("Token amount left over: {}", token_info.amount);
    assert_eq!(token_info.amount, 8500);

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
