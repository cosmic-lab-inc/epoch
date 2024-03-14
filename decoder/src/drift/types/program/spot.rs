use bytemuck::{CheckedBitPattern, NoUninit};
use serde::Serialize;

/// ```typescript
/// export class SpotBalanceType {
///   static readonly DEPOSIT = { deposit: {} };
///   static readonly BORROW = { borrow: {} };
/// }
/// ```
#[derive(Debug, Clone, Copy, CheckedBitPattern, NoUninit, Serialize)]
#[repr(u8)]
#[serde(rename_all = "PascalCase")]
pub enum SpotBalanceType {
    Deposit,
    Borrow,
}

/// ```typescript
/// export type SpotPosition = {
///   marketIndex: number;
///   balanceType: SpotBalanceType;
///   scaledBalance: BN;
///   openOrders: number;
///   openBids: BN;
///   openAsks: BN;
///   cumulativeDeposits: BN;
/// };
/// ```
#[derive(Debug, Copy, Clone, CheckedBitPattern, NoUninit, Serialize)]
#[repr(C, packed)]
#[serde(rename_all = "camelCase")]
pub struct SpotPosition {
    pub scaled_balance: u64,
    pub open_bids: i64,
    pub open_asks: i64,
    pub cumulative_deposits: i64,
    pub market_index: u16,
    pub balance_type: SpotBalanceType,
    pub open_orders: u8,
    #[serde(with = "serde_bytes")]
    pub padding: [u8; 4],
}
