anchor_gen::generate_cpi_crate!("idl.json");
anchor_lang::declare_id!("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");

use borsh::{BorshDeserialize, BorshSerialize};
use common::{decode_account, DecodeProgramAccount};
// All accounts from anchor_gen are behind this mod. Do not delete it
use crate::typedefs::*;

pub const PROGRAM_ID: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";

#[allow(clippy::large_enum_variant)]
decode_account!(
    pub enum DriftAccountType {
        PhoenixV1FulfillmentConfig(PhoenixV1FulfillmentConfig),
        SerumV3FulfillmentConfig(SerumV3FulfillmentConfig),
        InsuranceFundStake(InsuranceFundStake),
        ProtocolIfSharesTransferConfig(ProtocolIfSharesTransferConfig),
        PerpMarket(PerpMarket),
        SpotMarket(SpotMarket),
        State(State),
        User(User),
        UserStats(UserStats),
        ReferrerName(ReferrerName),
    }
);
