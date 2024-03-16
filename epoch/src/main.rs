mod account;
mod auth;
mod config;
mod decoded_account;
mod errors;
mod handler;
mod logger;
mod utils;

use auth::*;
use clap::Parser;
use errors::EpochError;
use handler::EpochHandler;
use log::*;
use logger::*;

use crate::config::EpochConfig;
use crate::errors::EpochResult;
use actix_cors::Cors;
use actix_web::web::{Data, Payload};
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use borsh::BorshSerialize;
use dotenv::dotenv;
use gcs::bq::BigQueryClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct AppState {
    handler: EpochHandler,
}

#[derive(Parser, Debug)]
struct Args {
    /// Path to backfill.yaml config.
    /// Should deserialize into EpochConfig
    #[arg(long, env, default_value = "epoch.yaml")]
    config_file_path: PathBuf,
}

#[actix_web::main]
async fn main() -> EpochResult<()> {
    dotenv().ok();
    init_logger()?;
    let args: Args = Args::parse();
    info!("Starting Epoch server ⏳ ");

    let port = std::env::var("PORT").unwrap_or_else(|_| "3333".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    let epoch_config = EpochConfig::read_config(&args.config_file_path)?;
    let bq_client = BigQueryClient::new(Path::new(&epoch_config.gcs_sa_key)).await?;
    let rpc = RpcClient::new(epoch_config.solana_rpc.clone());
    let handler = tokio::task::spawn_blocking(move || EpochHandler::new(bq_client, rpc)).await??;

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
                    .service(account_id)
                    .service(accounts)
                    .service(decoded_accounts),
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

#[post("/account-id")]
async fn account_id(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = state.handler.account_id(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts")]
async fn accounts(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = state.handler.accounts(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/decoded-accounts")]
async fn decoded_accounts(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.decoded_accounts(payload).await?;
    let mut buf = Vec::new();
    accts.serialize(&mut buf)?;
    Ok(HttpResponse::Ok().body(buf))
}

// ================================== ADMIN ================================== //

#[get("/admin-test")]
async fn admin_test(_state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    Ok(HttpResponse::Ok().json("Ok"))
}
