use crate::BackfillError;
use serde::{Deserialize, Deserializer};
use solana_sdk::pubkey::Pubkey;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Deserialize)]
pub struct BackfillConfig {
    /// Number of threads to ETL (extract, transform, and load) snapshots.
    pub max_workers: usize,
    /// Only parse and store accounts for these programs.
    #[serde(deserialize_with = "BackfillConfig::deserialize_pubkey")]
    pub programs: Vec<Pubkey>,
    /// Only parse and store historical data for these slots.
    pub slots: Vec<u64>,
    /// Google cloud storage bucket to pull snapshots from
    pub gcs_bucket: String,
    /// Optional local file path to gcs [`ObjectResponse`]. Primarily for development to speed up iteration.
    pub gcs_local_file: Option<String>,
    /// GCS service account JSON file
    pub gcs_sa_key: String,
}

impl BackfillConfig {
    fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Vec<Pubkey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let keys: Vec<String> = Vec::deserialize(deserializer)?;
        keys.into_iter()
            .map(|pubkey| Pubkey::from_str(&pubkey).map_err(serde::de::Error::custom))
            .collect()
    }

    pub fn read_config(path: &PathBuf) -> anyhow::Result<BackfillConfig> {
        let contents = String::from_utf8(std::fs::read(path)?)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
}
