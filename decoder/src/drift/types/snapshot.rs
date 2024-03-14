use crate::drift::trader::DriftTrader;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DriftTraderSnapshot {
    pub slot: u64,
    pub trader: DriftTrader,
}
