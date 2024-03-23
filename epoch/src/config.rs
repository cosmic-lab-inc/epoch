use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct EpochConfig {
    /// GCS service account JSON file
    pub gcs_sa_key: String,
    /// Solana RPC endpoint
    pub solana_rpc: String,
    /// Redis username
    pub redis_username: String,
    /// Redis password
    pub redis_password: String,
    /// Redis host
    pub redis_host: String,
    /// Redis port
    pub redis_port: u16,
}

impl EpochConfig {
    pub fn read_config(path: &PathBuf) -> anyhow::Result<EpochConfig> {
        let contents = String::from_utf8(std::fs::read(path)?)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
}
