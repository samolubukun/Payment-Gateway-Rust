use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("missing Idempotency-Key header")]
    MissingIdempotencyKey,
    #[error("idempotency key reused with different request")]
    IdempotencyKeyConflict,
    #[error("idempotency key request in flight")]
    IdempotencyInFlight,
    #[error("payment not found")]
    NotFound,
    #[error("cannot {operation} a payment in {from} status")]
    InvalidTransition { from: String, operation: String },
    #[error("bank declined: {reason}")]
    BankDeclined { reason: String, retryable: bool },
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::MissingIdempotencyKey    => (StatusCode::BAD_REQUEST, "MISSING_IDEMPOTENCY_KEY"),
            AppError::IdempotencyKeyConflict   => (StatusCode::UNPROCESSABLE_ENTITY, "IDEMPOTENCY_KEY_CONFLICT"),
            AppError::IdempotencyInFlight      => (StatusCode::CONFLICT, "IDEMPOTENCY_IN_FLIGHT"),
            AppError::NotFound                 => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::InvalidTransition { .. } => (StatusCode::CONFLICT, "INVALID_TRANSITION"),
            AppError::BankDeclined { .. }      => (StatusCode::PAYMENT_REQUIRED, "BANK_DECLINED"),
            AppError::Internal(_)              => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };
        
        (status, Json(json!({ "error": { "code": code, "message": self.to_string() } }))).into_response()
    }
}
