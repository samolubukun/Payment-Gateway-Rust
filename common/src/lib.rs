use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum Currency {
    USD,
    EUR,
    GBP,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Money {
    pub amount_cents: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardDetails {
    pub number: String,
    pub exp_month: u8,
    pub exp_year: u16,
    pub cvv: String,
}

pub mod config;
