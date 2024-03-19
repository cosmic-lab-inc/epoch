use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use thiserror::Error;

pub type EpochResult<T> = Result<T, EpochError>;

#[derive(Debug, Error)]
pub enum EpochError {
    #[error("SerdeYaml: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    #[allow(unused)]
    #[error("Init logger error")]
    InitLogger,

    #[allow(unused)]
    #[error("Internal Server Error")]
    InternalServerError(String),

    #[allow(unused)]
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    // VarError
    #[error("Invalid variable: {0}")]
    VarError(#[from] std::env::VarError),

    // io Error
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    // request overflow
    #[error("Request payload size is too large")]
    Overflow,

    #[error("Serde Error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Payload error: {0}")]
    PayloadError(#[from] actix_web::error::PayloadError),

    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

impl ResponseError for EpochError {
    fn status_code(&self) -> StatusCode {
        match &self {
            Self::SerdeYaml(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::InitLogger => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::VarError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Overflow => StatusCode::PAYLOAD_TOO_LARGE,
            Self::SerdeError(_) => StatusCode::BAD_REQUEST,
            Self::PayloadError(_) => StatusCode::BAD_REQUEST,
            Self::JoinError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}
