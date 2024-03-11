mod auth;
mod errors;
mod logger;
mod utils;

use auth::*;
use clap::Parser;
use errors::EpochError;
use log::*;
use logger::*;

use crate::errors::EpochResult;
use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use archive_stream::ArchiveAccount;
use dotenv::dotenv;
use postgres_client::DbAccount;
use postgres_client::{FromDbAccount, PostgresClient};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_SIZE: usize = 262_144; // max payload size is 256k

struct AppState {
    client: PostgresClient,
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
    let client = PostgresClient::new_from_url(db_url).await?;
    let state = Data::new(Arc::new(AppState { client }));

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
            .service(web::scope("/api").service(accounts))
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

#[get("/accounts")]
async fn accounts(state: web::Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    let accounts: Vec<ArchiveAccount> = state
        .client
        .accounts()
        .await?
        .into_iter()
        .filter_map(|a| match DbAccount::try_from(&a) {
            Err(e) => {
                error!("Error converting DbAccount: {}", e);
                None
            }
            Ok(db) => ArchiveAccount::from_db_account(db).ok(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(accounts))
}

#[post("/test_post")]
async fn test_post(
    state: web::Data<Arc<AppState>>,
    payload: web::Payload,
) -> EpochResult<HttpResponse> {
    info!("test post");
    Ok(HttpResponse::Ok().json("Ok"))
}

// ================================== ADMIN ================================== //

#[get("/admin_test")]
async fn admin_test(state: web::Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    Ok(HttpResponse::Ok().json("Ok"))
}
