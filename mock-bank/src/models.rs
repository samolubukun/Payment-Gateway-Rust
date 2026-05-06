use common::CardDetails;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorizeRequest {
    pub amount_cents: i64,
    pub currency: String,
    pub card: CardDetails,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthorizeResponse {
    pub auth_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CaptureRequest {
    pub auth_id: String,
    pub amount_cents: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CaptureResponse {
    pub capture_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VoidRequest {
    pub auth_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VoidResponse {
    pub void_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefundRequest {
    pub capture_id: String,
    pub amount_cents: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefundResponse {
    pub refund_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BankErrorResponse {
    pub error: String,
    pub code: String,
}
