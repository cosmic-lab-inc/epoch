use crate::errors::GcsError;
// use base64::{engine::general_purpose, Engine as _};
use gcp_bigquery_client::model::table_cell::TableCell;

pub fn i64_column(columns: &[TableCell], index: usize) -> anyhow::Result<i64> {
    let value = columns
        .get(index)
        .ok_or(GcsError::None)?
        .value
        .clone()
        .ok_or(GcsError::None)?;
    serde_json::from_value::<String>(value)?
        .parse::<i64>()
        .map_err(|_e| anyhow::anyhow!("value at index {} not i64", index))
}

pub fn string_column(columns: &[TableCell], index: usize) -> anyhow::Result<String> {
    let value = columns
        .get(index)
        .ok_or(GcsError::None)?
        .value
        .clone()
        .ok_or(GcsError::None)?;
    serde_json::from_value::<String>(value)
        .map_err(|_e| anyhow::anyhow!("value at index {} not String", index))
}

pub fn bool_column(columns: &[TableCell], index: usize) -> anyhow::Result<bool> {
    let value = columns
        .get(index)
        .ok_or(GcsError::None)?
        .value
        .clone()
        .ok_or(GcsError::None)?;
    serde_json::from_value::<String>(value)?
        .parse::<bool>()
        .map_err(|_e| anyhow::anyhow!("value at index {} not bool", index))
}
