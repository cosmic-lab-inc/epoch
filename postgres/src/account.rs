use common::ArchiveAccount;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use tokio_postgres::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DbPubkey {
    pub key: String,
}

impl From<Pubkey> for DbPubkey {
    fn from(pubkey: Pubkey) -> Self {
        DbPubkey {
            key: pubkey.to_string(),
        }
    }
}

impl TryFrom<Vec<u8>> for DbPubkey {
    type Error = anyhow::Error;
    fn try_from(pubkey: Vec<u8>) -> anyhow::Result<Self> {
        Ok(DbPubkey {
            key: Pubkey::try_from(pubkey.as_slice())?.to_string(),
        })
    }
}

impl Eq for DbAccount {}

/// Shadows [`ArchiveAccount`] but with postgres compatible types.
/// u64 -> i64
/// Pubkey -> Vec<u8>
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DbAccount {
    /// unique has of key at this slot
    pub hash: i64,
    /// account key
    pub key: Vec<u8>,
    /// historical snapshot slot at which this state existed
    pub slot: i64,
    /// lamports in the account
    pub lamports: i64,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Vec<u8>,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: i64,
    /// First 8 bytes of data used to determine Anchor program account type
    pub discriminant: Option<Vec<u8>>,
    /// data held in this account
    pub data: Vec<u8>,
}

impl DbAccount {
    pub fn new(account: ArchiveAccount) -> Self {
        let discriminant = account.discrim().map(|d| d.to_vec());
        Self {
            hash: account.hash() as i64,
            key: account.key.to_bytes().to_vec(),
            slot: account.slot as i64,
            lamports: account.lamports as i64,
            owner: account.owner.to_bytes().to_vec(),
            executable: account.executable,
            rent_epoch: account.rent_epoch as i64,
            discriminant,
            data: account.data,
        }
    }
}

impl TryFrom<&Row> for DbAccount {
    type Error = anyhow::Error;
    fn try_from(row: &Row) -> anyhow::Result<Self> {
        Ok(Self {
            hash: row.get("hash"),
            key: row.get("key"),
            slot: row.get("slot"),
            lamports: row.get("lamports"),
            owner: row.get("owner"),
            executable: row.get("executable"),
            rent_epoch: row.get("rent_epoch"),
            discriminant: row.get("discriminant"),
            data: row.get("data"),
        })
    }
}

impl TryFrom<ArchiveAccount> for DbAccount {
    type Error = anyhow::Error;
    fn try_from(account: ArchiveAccount) -> anyhow::Result<Self> {
        Ok(DbAccount::new(account))
    }
}

pub trait FromDbAccount: Sized {
    fn from_db_account(account: DbAccount) -> anyhow::Result<Self>;
}

impl FromDbAccount for ArchiveAccount {
    fn from_db_account(account: DbAccount) -> anyhow::Result<Self> {
        Ok(ArchiveAccount {
            key: Pubkey::new_from_array(account.key.as_slice().try_into()?),
            slot: account.slot as u64,
            lamports: account.lamports as u64,
            owner: Pubkey::new_from_array(account.owner.as_slice().try_into()?),
            executable: account.executable,
            rent_epoch: account.rent_epoch as u64,
            data: account.data,
        })
    }
}
