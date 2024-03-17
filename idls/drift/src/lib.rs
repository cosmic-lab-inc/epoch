// TODO: this code is all repeated except for the AccountType variants
// using the IDL, the list of account names can be discovered.
// The missing piece is using std::any::type_name<T>() to go for the IDL account name to the actual struct.
// To get the struct I would need to be able to iterate all exported types in this anchor-gen `typedefs` module.
// Reading the type at compile time isn't possible, it means a trait must be defined in anchor-gen
// that defines the string name of the type as std::any::type_name<Self>().
// The module then needs some way of iterating the types themselves with this string-type map.
anchor_gen::generate_cpi_crate!("idl.json");
anchor_lang::declare_id!("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");

use common::decode_account;
use once_cell::sync::Lazy;

pub static PATH: Lazy<String> = Lazy::new(|| env!("CARGO_MANIFEST_DIR").to_string());
pub static PROGRAM_NAME: Lazy<String> = Lazy::new(|| PATH.split('/').last().unwrap().to_string());
pub static IDL_PATH: Lazy<String> = Lazy::new(|| format!("{}/idl.json", *PATH));
pub static PROGRAM_ID: Lazy<Pubkey> = Lazy::new(|| ID);

decode_account!(
    pub enum AccountType {
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
