use thiserror::Error;

#[derive(Debug, Error)]
pub enum GcsError {
    #[error("Gcloud error: {0}")]
    GcloudError(#[from] reqwest::Error),
}
