use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct EpochConfig {
    /// GCS service account JSON file
    pub gcs_sa_key: String,

    /// Solana RPC endpoint
    pub solana_rpc: String,
}

impl EpochConfig {
    pub fn read_config(path: &PathBuf) -> anyhow::Result<EpochConfig> {
        let contents = String::from_utf8(std::fs::read(path)?)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
}
