use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub struct DriftAssistant;
impl DriftAssistant {
    pub fn program_id(&self) -> anyhow::Result<Pubkey> {
        Ok(Pubkey::from_str(drift_cpi::PROGRAM_ID)?)
    }

    pub fn decode_name(name: &[u8; 32]) -> String {
        String::from_utf8(name.to_vec()).unwrap().trim().to_string()
    }

    pub fn user_stats_pda(&self, user_authority: &Pubkey) -> anyhow::Result<Pubkey> {
        let seeds: &[&[u8]] = &[b"user_stats", &user_authority.to_bytes()[..]];
        Ok(Pubkey::find_program_address(seeds, &self.program_id()?).0)
    }
}

pub const QUOTE_PRECISION: u128 = 1_000_000; // expo = -6
pub const PRICE_PRECISION: u128 = 1_000_000; //expo = -6;
