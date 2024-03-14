use crate::drift::snapshot::DriftTraderSnapshot;
use crate::drift::trader::DriftTrader;
use common::serde::serialize_pubkey;
use serde::{Deserialize, Serialize};
use solana_sdk::clock::Slot;
use solana_sdk::pubkey::Pubkey;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TraderStats {
    #[serde(serialize_with = "serialize_pubkey")]
    pub authority: Pubkey,
    pub settled_perp_pnl: f64,
    pub total_deposits: f64,
    pub roi: Option<f64>,
    pub taker_volume_30d: f64,
    pub maker_volume_30d: f64,
    pub pnl_per_volume: f64,
    pub slot: Option<Slot>,
}

impl From<DriftTrader> for TraderStats {
    fn from(trader: DriftTrader) -> Self {
        TraderStats {
            authority: trader.authority,
            settled_perp_pnl: trader.settled_perp_pnl(),
            total_deposits: trader.total_deposits(),
            roi: trader.roi(),
            taker_volume_30d: trader.taker_volume_30d(),
            maker_volume_30d: trader.maker_volume_30d(),
            pnl_per_volume: trader.pnl_per_volume(),
            slot: None,
        }
    }
}

impl From<DriftTraderSnapshot> for TraderStats {
    fn from(snapshot: DriftTraderSnapshot) -> Self {
        TraderStats {
            authority: snapshot.trader.authority,
            settled_perp_pnl: snapshot.trader.settled_perp_pnl(),
            total_deposits: snapshot.trader.total_deposits(),
            roi: snapshot.trader.roi(),
            taker_volume_30d: snapshot.trader.taker_volume_30d(),
            maker_volume_30d: snapshot.trader.maker_volume_30d(),
            pnl_per_volume: snapshot.trader.pnl_per_volume(),
            slot: Some(snapshot.slot),
        }
    }
}

impl Display for TraderStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TraderStats: authority: {}, settled_perp_pnl: {}, total_deposits: {}, roi: {:?}, taker_volume_30d: {}, \
            maker_volume_30d: {}, pnl_per_volume: {}, slot: {}",
            self.authority,
            self.settled_perp_pnl,
            self.total_deposits,
            self.roi,
            self.taker_volume_30d,
            self.maker_volume_30d,
            self.pnl_per_volume,
            match self.slot {
                Some(slot) => slot.to_string(),
                None => "None".to_string(),
            }
        )
    }
}
