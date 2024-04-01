use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_cors::Cors;
use actix_web::{
    get, post, web,
    web::{Data, Payload},
    App, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use borsh::BorshSerialize;
use clap::Parser;
use dotenv::dotenv;
use log::*;

use auth::*;
use common::init_logger;
use decoder::Decoder;
use errors::EpochError;
use gcs::bq::BigQueryClient;
use handler::*;

use crate::{config::EpochConfig, errors::EpochResult};

mod auth;
mod bootstrap;
mod config;
mod errors;
mod handler;
mod utils;

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
    init_logger();
    let args: Args = Args::parse();
    info!("Starting Epoch server ⏳ ");

    let port = std::env::var("PORT").unwrap_or_else(|_| "3333".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    let epoch_config = match EpochConfig::read_config(&args.config_file_path) {
        Ok(config) => config,
        Err(e) => {
            error!("Error reading config: {:?}", e);
            return Err(EpochError::from(e));
        }
    };

    let rpc_url = std::env::var("RPC_URL")
        .map_err(|_e| EpochError::EnvVarError("RPC_URL missing".to_string()))?;
    let is_mainnet = std::env::var("IS_MAINNET")
        .map_err(|_e| EpochError::EnvVarError("IS_MAINNET missing".to_string()))?
        .parse::<bool>()
        .map_err(|_e| EpochError::EnvVarError("IS_MAINNET must be a boolean".to_string()))?;

    // bootstrap network
    bootstrap::bootstrap_epoch(rpc_url.clone()).await?;

    // init Google BigQuery client to read historical accounts
    let bq_client = match BigQueryClient::new(Path::new(&epoch_config.gcs_sa_key)).await {
        Ok(client) => client,
        Err(e) => {
            error!("Error creating BigQuery client: {:?}", e);
            return Err(EpochError::from(e));
        }
    };
    let handler = tokio::task::spawn_blocking(move || {
        EpochHandler::new(bq_client, &epoch_config.redis_url(), rpc_url, is_mainnet)
    })
    .await??;

    let state = Data::new(Arc::new(AppState { handler }));

    HttpServer::new(move || {
        let cors = Cors::default()
            // .allowed_origin("http://localhost:3000")
            // .allowed_origin("http://epoch.fm")
            .allow_any_origin()
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
            .service(test)
            .service(create_user)
            .service(delete_user)
            .service(user_balance)
            .service(read_user)
            .service(airdrop)
            .service(request_challenge)
            .service(authenticate)
            .service(highest_slot)
            .service(lowest_slot)
            .service(web::scope("/admin").wrap(admin_auth).service(admin_test))
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

// ================================== ACCOUNTS ================================== //

#[post("/account-id")]
async fn account_id(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = match state.handler.account_id(payload).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(accts))
}

#[post("/accounts")]
async fn accounts(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let accts = match state.handler.accounts(payload).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
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
    let mut buf = Vec::new();
    accts.serialize(&mut buf)?;
    Ok(HttpResponse::Ok().body(buf))
}

#[post("/decoded-accounts")]
async fn json_decoded_accounts(
    state: Data<Arc<AppState>>,
    payload: Payload,
    req: HttpRequest,
) -> EpochResult<HttpResponse> {
    let epoch_api_key = EpochHandler::parse_api_key_header(req)?;
    let accts = match state
        .handler
        .json_decoded_accounts(payload, epoch_api_key, 1_f64)
        .await
    {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(accts))
}

// ================================== TYPES ================================== //

#[post("/registered-types")]
async fn filtered_registered_types(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let accts = match state.handler.registered_types(Some(payload)).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(accts))
}

#[get("/registered-types")]
async fn all_registered_types(state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    let accts = match state.handler.registered_types(None).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(accts))
}

// ================================== USER ================================== //

#[post("/challenge")]
async fn request_challenge(
    state: Data<Arc<AppState>>,
    payload: Payload,
) -> EpochResult<HttpResponse> {
    let res = match state.handler.request_challenge(payload).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(res))
}

#[post("/authenticate")]
async fn authenticate(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    let res = match state.handler.authenticate_signature(payload).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(res))
}

#[get("/user-balance")]
async fn user_balance(state: Data<Arc<AppState>>, req: HttpRequest) -> EpochResult<HttpResponse> {
    let epoch_api_key = EpochHandler::parse_api_key_header(req)?;
    let res = match state.handler.user_balance(epoch_api_key).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(res))
}

#[get("/read-user")]
async fn read_user(state: Data<Arc<AppState>>, req: HttpRequest) -> EpochResult<HttpResponse> {
    let epoch_api_key = EpochHandler::parse_api_key_header(req)?;
    let res = match state.handler.read_user(epoch_api_key).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    // map to Option<String>
    let res = res.map(|user| user.to_string());

    Ok(HttpResponse::Ok().json(res))
}

#[post("/create-user")]
async fn create_user(
    state: Data<Arc<AppState>>,
    payload: Payload,
    req: HttpRequest,
) -> EpochResult<HttpResponse> {
    let epoch_api_key = EpochHandler::parse_api_key_header(req)?;
    let res = match state.handler.create_user(payload, epoch_api_key).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(res.to_string()))
}

#[post("/update-user")]
async fn update_user(
    state: Data<Arc<AppState>>,
    payload: Payload,
    req: HttpRequest,
) -> EpochResult<HttpResponse> {
    let epoch_api_key = EpochHandler::parse_api_key_header(req)?;
    let res = match state.handler.update_user(payload, epoch_api_key).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(res.to_string()))
}

#[post("/delete-user")]
async fn delete_user(
    state: Data<Arc<AppState>>,
    payload: Payload,
    req: HttpRequest,
) -> EpochResult<HttpResponse> {
    let epoch_api_key = EpochHandler::parse_api_key_header(req)?;
    match state.handler.delete_user(payload, epoch_api_key).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json("User deleted"))
}

#[post("/airdrop")]
async fn airdrop(state: Data<Arc<AppState>>, payload: Payload) -> EpochResult<HttpResponse> {
    match state.handler.airdrop(payload).await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json("Airdrop successful"))
}

#[get("/highest-slot")]
async fn highest_slot(state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    let slot = match state.handler.highest_slot().await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(slot))
}

#[get("/lowest-slot")]
async fn lowest_slot(state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    let slot = match state.handler.lowest_slot().await {
        Ok(res) => Ok(res),
        Err(e) => {
            error!("{:?}", e);
            Err(e)
        }
    }?;
    Ok(HttpResponse::Ok().json(slot))
}

// ================================== ADMIN ================================== //

#[get("/test")]
async fn admin_test(_state: Data<Arc<AppState>>) -> EpochResult<HttpResponse> {
    Ok(HttpResponse::Ok().json("Ok"))
}
