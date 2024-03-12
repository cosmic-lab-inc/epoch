use crate::settings::DatabaseSettings;
use anyhow::anyhow;
use tokio_postgres::{Client, Statement};

pub struct StatementBuilder {}

impl StatementBuilder {
    pub async fn account_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/account.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare account statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn account_upsert_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/account_upsert.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare account upsert statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn account_delete_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/account_delete.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare account delete statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by hash statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_key_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_key.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by key statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_owner_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_owner.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by owner statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_slot_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_slot.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by slot statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_key_and_owner_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_key_and_owner.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by key and owner statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_key_and_slot_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_key_and_slot.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by key and slot statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_owner_and_slot_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_owner_and_slot.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by owner and slot statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }

    pub async fn accounts_by_key_and_owner_and_slot_statement(
        client: &Client,
        config: &DatabaseSettings,
    ) -> anyhow::Result<Statement> {
        let stmt = include_str!("../statements/accounts_by_key_and_owner_and_slot.sql");
        let stmt = client.prepare(stmt).await;

        match stmt {
            Ok(stmt) => Ok(stmt),
            Err(error) => {
                let error = anyhow!(
                    "Failed to prepare accounts by key and owner and slot statement: {} host: {:?}, user: {:?}, config: {:?}",
                    error,
                    config.host,
                    config.username,
                    config
                );
                Err(error)
            }
        }
    }
}
