use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct ChaosConfig {
    pub enabled: bool,
    pub failure_rate: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

pub async fn chaos_middleware(
    State(state): State<crate::state::SharedState>,
    req: Request,
    next: Next,
) -> Response {
    if req.uri().path() == "/api/v1/chaos" || req.uri().path().starts_with("/docs") || req.uri().path().starts_with("/api-docs") {
        return next.run(req).await;
    }

    let config = {
        let s = state.read().await;
        s.chaos.clone()
    };

    if !config.enabled {
        return next.run(req).await;
    }

    // Latency injection
    let delay = {
        let mut rng = rand::thread_rng();
        rng.gen_range(config.min_latency_ms..=config.max_latency_ms)
    };
    sleep(Duration::from_millis(delay)).await;

    // Failure injection
    let should_fail = {
        let mut rng = rand::thread_rng();
        rng.gen_bool(config.failure_rate)
    };
    
    if should_fail {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Random failure injection").into_response();
    }

    next.run(req).await
}
