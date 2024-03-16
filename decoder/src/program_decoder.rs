use base64::{engine::general_purpose, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use common::DecodeProgramAccount;
use drift_cpi::DriftAccountType;
use lazy_static::lazy_static;
use log::*;
use serde_json::Value;
use sol_chainsaw::network::IdlClient;
use sol_chainsaw::{ChainsawDeserializer, IdlProvider};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::hash;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

lazy_static! {
    /// Master list of supported programs that can provide decoded accounts based on an Anchor IDL.
    pub static ref PROGRAMS: Vec<Pubkey> = vec![Pubkey::from_str(drift_cpi::PROGRAM_ID).unwrap()];
}

/// Registry of program account decoders that match a discriminant,
/// such as "User", to a specific account type.
#[derive(BorshDeserialize, BorshSerialize)]
pub enum Decoder {
    Drift(DriftAccountType),
}

pub struct ProgramDecoder {
    pub chainsaw: ChainsawDeserializer<'static>,
    pub idls: HashMap<Pubkey, String>,
}

impl ProgramDecoder {
    pub fn new(rpc: RpcClient) -> anyhow::Result<Self> {
        info!("Initializing ProgramDecoder");
        let idl_client = IdlClient::for_anchor_on_rpc(rpc.url());
        let mut chainsaw = ChainsawDeserializer::new(&*Box::leak(Box::default()));

        let mut idls = HashMap::new();
        for program in PROGRAMS.iter() {
            info!("fetching idl for program: {}", program.to_string());
            let program_idl = match idl_client.fetch_idl(*program) {
                Ok(idl) => Ok(idl),
                Err(err) => Err(anyhow::anyhow!(
                    "Error fetching idl for program {}: {}",
                    program.to_string(),
                    err
                )),
            }?;
            info!("idl fetched");
            let idl = program_idl.json;
            chainsaw.add_idl_json(program.to_string(), &idl, IdlProvider::Anchor)?;
            info!("added idl");
            idls.insert(*program, idl);
        }

        Ok(Self { chainsaw, idls })
    }

    pub fn decode_account(
        program_id: &Pubkey,
        account_name: &str,
        data: &[u8],
    ) -> anyhow::Result<Decoder> {
        Ok(match program_id.to_string().as_str() {
            drift_cpi::PROGRAM_ID => {
                let discrim = Self::name_to_base64_discrim(account_name);
                Decoder::Drift(DriftAccountType::decode_account(&discrim, data)?)
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Program {} not supported",
                    program_id.to_string()
                ))
            }
        })
    }

    fn idl(&self, program_id: &Pubkey) -> anyhow::Result<String> {
        self.idls
            .get(program_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No IDL found for program"))
    }

    pub fn idl_accounts(&self, program_id: &Pubkey) -> anyhow::Result<Vec<String>> {
        let idl_str = self.idl(program_id)?;
        let idl = serde_json::from_str::<Value>(&idl_str)?;
        let accounts = serde_json::from_value::<Vec<Value>>(idl["accounts"].clone())?;
        Ok(accounts
            .iter()
            .map(|a| a["name"].as_str().unwrap().to_string())
            .collect::<Vec<String>>())
    }

    pub fn name_to_discrim(&self, account_name: &str) -> [u8; 8] {
        Self::account_discriminator(account_name)
    }

    pub fn discrim_to_name(
        &self,
        program_id: &Pubkey,
        account_discrim: &[u8; 8],
    ) -> Option<String> {
        self.chainsaw
            .account_name(&program_id.to_string(), account_discrim)
            .map(|name| name.to_string())
    }

    pub fn name_to_base64_discrim(account_name: &str) -> String {
        let bytes = Self::account_discriminator(account_name);
        general_purpose::STANDARD.encode(bytes)
    }

    pub fn base64_discrim_to_name(
        &self,
        program_id: &Pubkey,
        base64_discrim: &str,
    ) -> anyhow::Result<String> {
        let bytes = general_purpose::STANDARD.decode(base64_discrim)?;
        let discrim: [u8; 8] = bytes[..8].try_into()?;
        match self.discrim_to_name(program_id, &discrim) {
            Some(name) => Ok(name),
            None => Err(anyhow::anyhow!("No name found for base64 discriminator")),
        }
    }

    /// Derives the account discriminator form the account name as Anchor does.
    pub fn account_discriminator(name: &str) -> [u8; 8] {
        let mut discriminator = [0u8; 8];
        let hashed = hash(format!("account:{}", name).as_bytes()).to_bytes();
        discriminator.copy_from_slice(&hashed[..8]);
        discriminator
    }
}
