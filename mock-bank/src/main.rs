mod routes;
mod models;
mod state;
mod chaos;
mod cards;
mod docs;

use axum::{
    routing::post,
    Router,
    middleware,
};
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use crate::state::new_state;
use crate::chaos::{chaos_middleware, ChaosConfig};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let chaos_config = ChaosConfig {
        enabled: std::env::var("CHAOS_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true",
        failure_rate: std::env::var("CHAOS_FAILURE_RATE").unwrap_or_else(|_| "0.05".to_string()).parse()?,
        min_latency_ms: std::env::var("CHAOS_MIN_LATENCY_MS").unwrap_or_else(|_| "100".to_string()).parse()?,
        max_latency_ms: std::env::var("CHAOS_MAX_LATENCY_MS").unwrap_or_else(|_| "2000".to_string()).parse()?,
    };

    let state = new_state(chaos_config);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/v1/authorizations", post(routes::authorize))
        .route("/api/v1/captures", post(routes::capture))
        .route("/api/v1/voids", post(routes::void))
        .route("/api/v1/refunds", post(routes::refund))
        .route("/api/v1/chaos", post(routes::update_chaos))
        .layer(middleware::from_fn_with_state(state.clone(), chaos_middleware))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", docs::ApiDoc::openapi()))
        .layer(cors)
        .with_state(state);

    let port: u16 = std::env::var("MOCK_BANK_PORT").unwrap_or_else(|_| "8787".to_string()).parse()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    tracing::info!("Mock Bank listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
