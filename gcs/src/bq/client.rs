use crate::{bq::*, errors::GcsError};
use common::types::*;
use decoder::ProgramDecoder;
use gcp_bigquery_client::{
    error::BQError,
    model::{
        dataset::Dataset, job_configuration_query::JobConfigurationQuery, table::Table,
        table_data_insert_all_request::TableDataInsertAllRequest,
        table_data_insert_all_request_rows::TableDataInsertAllRequestRows,
        table_data_insert_all_response::TableDataInsertAllResponse, table_row::TableRow,
    },
    Client,
};
use log::{debug, error, info, warn};
use rayon::{iter::ParallelIterator, prelude::IntoParallelRefIterator};
use std::path::Path;
use tokio_stream::{Stream, StreamExt};

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
                    insert_id: Some(account.id.to_string()),
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

    //
    // ================================== READ QUERIES ================================== //
    //

    /// Helper function to collect a streamed response from BigQuery
    async fn read_stream(
        mut stream: impl Stream<Item = Result<Vec<TableRow>, BQError>> + Sized + Unpin,
    ) -> anyhow::Result<Vec<ArchiveAccount>> {
        let mut res = Vec::new();
        while let Some(page) = stream.next().await {
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

    pub async fn account_id(
        &self,
        query: &QueryAccountId,
    ) -> anyhow::Result<Option<ArchiveAccount>> {
        let res = self.client.job().query_all(
            BQ_PROJECT_ID,
            JobConfigurationQuery {
                query: format!(
                    "SELECT * FROM {} WHERE id = {} LIMIT 1",
                    &self.accounts_table, query.id
                ),
                query_parameters: None,
                use_legacy_sql: Some(false),
                ..Default::default()
            },
            None,
        );
        tokio::pin!(res);
        let vec = Self::read_stream(res).await?;
        Ok(vec.first().cloned())
    }

    fn build_accounts_query(&self, params: &QueryAccounts) -> String {
        let mut query = format!("SELECT * FROM {}", &self.accounts_table);
        let mut where_added = false;

        if let Some(key) = &params.key {
            query = format!("{} WHERE key = \"{}\"", &query, key);
            where_added = true;
        }
        if let Some(slot) = &params.slot {
            let clause = match where_added {
                false => "WHERE",
                true => "AND",
            };
            query = format!("{} {} slot = {}", &query, clause, slot);
            where_added = true;
        }
        if let Some(owner) = &params.owner {
            let clause = match where_added {
                false => "WHERE",
                true => "AND",
            };
            query = format!("{} {} owner = \"{}\"", &query, clause, owner);
            where_added = true;
        }

        query = format!("{} LIMIT {} OFFSET {}", &query, params.limit, params.offset);
        query
    }

    pub async fn accounts(&self, params: &QueryAccounts) -> anyhow::Result<Vec<ArchiveAccount>> {
        let query = self.build_accounts_query(params);
        let res = self.client.job().query_all(
            BQ_PROJECT_ID,
            JobConfigurationQuery {
                query,
                query_parameters: None,
                use_legacy_sql: Some(false),
                ..Default::default()
            },
            None,
        );
        tokio::pin!(res);
        Self::read_stream(res).await
    }

    fn build_decoded_accounts_query(
        &self,
        params: &QueryDecodedAccounts,
    ) -> anyhow::Result<String> {
        let mut query = format!("SELECT * FROM {}", &self.accounts_table);
        let mut where_added = false;

        if let Some(key) = &params.key {
            query = format!("{} WHERE key = \"{}\"", &query, key);
            where_added = true;
        }
        if let Some(slot) = &params.slot {
            let clause = match where_added {
                false => "WHERE",
                true => "AND",
            };
            query = format!("{} {} slot = {}", &query, clause, slot);
            where_added = true;
        }

        let clause = match where_added {
            false => "WHERE",
            true => "AND",
        };
        query = format!("{} {} owner = \"{}\"", &query, clause, params.owner);
        where_added = true;

        let base64_discrim = ProgramDecoder::name_to_base64_discrim(&params.discriminant);
        query = format!(
            "{} AND TO_BASE64(SUBSTR(FROM_BASE64(data), 1, 8)) = \"{}\"",
            &query, base64_discrim
        );

        query = format!("{} LIMIT {} OFFSET {}", &query, params.limit, params.offset);
        info!("decoded accounts query: {:#?}", query);
        Ok(query)
    }

    pub async fn account_type(
        &self,
        params: &QueryDecodedAccounts,
    ) -> anyhow::Result<Vec<ArchiveAccount>> {
        let query = self.build_decoded_accounts_query(params)?;
        let res = self.client.job().query_all(
            BQ_PROJECT_ID,
            JobConfigurationQuery {
                query,
                query_parameters: None,
                use_legacy_sql: Some(false),
                ..Default::default()
            },
            None,
        );
        tokio::pin!(res);
        Self::read_stream(res).await
    }
}
