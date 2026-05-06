use crate::domain::payment::PaymentStatus;
use thiserror::Error;

pub enum PaymentEvent {
    AuthorizationSucceeded { bank_auth_id: String },
    AuthorizationFailed    { reason: String },
    CaptureSucceeded       { bank_capture_id: String },
    VoidSucceeded          { bank_void_id: String },
    RefundSucceeded        { bank_refund_id: String },
}

impl PaymentEvent {
    pub fn name(&self) -> &'static str {
        match self {
            PaymentEvent::AuthorizationSucceeded { .. } => "AuthorizationSucceeded",
            PaymentEvent::AuthorizationFailed { .. } => "AuthorizationFailed",
            PaymentEvent::CaptureSucceeded { .. } => "CaptureSucceeded",
            PaymentEvent::VoidSucceeded { .. } => "VoidSucceeded",
            PaymentEvent::RefundSucceeded { .. } => "RefundSucceeded",
        }
    }
}

#[derive(Debug, Error)]
#[error("cannot transition from {from} via {event}")]
pub struct InvalidTransitionError {
    pub from: PaymentStatus,
    pub event: &'static str,
}

impl PaymentStatus {
    pub fn apply(&self, event: &PaymentEvent) -> Result<PaymentStatus, InvalidTransitionError> {
        use PaymentStatus::*;
        use PaymentEvent::*;
        match (self, event) {
            (Pending,    AuthorizationSucceeded { .. }) => Ok(Authorized),
            (Authorized, CaptureSucceeded       { .. }) => Ok(Captured),
            (Authorized, VoidSucceeded          { .. }) => Ok(Voided),
            (Captured,   RefundSucceeded        { .. }) => Ok(Refunded),
            _ => Err(InvalidTransitionError {
                from: self.clone(),
                event: event.name(),
            }),
        }
    }
}
