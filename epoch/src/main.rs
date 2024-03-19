mod account;
mod auth;
mod config;
mod decoded_account;
mod errors;
mod handler;
mod logger;
mod utils;

use crate::{config::EpochConfig, errors::EpochResult};
use actix_cors::Cors;
use actix_web::{
    get, post, web,
    web::{Data, Payload},
    App, HttpResponse, HttpServer,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use auth::*;
use borsh::BorshSerialize;
use clap::Parser;
use decoder::Decoder;
use dotenv::dotenv;
use errors::EpochError;
use gcs::bq::BigQueryClient;
use handler::EpochHandler;
use log::*;
use logger::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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
    let handler = tokio::task::spawn_blocking(move || EpochHandler::new(bq_client)).await??;

    let state = Data::new(Arc::new(AppState { handler }));

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://epoch.fm")
            .allowed_methods(vec!["GET", "POST"])
            .allow_any_header()
            .max_age(3600);
        let admin_auth = HttpAuthentication::bearer(admin_validator);

        App::new()
            .app_data(Data::clone(&state))
            .wrap(cors)
            .service(account_id)
            .service(accounts)
            .service(borsh_decoded_accounts)
            .service(json_decoded_accounts)
            .service(filtered_registered_types)
            .service(all_registered_types)
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
    let accts = match state.handler.accounts(payload).await {
        Ok(accts) => accts,
        Err(e) => {
            error!("Error fetching accounts: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(e.to_string()));
        }
    };
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/borsh-decoded-accounts")]
async fn borsh_decoded_accounts(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = match state.handler.borsh_decoded_accounts(payload).await {
        Ok(accts) => accts,
        Err(e) => {
            error!("Error fetching decoded accounts: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(e.to_string()));
        }
    };
    // TODO: remove after debugging
    for acct in accts.iter() {
        #[allow(irrefutable_let_patterns)]
        if let Decoder::Drift(acc) = &acct.decoded {
            match acc {
                decoder::drift_cpi::AccountType::User(user) => {
                    info!(
                        "decoded user pnl: {:?}",
                        user.settled_perp_pnl as f64 / decoder::drift::QUOTE_PRECISION as f64
                    );
                }
                decoder::drift_cpi::AccountType::PerpMarket(market) => {
                    info!("decoded perp market: {:?}", market.pubkey.to_string());
                }
                decoder::drift_cpi::AccountType::SpotMarket(market) => {
                    info!("decoded spot market: {:?}", market.pubkey.to_string());
                }
                _ => {}
            }
        }
    }
    let mut buf = Vec::new();
    accts.serialize(&mut buf)?;
    Ok(HttpResponse::Ok().body(buf))
}

#[post("/decoded-accounts")]
async fn json_decoded_accounts(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.json_decoded_accounts(payload).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/registered-types")]
async fn filtered_registered_types(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = state.handler.registered_types(Some(payload)).await?;
    Ok(HttpResponse::Ok().json(accts))
}

#[get("/registered-types")]
async fn all_registered_types(state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    let accts = state.handler.registered_types(None).await?;
    Ok(HttpResponse::Ok().json(accts))
}

// ================================== ADMIN ================================== //

#[get("/admin-test")]
async fn admin_test(_state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    Ok(HttpResponse::Ok().json("Ok"))
}
