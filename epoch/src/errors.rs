use actix_web::http::StatusCode;
use actix_web::{error::ResponseError, HttpResponse};
use thiserror::Error;

pub type EpochResult<T> = Result<T, EpochError>;

#[derive(Debug, Error)]
pub enum EpochError {
    #[error("SerdeYaml: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error("Init logger error")]
    InitLogger,

    #[error("Internal Server Error")]
    InternalServerError(String),

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

    // serde
    #[error("Serde Error: {0}")]
    SerdeError(#[from] serde_json::Error),

    // PayloadError
    #[error("Payload error: {0}")]
    PayloadError(#[from] actix_web::error::PayloadError),
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
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}
