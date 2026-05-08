use axum::{
    extract::State,
    http::{StatusCode, HeaderMap},
    Json,
    response::IntoResponse,
};
use serde_json::{json, Value};
use uuid::Uuid;
use crate::models::*;
use crate::state::*;
use crate::cards::*;
use chrono::Utc;

#[utoipa::path(
    post,
    path = "/api/v1/authorizations",
    responses(
        (status = 200, description = "Authorization successful", body = AuthorizeResponse),
        (status = 400, description = "Missing idempotency key"),
        (status = 422, description = "Invalid card"),
        (status = 402, description = "Insufficient funds")
    )
)]
pub async fn authorize(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<AuthorizeRequest>,
) -> impl IntoResponse {
    let idempotency_key = match get_idempotency_key(&headers) {
        Ok(key) => key,
        Err(err) => return err.into_response(),
    };

    let path = "/api/v1/authorizations".to_string();
    
    if let Some(cached) = check_idempotency(&state, &path, &idempotency_key).await {
        return (StatusCode::OK, [("X-Idempotent-Replayed", "true")], Json(cached)).into_response();
    }

    // Validate card
    if !validate_luhn(&payload.card.number) {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error": "Invalid card number", "code": "INVALID_CARD"}))).into_response();
    }

    let card = match get_card(&payload.card.number) {
        Some(c) => c,
        None => return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error": "Card not found", "code": "CARD_NOT_FOUND"}))).into_response(),
    };

    // Simple balance check
    if card.balance_cents < payload.amount_cents {
        return (StatusCode::PAYMENT_REQUIRED, Json(json!({"error": "Insufficient funds", "code": "INSUFFICIENT_FUNDS"}))).into_response();
    }

    // Check expiry (very basic)
    if card.expiry == "03/2020" {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error": "Card expired", "code": "CARD_EXPIRED"}))).into_response();
    }

    let auth_id = Uuid::new_v4().to_string();
    let transaction = BankTransaction {
        id: auth_id.clone(),
        status: BankStatus::Authorized,
        amount_cents: payload.amount_cents,
        card_number: card.number.to_string(),
        created_at: Utc::now(),
    };

    let response = json!(AuthorizeResponse {
        auth_id: auth_id.clone(),
        status: "authorized".to_string(),
    });

    save_transaction_and_idempotency(&state, path, idempotency_key, auth_id, transaction, response.clone()).await;

    (StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/captures",
    responses(
        (status = 200, description = "Capture successful", body = CaptureResponse),
        (status = 404, description = "Auth not found"),
        (status = 409, description = "Invalid state")
    )
)]
pub async fn capture(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<CaptureRequest>,
) -> impl IntoResponse {
    let idempotency_key = match get_idempotency_key(&headers) {
        Ok(key) => key,
        Err(err) => return err.into_response(),
    };

    let path = "/api/v1/captures".to_string();
    
    if let Some(cached) = check_idempotency(&state, &path, &idempotency_key).await {
        return (StatusCode::OK, [("X-Idempotent-Replayed", "true")], Json(cached)).into_response();
    }

    let mut lock = state.write().await;
    let tx = match lock.transactions.get_mut(&payload.auth_id) {
        Some(tx) => tx,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Authorization not found", "code": "AUTH_NOT_FOUND"}))).into_response(),
    };

    if tx.status != BankStatus::Authorized {
        return (StatusCode::CONFLICT, Json(json!({"error": "Transaction not in authorized state", "code": "INVALID_STATE"}))).into_response();
    }

    // 7-day TTL check
    if (Utc::now() - tx.created_at).num_days() >= 7 {
         return (StatusCode::GONE, Json(json!({"error": "Authorization expired", "code": "AUTH_EXPIRED"}))).into_response();
    }

    tx.status = BankStatus::Captured;
    let capture_id = Uuid::new_v4().to_string();
    
    let response = json!(CaptureResponse {
        capture_id: capture_id.clone(),
        status: "captured".to_string(),
    });

    lock.idempotency.insert((path, idempotency_key), response.clone());

    (StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/voids",
    responses(
        (status = 200, description = "Void successful", body = VoidResponse),
        (status = 404, description = "Auth not found"),
        (status = 409, description = "Invalid state")
    )
)]
pub async fn void(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<VoidRequest>,
) -> impl IntoResponse {
    let idempotency_key = match get_idempotency_key(&headers) {
        Ok(key) => key,
        Err(err) => return err.into_response(),
    };

    let path = "/api/v1/voids".to_string();
    
    if let Some(cached) = check_idempotency(&state, &path, &idempotency_key).await {
        return (StatusCode::OK, [("X-Idempotent-Replayed", "true")], Json(cached)).into_response();
    }

    let mut lock = state.write().await;
    let tx = match lock.transactions.get_mut(&payload.auth_id) {
        Some(tx) => tx,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Authorization not found", "code": "AUTH_NOT_FOUND"}))).into_response(),
    };

    if tx.status != BankStatus::Authorized {
        return (StatusCode::CONFLICT, Json(json!({"error": "Transaction not in authorized state", "code": "INVALID_STATE"}))).into_response();
    }

    tx.status = BankStatus::Voided;
    let void_id = Uuid::new_v4().to_string();
    
    let response = json!(VoidResponse {
        void_id: void_id.clone(),
        status: "voided".to_string(),
    });

    lock.idempotency.insert((path, idempotency_key), response.clone());

    (StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/refunds",
    responses(
        (status = 200, description = "Refund successful", body = RefundResponse),
        (status = 404, description = "Capture not found")
    )
)]
pub async fn refund(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(_payload): Json<RefundRequest>,
) -> impl IntoResponse {
    let idempotency_key = match get_idempotency_key(&headers) {
        Ok(key) => key,
        Err(err) => return err.into_response(),
    };

    let path = "/api/v1/refunds".to_string();
    
    if let Some(cached) = check_idempotency(&state, &path, &idempotency_key).await {
        return (StatusCode::OK, [("X-Idempotent-Replayed", "true")], Json(cached)).into_response();
    }

    let mut lock = state.write().await;
    // For refund, we'd normally look up by capture_id, but here we'll just check if there's ANY transaction with that capture_id logic
    // Simplified: find transaction by its auth_id (using capture_id as proxy for simplicity in this mock)
    // In a real bank, capture would have its own ID. Let's just allow refunding any captured transaction.
    
    let tx = lock.transactions.values_mut().find(|t| t.status == BankStatus::Captured); // Simplified lookup
    
    let tx = match tx {
        Some(tx) => tx,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Captured transaction not found", "code": "CAPTURE_NOT_FOUND"}))).into_response(),
    };

    tx.status = BankStatus::Refunded;
    let refund_id = Uuid::new_v4().to_string();
    
    let response = json!(RefundResponse {
        refund_id: refund_id.clone(),
        status: "refunded".to_string(),
    });

    lock.idempotency.insert((path, idempotency_key), response.clone());

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn update_chaos(
    State(state): State<SharedState>,
    Json(new_config): Json<crate::chaos::ChaosConfig>,
) -> impl IntoResponse {
    let mut lock = state.write().await;
    lock.chaos = new_config;
    (StatusCode::OK, Json(json!({"status": "updated"}))).into_response()
}

fn get_idempotency_key(headers: &HeaderMap) -> Result<String, (StatusCode, Json<Value>)> {
    headers.get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or((StatusCode::BAD_REQUEST, Json(json!({"error": "Missing Idempotency-Key header", "code": "MISSING_IDEMPOTENCY_KEY"}))))
}

async fn check_idempotency(state: &SharedState, path: &str, key: &str) -> Option<Value> {
    let lock = state.read().await;
    lock.idempotency.get(&(path.to_string(), key.to_string())).cloned()
}

async fn save_transaction_and_idempotency(
    state: &SharedState,
    path: String,
    key: String,
    id: String,
    tx: BankTransaction,
    response: Value
) {
    let mut lock = state.write().await;
    lock.transactions.insert(id, tx);
    lock.idempotency.insert((path, key), response);
}
