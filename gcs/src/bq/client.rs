use crate::bq::{AccountTableSchema, BqAccount};
use crate::errors::GcsError;
use gcp_bigquery_client::error::BQError;
use gcp_bigquery_client::model::dataset::Dataset;
use gcp_bigquery_client::model::table::Table;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use gcp_bigquery_client::model::table_data_insert_all_request_rows::TableDataInsertAllRequestRows;
use gcp_bigquery_client::model::table_data_insert_all_response::TableDataInsertAllResponse;
use gcp_bigquery_client::Client;
use log::{error, info, warn};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::path::Path;

const BQ_PROJECT_ID: &str = "epoch-417015";
const BQ_DATASET_ID: &str = "epoch";
const BQ_ACCOUNTS_TABLE_ID: &str = "accounts";

pub struct BigQueryClient {
    pub client: Client,
}

impl BigQueryClient {
    pub async fn new(gcp_sa_key: &Path) -> anyhow::Result<Self> {
        let path = match gcp_sa_key.as_os_str().to_str() {
            Some(s) => Ok(s),
            None => Err(anyhow::Error::from(GcsError::FilePathInvalid)),
        }?;
        let client = Client::from_service_account_key_file(path).await?;
        // create table if not exists
        let this = Self { client };
        this.create_accounts_table().await?;
        Ok(this)
    }

    pub async fn dataset(&self) -> anyhow::Result<Dataset> {
        Ok(self
            .client
            .dataset()
            .get(BQ_PROJECT_ID, BQ_DATASET_ID)
            .await?)
    }

    pub async fn create_accounts_table(&self) -> anyhow::Result<()> {
        let res = self
            .dataset()
            .await?
            .create_table(
                &self.client,
                Table::from_dataset(
                    &self.dataset().await?,
                    BQ_ACCOUNTS_TABLE_ID,
                    BqAccount::to_schema(),
                )
                .friendly_name("Unparsed Accounts")
                .description("Unparsed Solana accounts (serialized account data)"),
            )
            .await;
        // if status is ALREADY_EXISTS return Ok
        match res {
            Err(e) => {
                if let BQError::ResponseError { error } = &e {
                    if error.error.status == "ALREADY_EXISTS" {
                        warn!("BigQuery accounts table already exists");
                        Ok(())
                    } else {
                        error!("Error creating BigQuery accounts table: {:?}", error);
                        Err(anyhow::Error::from(e))
                    }
                } else {
                    error!("Error creating BigQuery accounts table: {:?}", e);
                    Err(anyhow::Error::from(e))
                }
            }
            Ok(_) => {
                info!("Created BigQuery accounts table");
                Ok(())
            }
        }
    }

    pub async fn upsert_accounts(
        &self,
        accounts: Vec<BqAccount>,
    ) -> anyhow::Result<TableDataInsertAllResponse> {
        let rows: Vec<TableDataInsertAllRequestRows> = accounts
            .par_iter()
            .map(|account| {
                Result::<_, anyhow::Error>::Ok(TableDataInsertAllRequestRows {
                    insert_id: Some(account.hash.to_string()),
                    json: serde_json::to_value(account)?,
                })
            })
            .flatten()
            .collect();

        let mut req = TableDataInsertAllRequest::new();
        req.add_rows(rows)?;

        let res = self
            .client
            .tabledata()
            .insert_all(BQ_PROJECT_ID, BQ_DATASET_ID, BQ_ACCOUNTS_TABLE_ID, req)
            .await?;
        let errors = &res.insert_errors;
        match errors {
            Some(errors) => {
                for error in errors {
                    error!("Error upserting account: {:?}", error);
                }
                Err(anyhow::Error::from(GcsError::BigQueryUpsertError))
            }
            None => {
                info!("Upserted accounts: {:?}", accounts.len());
                Ok(res)
            }
        }
    }
}
