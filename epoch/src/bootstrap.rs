#![allow(clippy::inconsistent_digit_grouping)]

use anchor_lang::Id;
use anchor_spl::associated_token;
use common_utils::prelude::anchor_spl::associated_token::get_associated_token_address_with_program_id;
use common_utils::prelude::solana_client::rpc_config::RpcRequestAirdropConfig;
use common_utils::prelude::*;
use log::{error, info};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use warden::Warden;

pub async fn bootstrap_epoch(rpc_url: String) -> anyhow::Result<()> {
    info!("Bootstrap Epoch with RPC: {}", rpc_url);
    dotenv::dotenv().ok();

    // check if localnet by seeing http in beginning of rpc url and not https
    let is_localnet = rpc_url.starts_with("http://") && !rpc_url.starts_with("https://");

    let client = RpcClient::new(rpc_url);

    let epoch_protocol = Warden::read_keypair_from_env("EPOCH_PROTOCOL")?;
    let mint = Warden::read_keypair_from_env("EPOCH_MINT")?;
    let decimals = 2;
    let fee_basis_points = 5_00; // 5%
    info!("Epoch mint: {}", mint.pubkey());
    info!("Epoch protocol: {}", epoch_protocol.pubkey());

    if is_localnet {
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
    } else {
        let signer_balance = client
            .get_balance_with_commitment(&epoch_protocol.pubkey(), CommitmentConfig::confirmed())
            .await?
            .value;
        if (signer_balance as f64) < (0.2 * LAMPORTS_PER_SOL as f64) {
            error!(
                "Epoch protocol has insufficient balance to pay for bootstrap: {} < {}",
                signer_balance, 0.2
            );
            return Err(anyhow::anyhow!(
                "Epoch protocol has insufficient balance to pay for bootstrap: {} < {}",
                signer_balance,
                0.2
            ));
        }
    }

    let protocol_vault = get_associated_token_address_with_program_id(
        &epoch_protocol.pubkey(),
        &mint.pubkey(),
        &Token2022::id(),
    );

    let cfg = CreateMint2022Config {
        funder: Keypair::from_bytes(&epoch_protocol.to_bytes())?,
        mint: Keypair::from_bytes(&mint.to_bytes())?,
        mint_authority: Keypair::from_bytes(&epoch_protocol.to_bytes())?,
        freeze_authority: None,
        fee_authority: Some(Keypair::from_bytes(&epoch_protocol.to_bytes())?),
        fee_basis_points,
        decimals,
    };

    match client
        .create_mint_2022_with_config(&epoch_protocol as &DynSigner, cfg)
        .await
    {
        Err(e) => {
            // if error contains "already in use" then ignore
            if e.to_string().contains("already in use") {
                info!("Mint already initialized");
                Ok(())
            } else {
                Err(anyhow::Error::from(e))
            }
        }
        Ok(_res) => Ok(()),
    }?;

    // create epoch protocol vault (associated token account)
    let create_protocol_vault_ix = InstructionWithSigners::build(|_| {
        (
            associated_token::instruction::create_associated_token_account_idempotent(
                &epoch_protocol.pubkey(),
                &epoch_protocol.pubkey(),
                &mint.pubkey(),
                &Token2022::id(),
            ),
            vec![],
        )
    });
    client
        .build_send_and_check([create_protocol_vault_ix], &epoch_protocol as &DynSigner)
        .await?;

    let protocol_vault_before = Warden::read_epoch_vault(&client, &protocol_vault).await?;
    if protocol_vault_before.amount >= 100_000_000_000_000_00 {
        info!("Protocol vault already has enough tokens");
        return Ok(());
    }

    info!("Protocol vault: {}", protocol_vault);
    client
        .mint_to_token_2022_account(
            &epoch_protocol,
            &mint.pubkey(),
            protocol_vault,
            100_000_000_000_000_00,
            &epoch_protocol,
        )
        .await?;

    let protocol_vault_after = Warden::read_epoch_vault(&client, &protocol_vault).await?;
    info!("Epoch protocol vault balance: {:?}", protocol_vault_after);

    Ok(())
}
