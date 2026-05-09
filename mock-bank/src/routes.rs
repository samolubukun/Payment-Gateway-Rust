use axum::{
    extract::State,
    http::{StatusCode, HeaderMap},
    Json,
    response::IntoResponse,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
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
    let request_hash = hash_request(&payload);

    if let Some(cached) = match check_idempotency(&state, &path, &idempotency_key, &request_hash).await {
        Ok(result) => result,
        Err(err) => return err.into_response(),
    } {
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
        capture_id: None,
        refund_id: None,
    };

    let response = json!(AuthorizeResponse {
        auth_id: auth_id.clone(),
        status: "authorized".to_string(),
    });

    save_transaction_and_idempotency(&state, path, idempotency_key, request_hash, auth_id, transaction, response.clone()).await;

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
    let request_hash = hash_request(&payload);

    if let Some(cached) = match check_idempotency(&state, &path, &idempotency_key, &request_hash).await {
        Ok(result) => result,
        Err(err) => return err.into_response(),
    } {
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

    if payload.amount_cents > tx.amount_cents {
        return (StatusCode::CONFLICT, Json(json!({"error": "Capture amount exceeds authorized amount", "code": "AMOUNT_EXCEEDS_AUTH"}))).into_response();
    }

    // 7-day TTL check
    if (Utc::now() - tx.created_at).num_days() >= 7 {
         return (StatusCode::GONE, Json(json!({"error": "Authorization expired", "code": "AUTH_EXPIRED"}))).into_response();
    }

    tx.status = BankStatus::Captured;
    let capture_id = Uuid::new_v4().to_string();
    tx.capture_id = Some(capture_id.clone());
    
    let response = json!(CaptureResponse {
        capture_id: capture_id.clone(),
        status: "captured".to_string(),
    });

    lock.idempotency.insert((path, idempotency_key), IdempotencyRecord { request_hash, response: response.clone() });

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
    let request_hash = hash_request(&payload);

    if let Some(cached) = match check_idempotency(&state, &path, &idempotency_key, &request_hash).await {
        Ok(result) => result,
        Err(err) => return err.into_response(),
    } {
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

    lock.idempotency.insert((path, idempotency_key), IdempotencyRecord { request_hash, response: response.clone() });

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
    Json(payload): Json<RefundRequest>,
) -> impl IntoResponse {
    let idempotency_key = match get_idempotency_key(&headers) {
        Ok(key) => key,
        Err(err) => return err.into_response(),
    };

    let path = "/api/v1/refunds".to_string();
    let request_hash = hash_request(&payload);

    if let Some(cached) = match check_idempotency(&state, &path, &idempotency_key, &request_hash).await {
        Ok(result) => result,
        Err(err) => return err.into_response(),
    } {
        return (StatusCode::OK, [("X-Idempotent-Replayed", "true")], Json(cached)).into_response();
    }

    let mut lock = state.write().await;
    let tx = lock.transactions.values_mut().find(|t| t.capture_id.as_deref() == Some(&payload.capture_id));

    let tx = match tx {
        Some(tx) => tx,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "Captured transaction not found", "code": "CAPTURE_NOT_FOUND"}))).into_response(),
    };

    if tx.status != BankStatus::Captured {
        return (StatusCode::CONFLICT, Json(json!({"error": "Transaction not in captured state", "code": "INVALID_STATE"}))).into_response();
    }

    if payload.amount_cents > tx.amount_cents {
        return (StatusCode::CONFLICT, Json(json!({"error": "Refund amount exceeds captured amount", "code": "AMOUNT_EXCEEDS_CAPTURE"}))).into_response();
    }

    tx.status = BankStatus::Refunded;
    let refund_id = Uuid::new_v4().to_string();
    tx.refund_id = Some(refund_id.clone());
    
    let response = json!(RefundResponse {
        refund_id: refund_id.clone(),
        status: "refunded".to_string(),
    });

    lock.idempotency.insert((path, idempotency_key), IdempotencyRecord { request_hash, response: response.clone() });

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

async fn check_idempotency(state: &SharedState, path: &str, key: &str, request_hash: &str) -> Result<Option<Value>, (StatusCode, Json<Value>)> {
    let lock = state.read().await;
    if let Some(record) = lock.idempotency.get(&(path.to_string(), key.to_string())) {
        if record.request_hash == request_hash {
            return Ok(Some(record.response.clone()));
        }
        return Err((StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"error": "Idempotency key reused with different request", "code": "IDEMPOTENCY_KEY_CONFLICT"}))));
    }
    Ok(None)
}

async fn save_transaction_and_idempotency(
    state: &SharedState,
    path: String,
    key: String,
    request_hash: String,
    id: String,
    tx: BankTransaction,
    response: Value
) {
    let mut lock = state.write().await;
    lock.transactions.insert(id, tx);
    lock.idempotency.insert((path, key), IdempotencyRecord { request_hash, response });
}

fn hash_request<T: serde::Serialize>(payload: &T) -> String {
    let body = serde_json::to_vec(payload).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&body);
    format!("{:x}", hasher.finalize())
}
