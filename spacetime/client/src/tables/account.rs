use crate::module::ReducerEvent;
use archive_stream::ArchiveAccount;
use solana_sdk::pubkey::Pubkey;
use spacetimedb_sdk::anyhow;
use spacetimedb_sdk::sats::{de::Deserialize, ser::Serialize};
#[allow(unused)] // NOTE: this is required to get serde macro to work
use spacetimedb_sdk::spacetimedb_lib;
use spacetimedb_sdk::table::{TableIter, TableType};
use std::str::FromStr;

// /// Duplicates [`ArchiveAccount`] in epoch/archive_stream crate and adds identity field for spacetime db lookups.
// /// The identity is just the account key cast to a spacetime Identity. They're both [u8; 32], so the data is the same.
// /// Can't import anything from epoch to this since this crate compiles into WASM, and epoch does not.
// /// However, this crate can be imported to epoch just fine.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SpacetimeAccount {
    /// account key
    pub key: String,
    /// historical snapshot slot at which this state existed
    pub slot: u64,
    /// lamports in the account
    pub lamports: u64,
    /// data held in this account
    pub data: Vec<u8>,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: String,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: u64,
}

impl From<ArchiveAccount> for SpacetimeAccount {
    fn from(account: ArchiveAccount) -> Self {
        Self {
            key: account.key.to_string(),
            slot: account.slot,
            lamports: account.lamports,
            data: account.data, // TODO: Cow?
            owner: account.owner.to_string(),
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }
}

pub trait FromSpacetimeAccount {
    fn from_spacetime(account: SpacetimeAccount) -> anyhow::Result<ArchiveAccount>;
}
impl FromSpacetimeAccount for ArchiveAccount {
    fn from_spacetime(account: SpacetimeAccount) -> anyhow::Result<Self> {
        Ok(Self {
            key: Pubkey::from_str(&account.key)?,
            slot: account.slot,
            lamports: account.lamports,
            data: account.data,
            owner: Pubkey::from_str(&account.owner)?,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        })
    }
}

impl TableType for SpacetimeAccount {
    const TABLE_NAME: &'static str = "Account";
    type ReducerEvent = ReducerEvent;
}

impl SpacetimeAccount {
    #[allow(unused)]
    pub fn filter_by_key(key: String) -> TableIter<Self> {
        Self::filter(|row| row.key == key)
    }
}
