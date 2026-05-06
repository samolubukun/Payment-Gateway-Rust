use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize, PartialEq)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Authorized,
    Captured,
    Voided,
    Refunded,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Authorized => "authorized",
            PaymentStatus::Captured => "captured",
            PaymentStatus::Voided => "voided",
            PaymentStatus::Refunded => "refunded",
        };
        write!(f, "{}", s)
    }
}
