use common::CardDetails;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AuthorizeRequest {
    pub amount_cents: i64,
    pub currency: String,
    pub card: CardDetails,
}

#[derive(Debug, Deserialize)]
pub struct BankAuthResponse {
    pub auth_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CaptureRequest {
    pub auth_id: String,
    pub amount_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct BankCaptureResponse {
    pub capture_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct VoidRequest {
    pub auth_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BankVoidResponse {
    pub void_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct RefundRequest {
    pub capture_id: String,
    pub amount_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct BankRefundResponse {
    pub refund_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthStatus {
    pub auth_id: String,
    pub status: String,
}
