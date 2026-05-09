use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use chrono::{DateTime, Utc};
use crate::chaos::ChaosConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum BankStatus {
    Authorized,
    Captured,
    Voided,
    Refunded,
}

#[derive(Debug, Clone)]
pub struct BankTransaction {
    pub id: String,
    pub status: BankStatus,
    pub amount_cents: i64,
    pub card_number: String,
    pub created_at: DateTime<Utc>,
    pub capture_id: Option<String>,
    pub refund_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IdempotencyRecord {
    pub request_hash: String,
    pub response: Value,
}

#[derive(Debug, Default)]
pub struct BankState {
    // Map of ID to Transaction
    pub transactions: HashMap<String, BankTransaction>,
    // Idempotency: (path, key) -> Response
    pub idempotency: HashMap<(String, String), IdempotencyRecord>,
    // Dynamic Chaos Config
    pub chaos: ChaosConfig,
}

pub type SharedState = Arc<RwLock<BankState>>;

pub fn new_state(chaos: ChaosConfig) -> SharedState {
    Arc::new(RwLock::new(BankState {
        transactions: HashMap::new(),
        idempotency: HashMap::new(),
        chaos,
    }))
}
