use actix_web::http::StatusCode;
use actix_web::{error::ResponseError, HttpResponse};
use thiserror::Error;

pub type EpochResult<T> = Result<T, EpochError>;

#[derive(Debug, Error)]
pub enum EpochError {
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
}

impl ResponseError for EpochError {
    fn status_code(&self) -> StatusCode {
        match &self {
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::InitLogger => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::VarError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}
