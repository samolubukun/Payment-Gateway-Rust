mod domain;
mod db;
mod bank;
mod idempotency;
mod routes;
mod workers;
mod errors;

use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use crate::bank::client::ReqwestBankClient;
use common::config::DbConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_config = envy::from_env::<DbConfig>()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_config.url())
        .await?;

    // Run migrations
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await?;

    let bank_base_url = std::env::var("BANK_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());
    let bank_client = Arc::new(ReqwestBankClient::new(bank_base_url)) as Arc<dyn bank::BankClient>;

    // Spawn recovery worker
    let worker_pool = pool.clone();
    let worker_bank = bank_client.clone();
    tokio::spawn(async move {
        workers::recovery::run(worker_pool, worker_bank).await;
    });

    #[derive(Clone)]
    struct AppState {
        pool: sqlx::PgPool,
        bank: Arc<dyn bank::BankClient>,
    }

    impl axum::extract::FromRef<AppState> for sqlx::PgPool {
        fn from_ref(state: &AppState) -> Self {
            state.pool.clone()
        }
    }

    impl axum::extract::FromRef<AppState> for Arc<dyn bank::BankClient> {
        fn from_ref(state: &AppState) -> Self {
            state.bank.clone()
        }
    }

    let state = AppState {
        pool: pool.clone(),
        bank: bank_client.clone(),
    };

    let app = Router::new()
        .route("/v1/payments", post(routes::payments::authorize))
        .route("/v1/payments/:id/capture", post(routes::payments::capture))
        .route("/v1/payments/:id", get(routes::payments::get_payment))
        .route("/health", get(routes::health::health))
        .with_state(state);

    let port: u16 = std::env::var("GATEWAY_PORT").unwrap_or_else(|_| "3000".to_string()).parse()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("Gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
