// #![cfg(feature = "local-validator")]
//
// use anyhow::Result;
// use common_utils::prelude::*;
// use player_profile::{
//     client::AddProfileKey, instructions::create_profile_ix, state::ProfilePermissions,
// };
// use profile_vault::{
//     close_vault_ix, create_vault_authority_ix, drain_vault_ix, ProfileVaultPermissions,
//     VaultAuthority,
// };
//
// #[tokio::test]
// async fn create_vault_authority_test() -> Result<()> {
//     let client = get_client();
//     let [funder, key, create_vault_key, vault_seed] = client.create_funded_keys().await?;
//
//     let profile_key = Keypair::new();
//     let ixs = [
//         create_profile_ix(
//             &profile_key,
//             [
//                 AddProfileKey::new(&key, player_profile::ID, -1, ProfilePermissions::AUTH),
//                 AddProfileKey::new(
//                     &create_vault_key,
//                     profile_vault::ID,
//                     -1,
//                     ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
//                 ),
//             ],
//             1,
//         ),
//         create_vault_authority_ix(
//             profile_key.pubkey(),
//             1,
//             &create_vault_key,
//             vault_seed.pubkey(),
//         ),
//     ];
//     client.build_send_and_check(ixs, &funder).await?;
//
//     let (vault_authority_key, vault_authority_bump) =
//         VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());
//     let vault_authority_account = client
//         .get_parsed_account::<VaultAuthority>(vault_authority_key)
//         .await?;
//
//     assert_eq!(
//         vault_authority_account.header,
//         VaultAuthority {
//             version: 0,
//             profile: profile_key.pubkey(),
//             vault_seed: vault_seed.pubkey(),
//             vault_bump: vault_authority_bump,
//         }
//     );
//
//     Ok(())
// }
//
// #[tokio::test]
// async fn drain_vault_test() -> Result<()> {
//     let client = get_client();
//     let [funder, key, create_vault_key, drain_vault_key, vault_seed] =
//         client.create_funded_keys().await?;
//
//     let profile_key = Keypair::new();
//
//     let (vault_authority_key, vault_authority_bump) =
//         VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());
//
//     let CreateMintResult {
//         mint,
//         mint_authority,
//         freeze_authority: _,
//         decimals: _,
//     } = client.create_mint().await?;
//
//     let vault = client
//         .create_token_account(&funder, &mint.pubkey(), &vault_authority_key)
//         .await?;
//
//     let funder_tokens = client
//         .create_token_account(&funder, &mint.pubkey(), &funder.pubkey())
//         .await?;
//
//     client
//         .mint_to_token_account(
//             &funder,
//             &mint.pubkey(),
//             vault.pubkey(),
//             100,
//             &mint_authority,
//         )
//         .await?;
//
//     let ixs = [
//         create_profile_ix(
//             &profile_key,
//             [
//                 AddProfileKey::new(&key, player_profile::ID, -1, ProfilePermissions::AUTH),
//                 AddProfileKey::new(
//                     &create_vault_key,
//                     profile_vault::ID,
//                     -1,
//                     ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
//                 ),
//                 AddProfileKey::new(
//                     &drain_vault_key,
//                     profile_vault::ID,
//                     -1,
//                     ProfileVaultPermissions::DRAIN_VAULT,
//                 ),
//             ],
//             1,
//         ),
//         create_vault_authority_ix(
//             profile_key.pubkey(),
//             1,
//             &create_vault_key,
//             vault_seed.pubkey(),
//         ),
//         drain_vault_ix(
//             profile_key.pubkey(),
//             2,
//             &drain_vault_key,
//             vault.pubkey(),
//             vault_authority_key,
//             funder_tokens.pubkey(),
//             50,
//         ),
//     ];
//     client.build_send_and_check(ixs, &funder).await?;
//
//     let token_info = client.get_token_account_info(&vault.pubkey()).await?;
//
//     let vault_authority_account = client
//         .get_parsed_account::<VaultAuthority>(vault_authority_key)
//         .await?;
//
//     assert_eq!(
//         vault_authority_account.header,
//         VaultAuthority {
//             version: 0,
//             profile: profile_key.pubkey(),
//             vault_seed: vault_seed.pubkey(),
//             vault_bump: vault_authority_bump,
//         }
//     );
//     assert_eq!(token_info.amount, 50);
//
//     Ok(())
// }
//
// #[tokio::test]
// #[should_panic(expected = "Vault should be closed")]
// async fn close_vault_test() {
//     let client = get_client();
//     let [funder, key, create_vault_key, close_vault_key, vault_seed] =
//         client.create_funded_keys().await.unwrap();
//
//     let profile_key = Keypair::new();
//
//     let (vault_authority_key, _vault_authority_bump) =
//         VaultAuthority::find_program_address(&profile_key.pubkey(), &vault_seed.pubkey());
//
//     let CreateMintResult {
//         mint,
//         mint_authority,
//         freeze_authority: _,
//         decimals: _,
//     } = client.create_mint().await.unwrap();
//
//     let vault = client
//         .create_token_account(&funder, &mint.pubkey(), &vault_authority_key)
//         .await
//         .unwrap();
//
//     let funder_tokens = client
//         .create_token_account(&funder, &mint.pubkey(), &funder.pubkey())
//         .await
//         .unwrap();
//
//     client
//         .mint_to_token_account(
//             &funder,
//             &mint.pubkey(),
//             vault.pubkey(),
//             100,
//             &mint_authority,
//         )
//         .await
//         .expect("Failed to mint tokens");
//
//     let ixs = [
//         create_profile_ix(
//             &profile_key,
//             [
//                 AddProfileKey::new(&key, player_profile::ID, -1, ProfilePermissions::AUTH),
//                 AddProfileKey::new(
//                     &create_vault_key,
//                     profile_vault::ID,
//                     -1,
//                     ProfileVaultPermissions::CREATE_VAULT_AUTHORITY,
//                 ),
//                 AddProfileKey::new(
//                     &close_vault_key,
//                     profile_vault::ID,
//                     -1,
//                     ProfileVaultPermissions::CLOSE_VAULT,
//                 ),
//             ],
//             1,
//         ),
//         create_vault_authority_ix(
//             profile_key.pubkey(),
//             1,
//             &create_vault_key,
//             vault_seed.pubkey(),
//         ),
//         close_vault_ix(
//             profile_key.pubkey(),
//             2,
//             &close_vault_key,
//             vault.pubkey(),
//             vault_authority_key,
//             funder_tokens.pubkey(),
//             funder.pubkey(),
//         ),
//     ];
//     client.build_send_and_check(ixs, &funder).await.unwrap();
//
//     client
//         .get_token_account_info(&vault.pubkey())
//         .await
//         .expect("Vault should be closed");
// }
