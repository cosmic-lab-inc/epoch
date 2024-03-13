use actix_web::{dev::ServiceRequest, Error as ActixError};
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use log::*;

// Auth0 Rust example
// https://auth0.com/blog/build-an-api-in-rust-with-jwt-authentication-using-actix-web/#Getting-Started

/// Credentials is a key that should match environment variable ADMIN_BEARER_TOKEN
pub async fn admin_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (ActixError, ServiceRequest)> {
    let admin_bearer_token =
        std::env::var("ADMIN_BEARER_TOKEN").expect("ADMIN_BEARER_TOKEN must be set");
    if credentials.token() != admin_bearer_token {
        info!("Token validation failed");
        let config = req.app_data::<Config>().cloned().unwrap_or_default();
        Err((AuthenticationError::from(config).into(), req))
    } else {
        Ok(req)
    }
}
