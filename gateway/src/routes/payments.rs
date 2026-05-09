use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use crate::db::repository::{PaymentRepository, IdempotencyStatus, idempotency};
use crate::idempotency::{IdempotencyKey, MerchantId};
use crate::bank::{BankClient, types::*};
use crate::domain::payment::PaymentStatus;
use crate::errors::AppError;
use std::sync::Arc;
use sqlx::PgPool;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePaymentRequest {
    pub order_id: String,
    pub customer_id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub card: common::CardDetails,
}

pub async fn authorize(
    State(pool): State<PgPool>,
    State(bank): State<Arc<dyn BankClient>>,
    IdempotencyKey(key): IdempotencyKey,
    MerchantId(merchant_id): MerchantId,
    Json(payload): Json<CreatePaymentRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Received authorize request for order: {} from merchant: {}", payload.order_id, merchant_id);
    // 1. Idempotency check
    let hash = {
        let body = serde_json::to_vec(&payload).map_err(anyhow::Error::from)?;
        let mut hasher = Sha256::new();
        hasher.update(&body);
        format!("{:x}", hasher.finalize())
    };
    if let Some(req) = idempotency::get_request(&pool, &key, &merchant_id).await.map_err(anyhow::Error::from)? {
        if req.request_hash != hash {
            return Err(AppError::IdempotencyKeyConflict);
        }

        if req.status == IdempotencyStatus::Complete {
            let body = req.response_body.unwrap_or(json!({}));
            return Ok((StatusCode::OK, [("X-Idempotent-Replayed", "true")], Json(body)).into_response());
        } else {
            return Err(AppError::IdempotencyInFlight);
        }
    }
    idempotency::create_request(&pool, &key, &merchant_id, &hash).await.map_err(anyhow::Error::from)?;

    // 2. Create pending payment
    let payment_id = PaymentRepository::create_payment(
        &pool, &key, &payload.order_id, &payload.customer_id, payload.amount_cents, &payload.currency
    ).await.map_err(anyhow::Error::from)?;

    // 3. Call bank
    let bank_req = AuthorizeRequest {
        amount_cents: payload.amount_cents,
        currency: payload.currency,
        card: payload.card,
    };

    let bank_res = bank.authorize(bank_req, &key).await;

    // 4. Update state
    let mut tx = pool.begin().await.map_err(anyhow::Error::from)?;
    match bank_res {
        Ok(res) => {
            PaymentRepository::update_status(
                &mut tx, payment_id, PaymentStatus::Pending, PaymentStatus::Authorized, Some(res.auth_id), "client"
            ).await.map_err(anyhow::Error::from)?;
            
            let response = json!({ "id": payment_id, "status": "authorized" });
            idempotency::complete_request_tx(&mut tx, &key, 200, response.clone()).await.map_err(anyhow::Error::from)?;
            tx.commit().await.map_err(anyhow::Error::from)?;
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) => {
            // Handle failure - for now just return error
            // In a real app, we might store the failure reason
            tx.rollback().await.map_err(anyhow::Error::from)?;
            Err(AppError::BankDeclined { reason: e.to_string(), retryable: e.is_retryable() })
        }
    }
}

pub async fn capture(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
    State(bank): State<Arc<dyn BankClient>>,
    IdempotencyKey(key): IdempotencyKey,
) -> Result<impl IntoResponse, AppError> {
    let payment = PaymentRepository::get_payment(&pool, id).await.map_err(anyhow::Error::from)?
        .ok_or(AppError::NotFound)?;

    if payment.status != PaymentStatus::Authorized {
        return Err(AppError::InvalidTransition { from: payment.status.to_string(), operation: "capture".to_string() });
    }

    let bank_req = CaptureRequest {
        auth_id: payment.bank_auth_id.unwrap(),
        amount_cents: payment.amount_cents,
    };

    let bank_res = bank.capture(bank_req, &key).await;

    let mut tx = pool.begin().await.map_err(anyhow::Error::from)?;
    match bank_res {
        Ok(res) => {
            PaymentRepository::update_status(
                &mut tx, id, PaymentStatus::Authorized, PaymentStatus::Captured, Some(res.capture_id), "client"
            ).await.map_err(anyhow::Error::from)?;
            tx.commit().await.map_err(anyhow::Error::from)?;
            Ok((StatusCode::OK, Json(json!({ "id": id, "status": "captured" }))).into_response())
        }
        Err(e) => {
             tx.rollback().await.map_err(anyhow::Error::from)?;
             Err(AppError::BankDeclined { reason: e.to_string(), retryable: e.is_retryable() })
        }
    }
}

pub async fn get_payment(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, AppError> {
    let payment = PaymentRepository::get_payment(&pool, id).await.map_err(anyhow::Error::from)?
        .ok_or(AppError::NotFound)?;
    
    Ok(Json(json!({
        "id": payment.id,
        "status": payment.status,
        "amount_cents": payment.amount_cents,
        "currency": payment.currency,
        "order_id": payment.order_id,
    })))
}
