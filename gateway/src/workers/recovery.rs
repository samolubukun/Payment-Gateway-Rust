use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use crate::bank::BankClient;
use crate::db::repository::{PaymentRepository, PaymentRow};
use crate::domain::payment::PaymentStatus;

pub async fn run(pool: PgPool, bank: Arc<dyn BankClient>) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        if let Err(e) = reconcile_once(&pool, &*bank).await {
            tracing::error!(error = %e, "recovery cycle failed");
        }
    }
}

async fn reconcile_once(pool: &PgPool, bank: &dyn BankClient) -> anyhow::Result<()> {
    // 1. Find stuck pending payments
    // (Simplified: for this mock we'll just check all pending payments)
    let stuck_payments = sqlx::query_as::<_, PaymentRow>(
        r#"
        SELECT id, idempotency_key, order_id, customer_id, amount_cents, currency, status, 
        bank_auth_id, bank_capture_id, bank_void_id, bank_refund_id, created_at, updated_at
        FROM payments 
        WHERE status = 'pending' AND updated_at < NOW() - INTERVAL '1 minute'
        "#
    )
    .fetch_all(pool)
    .await?;

    for payment in stuck_payments {
        tracing::info!(payment_id = %payment.id, "reconciling stuck payment");
        
        if let Some(bank_auth_id) = payment.bank_auth_id {
             match bank.get_auth_status(&bank_auth_id).await {
                Ok(status) => {
                    if status.status == "authorized" {
                        let mut tx = pool.begin().await?;
                        PaymentRepository::update_status(
                            &mut tx, payment.id, PaymentStatus::Pending, PaymentStatus::Authorized, None, "worker"
                        ).await?;
                        tx.commit().await?;
                        tracing::info!(payment_id = %payment.id, "recovered payment to authorized");
                    }
                }
                Err(e) => {
                    tracing::error!(payment_id = %payment.id, error = %e, "failed to check bank status during recovery");
                }
             }
        }
    }

    Ok(())
}
