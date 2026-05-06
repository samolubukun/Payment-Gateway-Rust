use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use crate::errors::AppError;

pub struct IdempotencyKey(pub String);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for IdempotencyKey {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, AppError> {
        parts.headers
            .get("Idempotency-Key")
            .and_then(|v| v.to_str().ok())
            .map(|s| IdempotencyKey(s.to_string()))
            .ok_or(AppError::MissingIdempotencyKey)
    }
}

pub struct MerchantId(pub String);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for MerchantId {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, AppError> {
        parts.headers
            .get("X-Merchant-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| MerchantId(s.to_string()))
            .ok_or(AppError::Internal(anyhow::anyhow!("Missing X-Merchant-Id header")))
    }
}
