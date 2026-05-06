use crate::bank::types::*;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BankError {
    #[error("bank 5xx: {status}")]   ServerError { status: u16 },
    #[error("timeout")]              Timeout,
    #[error("network: {0}")]         Network(#[from] reqwest::Error),

    #[error("insufficient funds")]   InsufficientFunds,
    #[error("card declined: {reason}")] CardDeclined { reason: String },
    #[error("invalid card")]         InvalidCard,
    #[error("auth expired")]         AuthExpired,
    #[error("state conflict: {message}")] StateConflict { message: String },
}

impl BankError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, BankError::ServerError { .. } | BankError::Timeout | BankError::Network(_))
    }
}

#[async_trait]
pub trait BankClient: Send + Sync {
    async fn authorize(&self, req: AuthorizeRequest, idempotency_key: &str) -> Result<BankAuthResponse, BankError>;
    async fn capture  (&self, req: CaptureRequest, idempotency_key: &str)   -> Result<BankCaptureResponse, BankError>;
    async fn void     (&self, req: VoidRequest, idempotency_key: &str)      -> Result<BankVoidResponse, BankError>;
    async fn refund   (&self, req: RefundRequest, idempotency_key: &str)    -> Result<BankRefundResponse, BankError>;
    async fn get_auth_status(&self, auth_id: &str)   -> Result<AuthStatus, BankError>;
}

pub mod types;
pub mod client;
