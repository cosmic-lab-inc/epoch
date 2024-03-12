use archive_stream::ArchiveAccount;
use base64::{engine::general_purpose, Engine as _};
use gcp_bigquery_client::model::table_field_schema::TableFieldSchema;
use gcp_bigquery_client::model::table_schema::TableSchema;
use serde::{Deserialize, Serialize};

impl Eq for BqAccount {}

/// Shadows [`ArchiveAccount`] but with postgres compatible types.
/// u64 -> i64
/// Pubkey -> String
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BqAccount {
    pub hash: i64,
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
            hash: account.hash() as i64,
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

// impl TryFrom<&Row> for BqAccount {
//     type Error = anyhow::Error;
//     fn try_from(row: &Row) -> anyhow::Result<Self> {
//         Ok(Self {
//             hash: row.get("hash"),
//             key: row.get("key"),
//             slot: row.get("slot"),
//             lamports: row.get("lamports"),
//             owner: row.get("owner"),
//             executable: row.get("executable"),
//             rent_epoch: row.get("rent_epoch"),
//             discriminant: row.get("discriminant"),
//             data: row.get("data"),
//         })
//     }
// }

impl TryFrom<ArchiveAccount> for BqAccount {
    type Error = anyhow::Error;
    fn try_from(account: ArchiveAccount) -> anyhow::Result<Self> {
        Ok(BqAccount::new(account))
    }
}

pub trait AccountTableSchema {
    fn to_schema() -> TableSchema;
}

impl AccountTableSchema for BqAccount {
    fn to_schema() -> TableSchema {
        // let mut data = TableFieldSchema::bytes("data");
        // data.mode = Some("REPEATED".to_string());
        let data = TableFieldSchema::string("data");

        TableSchema::new(vec![
            TableFieldSchema::integer("hash"),
            TableFieldSchema::string("key"),
            TableFieldSchema::integer("slot"),
            TableFieldSchema::integer("lamports"),
            TableFieldSchema::string("owner"),
            TableFieldSchema::bool("executable"),
            TableFieldSchema::integer("rent_epoch"),
            data,
        ])
    }
}
