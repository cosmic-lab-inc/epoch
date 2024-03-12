use postgres_client::DbAccount;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EpochAccount {
    /// hash that is unique to the key at this slot
    pub hash: u64,
    /// address of this account on-chain
    pub key: String,
    /// historical snapshot slot at which this state existed
    pub slot: u64,
    /// lamports in the account
    pub lamports: u64,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: String,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: u64,
    /// data held in this account
    pub data: Vec<u8>,
}

impl TryFrom<DbAccount> for EpochAccount {
    type Error = anyhow::Error;
    fn try_from(account: DbAccount) -> anyhow::Result<Self> {
        Ok(Self {
            hash: account.hash as u64,
            key: Pubkey::new_from_array(account.key.as_slice().try_into()?).to_string(),
            slot: account.slot as u64,
            lamports: account.lamports as u64,
            owner: Pubkey::new_from_array(account.owner.as_slice().try_into()?).to_string(),
            executable: account.executable,
            rent_epoch: account.rent_epoch as u64,
            data: account.data,
        })
    }
}
