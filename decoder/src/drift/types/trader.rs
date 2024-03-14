use crate::drift::program::math::QUOTE_PRECISION;
use crate::drift::program::user::{User, UserStats};
use common::serde::serialize_pubkey;
use common::trunc;
use common::KeyedAccount;
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;

/// User authority pubkey
pub type Authority = Pubkey;

#[derive(Debug, Serialize)]
pub struct DriftTrader {
    #[serde(serialize_with = "serialize_pubkey")]
    pub authority: Pubkey,
    pub user_stats: KeyedAccount<UserStats>,
    pub users: Vec<KeyedAccount<User>>,
}

impl DriftTrader {
    pub fn settled_perp_pnl(&self) -> f64 {
        // iterate each UserAccountInfo.account.settled_perp_pnl and sum
        let sum: f64 = self
            .users
            .iter()
            .map(|u| (u.account.settled_perp_pnl as f64) / (QUOTE_PRECISION as f64))
            .sum();
        trunc!(sum, 3)
    }
    pub fn total_deposits(&self) -> f64 {
        // iterate each UserAccountInfo.account.total_deposits and sum
        let sum: f64 = self
            .users
            .iter()
            .map(|u| (u.account.total_deposits as f64) / (QUOTE_PRECISION as f64))
            .sum();
        trunc!(sum, 3)
    }
    pub fn taker_volume_30d(&self) -> f64 {
        trunc!(
            self.user_stats.account.taker_volume_30d as f64 / (QUOTE_PRECISION as f64),
            3
        )
    }
    pub fn maker_volume_30d(&self) -> f64 {
        trunc!(
            self.user_stats.account.maker_volume_30d as f64 / (QUOTE_PRECISION as f64),
            3
        )
    }
    pub fn roi(&self) -> Option<f64> {
        match self.total_deposits() > 0_f64 {
            true => Some(trunc!(self.settled_perp_pnl() / self.total_deposits(), 3)),
            false => None,
        }
    }
    pub fn pnl_per_volume(&self) -> f64 {
        trunc!(
            self.settled_perp_pnl() / (self.taker_volume_30d() + self.maker_volume_30d()),
            3
        )
    }
}
