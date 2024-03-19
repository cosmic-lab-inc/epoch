use crate::{bq::column::*, errors::GcsError};
use base64::{engine::general_purpose, Engine as _};
use common::ArchiveAccount;
use gcp_bigquery_client::model::{
    table_field_schema::TableFieldSchema, table_row::TableRow, table_schema::TableSchema,
};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

impl Eq for BqAccount {}

/// Shadows [`ArchiveAccount`] but with postgres compatible types.
/// u64 -> i64
/// Pubkey -> String
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BqAccount {
    pub id: i64,
    /// account key
    pub key: String,
    /// historical snapshot slot at which this state existed
    pub slot: i64,
    /// lamports in the account
    pub lamports: i64,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: String,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: i64,
    /// data held in this account
    pub data: String,
}

impl BqAccount {
    pub fn new(account: ArchiveAccount) -> Self {
        let data = general_purpose::STANDARD.encode(&account.data);
        Self {
            id: account.id() as i64,
            key: account.key.to_string(),
            slot: account.slot as i64,
            lamports: account.lamports as i64,
            owner: account.owner.to_string(),
            executable: account.executable,
            rent_epoch: account.rent_epoch as i64,
            data,
        }
    }
}

impl TryFrom<TableRow> for BqAccount {
    type Error = anyhow::Error;
    fn try_from(row: TableRow) -> anyhow::Result<Self> {
        let columns = row.columns.ok_or(GcsError::None)?;
        Ok(Self {
            id: i64_column(&columns, 0)?,
            key: string_column(&columns, 1)?,
            slot: i64_column(&columns, 2)?,
            lamports: i64_column(&columns, 3)?,
            owner: string_column(&columns, 4)?,
            executable: bool_column(&columns, 5)?,
            rent_epoch: i64_column(&columns, 6)?,
            data: string_column(&columns, 7)?,
        })
    }
}

impl TryFrom<ArchiveAccount> for BqAccount {
    type Error = anyhow::Error;
    fn try_from(account: ArchiveAccount) -> anyhow::Result<Self> {
        Ok(BqAccount::new(account))
    }
}

pub trait BqAccountTrait {
    fn to_schema() -> TableSchema;
    fn to_archive(&self) -> anyhow::Result<ArchiveAccount>;
}

impl BqAccountTrait for BqAccount {
    fn to_schema() -> TableSchema {
        TableSchema::new(vec![
            TableFieldSchema::integer("id"),
            TableFieldSchema::string("key"),
            TableFieldSchema::integer("slot"),
            TableFieldSchema::integer("lamports"),
            TableFieldSchema::string("owner"),
            TableFieldSchema::bool("executable"),
            TableFieldSchema::integer("rent_epoch"),
            TableFieldSchema::string("data"),
        ])
    }

    fn to_archive(&self) -> anyhow::Result<ArchiveAccount> {
        Ok(ArchiveAccount {
            key: Pubkey::from_str(&self.key)?,
            slot: self.slot as u64,
            lamports: self.lamports as u64,
            owner: Pubkey::from_str(&self.owner)?,
            executable: self.executable,
            rent_epoch: self.rent_epoch as u64,
            data: general_purpose::STANDARD.decode(&self.data).unwrap(),
        })
    }
}
