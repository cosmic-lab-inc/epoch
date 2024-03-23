use redis::RedisError;
use thiserror::Error;

pub type WardenResult<T> = Result<T, WardenError>;

#[derive(Debug, Error)]
pub enum WardenError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Argon2 error: {0}")]
    ArgonError(#[from] argon2::password_hash::Error),

    #[error("Redis error: {0}")]
    RedisError(#[from] RedisError),

    #[error("Failed to connect to Redis client")]
    RedisConnectionError,

    // api key does not match hash
    #[error("API key does not match hash")]
    ApiKeyMismatch,
}
