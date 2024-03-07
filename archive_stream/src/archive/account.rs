use serde::{Deserialize, Serialize};
use solana_sdk::clock::{Epoch, Slot};
use solana_sdk::pubkey::Pubkey;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveAccount {
    pub key: Pubkey,
    /// historical snapshot slot at which this state existed
    pub slot: Slot,
    /// lamports in the account
    pub lamports: u64,
    /// data held in this account
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
}
