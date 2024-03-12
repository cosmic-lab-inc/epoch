mod account;
mod auth;
mod errors;
// mod handler;
mod postgres_handler;
mod logger;
mod utils;

use auth::*;
use clap::Parser;
use errors::EpochError;
// use handler::*;
use log::*;
use logger::*;
use postgres_handler::EpochHandler;

use crate::errors::EpochResult;
use actix_cors::Cors;
use actix_web::web::{Data, Payload};
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use dotenv::dotenv;
use postgres_client::PostgresClient;
use std::path::PathBuf;
use std::sync::Arc;

struct AppState {
    handler: EpochHandler,
}

#[derive(Parser, Debug)]
struct Args {
    /// Path to backfill.yaml config.
    /// Should deserialize into BackfillConfig
    #[arg(long, env, default_value = "backfill.yaml")]
    config_file_path: PathBuf,
}

#[actix_web::main]
async fn main() -> EpochResult<()> {
    dotenv().ok();
    init_logger()?;
    info!("Starting Epoch server ⏳ ");

    let port = std::env::var("PORT").unwrap_or_else(|_| "3333".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    let db_url = std::env::var("DATABASE_URL")?;
    let handler = EpochHandler::new(PostgresClient::new_from_url(db_url).await?);
    let state = Data::new(Arc::new(AppState { handler }));

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        let admin_auth = HttpAuthentication::bearer(admin_validator);

        App::new()
            .app_data(Data::clone(&state))
            .wrap(cors)
            .service(
                web::scope("/api")
                    .service(accounts)
                    .service(accounts_key)
                    .service(accounts_owner)
                    .service(accounts_slot)
                    .service(accounts_key_owner)
                    .service(accounts_key_slot)
                    .service(accounts_owner_slot)
                    .service(accounts_key_owner_slot),
            )
            .service(web::scope("/admin").wrap(admin_auth).service(admin_test))
            .service(test)
    })
    .bind(bind_address)?
    .run()
    .await
    .map_err(EpochError::from)
}

#[get("/")]
async fn test() -> EpochResult<HttpResponse> {
    Ok(HttpResponse::Ok().body(
        r#"
 _____                      _
|  ___|                    | |
| |__   _ __    ___    ___ | |__
|  __| | '_ \  / _ \  / __|| '_ \
| |___ | |_) || (_) || (__ | | | |
\____/ | .__/  \___/  \___||_| |_|
       | |
       |_|

Everything back to genesis.
Every account for every program.
Every answer to any inquiry.
Solana data is a gold mine, and this is your pickaxe.
"#,
    ))
}

// ================================== API ================================== //

#[post("/accounts")]
async fn accounts(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-key")]
async fn accounts_key(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_key(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-owner")]
async fn accounts_owner(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_owner(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-slot")]
async fn accounts_slot(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_slot(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-key-owner")]
async fn accounts_key_owner(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_key_owner(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-key-slot")]
async fn accounts_key_slot(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_key_slot(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-owner-slot")]
async fn accounts_owner_slot(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_owner_slot(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts-key-owner-slot")]
async fn accounts_key_owner_slot(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts_key_owner_slot(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

// ================================== ADMIN ================================== //

#[get("/admin_test")]
async fn admin_test(state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    Ok(HttpResponse::Ok().json("Ok"))
}
