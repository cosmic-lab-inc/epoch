use crate::account::DbAccount;
use crate::settings::DatabaseSettings;
use crate::statement_builder::StatementBuilder;
use anyhow::anyhow;
use archive_stream::shorten_address;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use log::info;
use solana_sdk::pubkey::Pubkey;
use tokio_postgres::*;

pub struct PostgresClient {
    client: Client,

    account_stmt: Statement,
    account_delete_stmt: Statement,
    account_upsert_stmt: Statement,
    accounts_stmt: Statement,
    accounts_by_key_stmt: Statement,
    accounts_by_owner_stmt: Statement,
    accounts_by_slot_stmt: Statement,
}

impl PostgresClient {
    pub async fn new_from_url(connection_url: String) -> anyhow::Result<Self> {
        let client = match DatabaseSettings::new_from_url(connection_url) {
            Err(err) => return Err(anyhow!("Error loading configuration: {}", err)),
            Ok(config) => PostgresClient::new(&config).await?,
        };
        Ok(client)
    }

    pub async fn new(config: &DatabaseSettings) -> anyhow::Result<Self> {
        let pool = Self::connect_to_db(config).await?;
        let client = pool.dedicated_connection().await?;
        let account_stmt = StatementBuilder::account_statement(&client, config).await?;
        let account_delete_stmt =
            StatementBuilder::account_delete_statement(&client, config).await?;

        let account_upsert_stmt =
            StatementBuilder::account_upsert_statement(&client, config).await?;
        let accounts_stmt = StatementBuilder::accounts_statement(&client, config).await?;
        let accounts_by_key_stmt =
            StatementBuilder::accounts_by_key_statement(&client, config).await?;
        let accounts_by_owner_stmt =
            StatementBuilder::accounts_by_owner_statement(&client, config).await?;
        let accounts_by_slot_stmt =
            StatementBuilder::accounts_by_slot_statement(&client, config).await?;

        Ok(Self {
            client,
            account_stmt,
            account_delete_stmt,
            account_upsert_stmt,
            accounts_stmt,
            accounts_by_key_stmt,
            accounts_by_owner_stmt,
            accounts_by_slot_stmt,
        })
    }

    async fn connect_to_db(
        config: &DatabaseSettings,
    ) -> anyhow::Result<Pool<PostgresConnectionManager<NoTls>>> {
        let connection_string = if let Some(connection_string) = &config.connection_string {
            connection_string.clone()
        } else {
            if config.host.is_none() || config.username.is_none() {
                let error = anyhow::anyhow!("Missing host or username in database configuration");
                return Err(error);
            }
            if config.database_name.is_none() {
                format!(
                    "host={} user={} password={} port={}",
                    config.host.as_ref().unwrap(),
                    config.username.as_ref().unwrap(),
                    config.password.as_ref().unwrap(),
                    config.port.unwrap_or(5432)
                )
            } else {
                format!(
                    "host={} user={} password={} port={} dbname={}",
                    config.host.as_ref().unwrap(),
                    config.username.as_ref().unwrap(),
                    config.password.as_ref().unwrap(),
                    config.port.unwrap_or(5432),
                    config.database_name.as_ref().unwrap()
                )
            }
        };

        let config = connection_string.parse::<Config>()?;
        let manager = PostgresConnectionManager::new(config, NoTls);
        let pool = Pool::builder().build(manager).await?;

        Ok(pool)
    }

    // ================ QUERIES =================

    pub async fn account(&self, hash: u64) -> anyhow::Result<Vec<Row>> {
        let statement = &self.account_stmt;
        let client = &self.client;
        let result = client.query(statement, &[&(hash as i64)]).await;
        result.map_err(|err| anyhow!("Failed to get account: {}", err))
    }

    pub async fn account_delete(&self, hash: u64) -> anyhow::Result<Vec<Row>> {
        let statement = &self.account_delete_stmt;
        let client = &self.client;
        let result = client.query(statement, &[&(hash as i64)]).await;
        result.map_err(|err| anyhow!("Failed to delete account: {}", err))
    }

    pub async fn account_upsert(&self, account: &DbAccount) -> anyhow::Result<Vec<Row>> {
        let mut owner_arr = [0u8; 32];
        owner_arr.copy_from_slice(&account.owner);
        let owner = Pubkey::from(owner_arr);

        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&account.owner);
        let key = Pubkey::from(key_arr);
        info!(
            "upsert account: {}, slot: {}, owner: {}",
            shorten_address(&key),
            account.slot,
            shorten_address(&owner)
        );
        let statement = &self.account_upsert_stmt;
        let client = &self.client;
        let result = client
            .query(
                statement,
                &[
                    &account.hash,
                    &account.key.clone(),
                    &account.slot,
                    &account.lamports,
                    &account.owner,
                    &account.executable,
                    &account.rent_epoch,
                    &account.discriminant,
                    &account.data,
                ],
            )
            .await;
        result.map_err(|err| anyhow!("Failed to upsert account: {}", err))
    }

    pub async fn accounts(&self) -> anyhow::Result<Vec<Row>> {
        let statement = &self.accounts_stmt;
        let client = &self.client;
        let result = client.query(statement, &[]).await;
        result.map_err(|err| anyhow!("Failed to get all accounts: {}", err))
    }

    pub async fn accounts_by_key(&self, key: &Pubkey) -> anyhow::Result<Vec<Row>> {
        let statement = &self.accounts_by_key_stmt;
        let client = &self.client;
        let result = client.query(statement, &[&key.to_bytes().as_slice()]).await;
        result.map_err(|err| anyhow!("Failed to get accounts by key: {}", err))
    }

    pub async fn accounts_by_owner(&self, owner: &Pubkey) -> anyhow::Result<Vec<Row>> {
        let statement = &self.accounts_by_owner_stmt;
        let client = &self.client;
        let result = client
            .query(statement, &[&owner.to_bytes().as_slice()])
            .await;
        result.map_err(|err| anyhow!("Failed to get accounts by owner: {}", err))
    }

    pub async fn accounts_by_slot(&self, slot: u64) -> anyhow::Result<Vec<Row>> {
        let statement = &self.accounts_by_slot_stmt;
        let client = &self.client;
        let result = client.query(statement, &[&(slot as i64)]).await;
        result.map_err(|err| anyhow!("Failed to get accounts by slot: {}", err))
    }
}
