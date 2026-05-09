use crate::bank::{BankClient, BankError, types::*};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

pub struct ReqwestBankClient {
    client: Client,
    base_url: String,
}

impl ReqwestBankClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { client, base_url }
    }

    async fn post<T, R>(&self, path: &str, body: &T, idempotency_key: &str) -> Result<R, BankError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt: u32 = 0;
        let max_attempts: u32 = 5;

        loop {
            let resp = self.client.post(&url)
                .header("Idempotency-Key", idempotency_key)
                .json(body)
                .send()
                .await;

            let parsed = match resp {
                Ok(resp) => match resp.status() {
                    StatusCode::OK => Ok(resp.json().await?),
                    StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
                        Err(BankError::ServerError { status: resp.status().as_u16() })
                    }
                    StatusCode::PAYMENT_REQUIRED => Err(BankError::InsufficientFunds),
                    StatusCode::UNPROCESSABLE_ENTITY => {
                        let err_data: serde_json::Value = resp.json().await.unwrap_or_default();
                        let code = err_data.get("code").and_then(|v| v.as_str()).unwrap_or("");
                        match code {
                            "INVALID_CARD" | "CARD_NOT_FOUND" => Err(BankError::InvalidCard),
                            "CARD_EXPIRED" => Err(BankError::AuthExpired),
                            _ => Err(BankError::CardDeclined { reason: err_data.get("error").and_then(|v| v.as_str()).unwrap_or("unknown").to_string() }),
                        }
                    }
                    _ => Err(BankError::ServerError { status: resp.status().as_u16() }),
                },
                Err(err) => {
                    if err.is_timeout() {
                        Err(BankError::Timeout)
                    } else {
                        Err(BankError::Network(err))
                    }
                }
            };

            match parsed {
                Ok(value) => return Ok(value),
                Err(err) => {
                    attempt += 1;
                    if attempt >= max_attempts || !err.is_retryable() {
                        return Err(err);
                    }
                    let base_delay_ms: u64 = 200;
                    let max_delay_ms: u64 = 2000;
                    let exp_delay = base_delay_ms.saturating_mul(2u64.saturating_pow(attempt - 1));
                    let jitter: u64 = rand::random::<u64>() % 200;
                    let delay_ms = (exp_delay + jitter).min(max_delay_ms);
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
}

#[async_trait]
impl BankClient for ReqwestBankClient {
    async fn authorize(&self, req: AuthorizeRequest, idempotency_key: &str) -> Result<BankAuthResponse, BankError> {
        self.post("/api/v1/authorizations", &req, idempotency_key).await
    }

    async fn capture(&self, req: CaptureRequest, idempotency_key: &str) -> Result<BankCaptureResponse, BankError> {
        self.post("/api/v1/captures", &req, idempotency_key).await
    }

    async fn void(&self, req: VoidRequest, idempotency_key: &str) -> Result<BankVoidResponse, BankError> {
        self.post("/api/v1/voids", &req, idempotency_key).await
    }

    async fn refund(&self, req: RefundRequest, idempotency_key: &str) -> Result<BankRefundResponse, BankError> {
        self.post("/api/v1/refunds", &req, idempotency_key).await
    }

    async fn get_auth_status(&self, auth_id: &str) -> Result<AuthStatus, BankError> {
        let url = format!("{}/api/v1/authorizations/{}", self.base_url, auth_id);
        let resp = self.client.get(&url).send().await?;
        if resp.status() == StatusCode::OK {
            Ok(resp.json().await?)
        } else {
             Err(BankError::ServerError { status: resp.status().as_u16() })
        }
    }
}
