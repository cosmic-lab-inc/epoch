use crate::drift::program::math::QUOTE_PRECISION;
use crate::drift::program::order::Order;
use crate::drift::program::perp::PerpPosition;
use crate::drift::program::spot::SpotPosition;
use bytemuck::{CheckedBitPattern, NoUninit};
use common::serde::serialize_pubkey;
use common::{trunc, CheckedArray};
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;

/// ```typescript
/// export type UserAccount = {
///   authority: PublicKey;
///   delegate: PublicKey;
///   name: number[];
///   subAccountId: number;
///   spotPositions: SpotPosition[];
///   perpPositions: PerpPosition[];
///   orders: Order[];
///   status: number;
///   nextLiquidationId: number;
///   nextOrderId: number;
///   maxMarginRatio: number;
///   lastAddPerpLpSharesTs: BN;
///   settledPerpPnl: BN;
///   totalDeposits: BN;
///   totalWithdraws: BN;
///   totalSocialLoss: BN;
///   cumulativePerpFunding: BN;
///   cumulativeSpotFees: BN;
///   liquidationMarginFreed: BN;
///   lastActiveSlot: BN;
///   isMarginTradingEnabled: boolean;
///   idle: boolean;
///   openOrders: number;
///   hasOpenOrder: boolean;
///   openAuctions: number;
///   hasOpenAuction: boolean;
/// };
/// ```
#[derive(Debug, Copy, Clone, CheckedBitPattern, NoUninit, Serialize)]
#[repr(C, packed)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(serialize_with = "serialize_pubkey")]
    pub authority: Pubkey,
    #[serde(serialize_with = "serialize_pubkey")]
    pub delegate: Pubkey,
    #[serde(with = "serde_bytes")]
    pub name: [u8; 32],
    pub spot_positions: CheckedArray<SpotPosition, 8>, // [SpotPosition; 8],
    pub perp_positions: CheckedArray<PerpPosition, 8>, // [PerpPosition; 8],
    pub orders: CheckedArray<Order, 32>,               // [Order; 32],
    pub last_add_perp_lp_shares_ts: i64,
    pub total_deposits: u64,
    pub total_withdraws: u64,
    pub total_social_loss: u64,
    pub settled_perp_pnl: i64,
    pub cumulative_spot_fees: i64,
    pub cumulative_perp_funding: i64,
    pub liquidation_margin_freed: u64,
    pub last_active_slot: u64,
    pub next_order_id: u32,
    pub max_margin_ratio: u32,
    pub next_liquidation_id: u16,
    pub sub_account_id: u16,
    pub status: u8,
    pub is_margin_trading_enabled: bool,
    pub idle: bool,
    pub open_orders: u8,
    pub has_open_order: bool,
    pub open_auctions: u8,
    pub has_open_auction: bool,
    #[serde(with = "serde_bytes")]
    pub padding: [u8; 21],
}
impl User {
    pub fn settled_perp_pnl(&self) -> f64 {
        // iterate each UserAccountInfo.account.settled_perp_pnl and sum
        let pnl: f64 = self.settled_perp_pnl as f64 / QUOTE_PRECISION as f64;
        trunc!(pnl, 3)
    }
    pub fn total_deposits(&self) -> f64 {
        // iterate each UserAccountInfo.account.total_deposits and sum
        let deposits: f64 = self.total_deposits as f64 / QUOTE_PRECISION as f64;
        trunc!(deposits, 3)
    }
    pub fn roi(&self) -> Option<f64> {
        match self.total_deposits() > 0_f64 {
            true => Some(trunc!(self.settled_perp_pnl() / self.total_deposits(), 3)),
            false => None,
        }
    }
}

/// ```typescript
/// export type UserStatsAccount = {
///   numberOfSubAccounts: number;
///   numberOfSubAccountsCreated: number;
///   makerVolume30D: BN;
///   takerVolume30D: BN;
///   fillerVolume30D: BN;
///   lastMakerVolume30DTs: BN;
///   lastTakerVolume30DTs: BN;
///   lastFillerVolume30DTs: BN;
///   fees: {
///     totalFeePaid: BN;
///     totalFeeRebate: BN;
///     totalTokenDiscount: BN;
///     totalRefereeDiscount: BN;
///     totalReferrerReward: BN;
///     current_epoch_referrer_reward: BN;
///   };
///   referrer: PublicKey;
///   isReferrer: boolean;
///   authority: PublicKey;
///   ifStakedQuoteAssetAmount: BN;
/// };
/// ```
#[derive(Serialize, Debug, Copy, Clone, CheckedBitPattern, NoUninit)]
#[repr(C, packed)]
pub struct UserStats {
    #[serde(serialize_with = "serialize_pubkey")]
    pub authority: Pubkey,
    #[serde(serialize_with = "serialize_pubkey")]
    pub referrer: Pubkey,
    pub fees: UserFees,
    pub next_epoch_ts: i64,
    pub maker_volume_30d: u64,
    pub taker_volume_30d: u64,
    pub filler_volume_30d: u64,
    pub last_maker_volume_30d_ts: i64,
    pub last_taker_volume_30d_ts: i64,
    pub last_filler_volume_30d_ts: i64,
    pub if_staked_quote_asset_amount: u64,
    pub number_of_sub_accounts: u16,
    pub number_of_sub_accounts_created: u16,
    pub is_referrer: bool,
    pub disable_update_perp_bid_ask_twap: bool,
    #[serde(with = "serde_bytes")]
    pub padding: [u8; 50],
}

#[derive(Serialize, Debug, Copy, Clone, CheckedBitPattern, NoUninit)]
#[repr(C, packed)]
pub struct UserFees {
    pub total_fee_paid: u64,
    pub total_fee_rebate: u64,
    pub total_token_discount: u64,
    pub total_referee_discount: u64,
    pub total_referrer_reward: u64,
    pub current_epoch_referrer_reward: u64,
}
