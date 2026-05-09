use sqlx::{PgPool, Postgres, Transaction};
use crate::domain::payment::PaymentStatus;
use uuid::Uuid;
use chrono::Utc;

pub struct PaymentRepository;

impl PaymentRepository {
    pub async fn create_payment(
        pool: &PgPool,
        idempotency_key: &str,
        order_id: &str,
        customer_id: &str,
        amount_cents: i64,
        currency: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO payments (id, idempotency_key, order_id, customer_id, amount_cents, currency, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(id)
        .bind(idempotency_key)
        .bind(order_id)
        .bind(customer_id)
        .bind(amount_cents)
        .bind(currency)
        .bind(PaymentStatus::Pending as PaymentStatus)
        .execute(pool)
        .await?;
        Ok(id)
    }

    pub async fn get_payment(pool: &PgPool, id: Uuid) -> Result<Option<PaymentRow>, sqlx::Error> {
        sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT id, idempotency_key, order_id, customer_id, amount_cents, currency, status, 
            bank_auth_id, bank_capture_id, bank_void_id, bank_refund_id, created_at, updated_at
            FROM payments WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_status(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        old_status: PaymentStatus,
        new_status: PaymentStatus,
        bank_id: Option<String>,
        actor: &str,
    ) -> Result<(), sqlx::Error> {
        // Atomic update with status check
        let rows = sqlx::query(
            r#"
            UPDATE payments SET status = $1, updated_at = NOW(),
            bank_auth_id = COALESCE(bank_auth_id, CASE WHEN $1 = 'authorized' THEN $4 ELSE bank_auth_id END),
            bank_capture_id = COALESCE(bank_capture_id, CASE WHEN $1 = 'captured' THEN $4 ELSE bank_capture_id END),
            bank_void_id = COALESCE(bank_void_id, CASE WHEN $1 = 'voided' THEN $4 ELSE bank_void_id END),
            bank_refund_id = COALESCE(bank_refund_id, CASE WHEN $1 = 'refunded' THEN $4 ELSE bank_refund_id END)
            WHERE id = $2 AND status = $3
            "#
        )
        .bind(new_status as PaymentStatus)
        .bind(id)
        .bind(old_status as PaymentStatus)
        .bind(bank_id)
        .execute(&mut **tx)
        .await?;

        if rows.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        // Insert event
        sqlx::query(
            r#"
            INSERT INTO payment_events (payment_id, from_status, to_status, actor)
            VALUES ($1, $2, $3, $4)
            "#
        )
        .bind(id)
        .bind(old_status as PaymentStatus)
        .bind(new_status as PaymentStatus)
        .bind(actor)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct PaymentRow {
    pub id: Uuid,
    pub idempotency_key: String,
    pub order_id: String,
    pub customer_id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub bank_auth_id: Option<String>,
    pub bank_capture_id: Option<String>,
    pub bank_void_id: Option<String>,
    pub bank_refund_id: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub mod idempotency {
    use super::*;
    use serde_json::Value;

    pub async fn get_request(pool: &PgPool, key: &str, merchant_id: &str) -> Result<Option<IdempotencyRequest>, sqlx::Error> {
        sqlx::query_as::<_, IdempotencyRequest>(
            r#"
            SELECT key, merchant_id, request_hash, response_status, response_body, status
            FROM idempotency_requests WHERE key = $1 AND merchant_id = $2
            "#
        )
        .bind(key)
        .bind(merchant_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn create_request(pool: &PgPool, key: &str, merchant_id: &str, hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO idempotency_requests (key, merchant_id, request_hash, status)
            VALUES ($1, $2, $3, 'in_flight')
            "#
        )
        .bind(key)
        .bind(merchant_id)
        .bind(hash)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn complete_request_tx(
        tx: &mut Transaction<'_, Postgres>,
        key: &str,
        status: i32,
        body: Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE idempotency_requests SET status = 'complete', response_status = $1, response_body = $2
            WHERE key = $3
            "#
        )
        .bind(status)
        .bind(body)
        .bind(key)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, sqlx::Type)]
#[sqlx(type_name = "idempotency_status", rename_all = "snake_case")]
pub enum IdempotencyStatus {
    InFlight,
    Complete,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct IdempotencyRequest {
    pub key: String,
    pub merchant_id: String,
    pub request_hash: String,
    pub response_status: Option<i32>,
    pub response_body: Option<serde_json::Value>,
    pub status: IdempotencyStatus,
}
