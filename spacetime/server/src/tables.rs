use spacetimedb::{spacetimedb, Identity, Timestamp};

#[spacetimedb(table)]
pub struct User {
    #[primarykey]
    pub identity: Identity,
    pub name: Option<String>,
    pub online: bool,
}

#[spacetimedb(table)]
pub struct Message {
    pub sender: Identity,
    pub sent: Timestamp,
    pub text: String,
}

/// Duplicates [`ArchiveAccount`] in epoch/archive_stream crate and adds identity field for spacetime db lookups.
/// The identity is just the account key cast to a spacetime Identity. They're both [u8; 32], so the data is the same.
/// Can't import anything from epoch to this since this crate compiles into WASM, and epoch does not.
/// However, this crate can be imported to epoch just fine.
#[spacetimedb(table)]
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
