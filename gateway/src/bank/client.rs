use crate::bank::{BankClient, BankError, types::*};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::time::Duration;

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
        let resp = self.client.post(&url)
            .header("Idempotency-Key", idempotency_key)
            .json(body)
            .send()
            .await?;

        match resp.status() {
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
