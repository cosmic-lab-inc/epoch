use crate::bq::account::BqAccountTrait;
use crate::bq::{BqAccount, Paginate};
use crate::errors::GcsError;
use common::ArchiveAccount;
use gcp_bigquery_client::error::BQError;
use gcp_bigquery_client::model::dataset::Dataset;
use gcp_bigquery_client::model::job_configuration_query::JobConfigurationQuery;
use gcp_bigquery_client::model::table::Table;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use gcp_bigquery_client::model::table_data_insert_all_request_rows::TableDataInsertAllRequestRows;
use gcp_bigquery_client::model::table_data_insert_all_response::TableDataInsertAllResponse;
use gcp_bigquery_client::Client;
use log::{debug, error, info, warn};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::path::Path;
use tokio_stream::StreamExt;

const BQ_PROJECT_ID: &str = "epoch-417015";
const BQ_DATASET_ID: &str = "epoch";
const BQ_ACCOUNTS_TABLE_ID: &str = "accounts";

pub struct BigQueryClient {
    pub client: Client,
    pub accounts_table: String,
}

impl BigQueryClient {
    pub async fn new(gcp_sa_key: &Path) -> anyhow::Result<Self> {
        let path = match gcp_sa_key.as_os_str().to_str() {
            Some(s) => Ok(s),
            None => Err(anyhow::Error::from(GcsError::FilePathInvalid)),
        }?;
        let client = Client::from_service_account_key_file(path).await?;
        let accounts_table = format!("`{BQ_PROJECT_ID}.{BQ_DATASET_ID}.{BQ_ACCOUNTS_TABLE_ID}`");
        // create table if not exists
        let this = Self {
            client,
            accounts_table,
        };
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
                debug!("Upserted accounts: {:?}", accounts.len());
                Ok(res)
            }
        }
    }

    pub async fn accounts(&self, query: &Paginate) -> anyhow::Result<Vec<ArchiveAccount>> {
        let result_set = self.client.job().query_all(
            BQ_PROJECT_ID,
            JobConfigurationQuery {
                query: format!(
                    "SELECT * FROM {} LIMIT {} OFFSET {}",
                    &self.accounts_table, query.limit, query.offset
                ),
                query_parameters: None,
                use_legacy_sql: Some(false),
                ..Default::default()
            },
            None,
        );
        tokio::pin!(result_set);

        let mut res = Vec::new();
        while let Some(page) = result_set.next().await {
            match page {
                Ok(rows) => {
                    let mut accts: Vec<ArchiveAccount> = rows
                        .into_iter()
                        .flat_map(|row| match BqAccount::try_from(row) {
                            Ok(bq) => bq.to_archive().ok(),
                            Err(e) => {
                                error!("Error converting BqAccount: {}", e);
                                None
                            }
                        })
                        .collect();
                    res.append(&mut accts)
                }
                Err(e) => Err(anyhow::Error::from(e))?,
            }
        }
        Ok(res)
    }
}
