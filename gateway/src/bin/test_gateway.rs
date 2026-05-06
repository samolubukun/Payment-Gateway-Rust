use common::CardDetails;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const GATEWAY_URL: &str = "http://localhost:3000/v1/payments";
const MERCHANT_ID: &str = "merch_test_123";

#[derive(serde::Serialize)]
struct TestResult {
    name: String,
    status: u16,
    duration_ms: u128,
    response: Value,
}

async fn make_request(
    client: &Client,
    name: &str,
    method: reqwest::Method,
    url: &str,
    body: Option<Value>,
    idempotency_key: Option<&str>,
    results: &mut Vec<TestResult>,
) -> Option<Value> {
    println!("\n=========================================");
    println!("🚀 RUNNING TEST: {}", name);
    println!("=========================================");

    let mut builder = client.request(method, url)
        .header("X-Merchant-Id", MERCHANT_ID)
        .header("Content-Type", "application/json");

    if let Some(key) = idempotency_key {
        builder = builder.header("Idempotency-Key", key);
    }

    if let Some(b) = body {
        builder = builder.json(&b);
    }

    let start = Instant::now();
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let duration = start.elapsed().as_millis();
            let data: Value = response.json().await.unwrap_or(json!({ "error": "failed to parse json" }));

            println!("Status: {} ({}ms)", status, duration);
            println!("Response: {}", serde_json::to_string_pretty(&data).unwrap());

            results.push(TestResult {
                name: name.to_string(),
                status: status.as_u16(),
                duration_ms: duration,
                response: data.clone(),
            });

            Some(data)
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            results.push(TestResult {
                name: name.to_string(),
                status: 500,
                duration_ms: start.elapsed().as_millis(),
                response: json!({ "error": e.to_string() }),
            });
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Rust Gateway Tests...\n");
    let client = Client::new();
    let mut results = Vec::new();

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let idempotency_key = format!("idem_{}", timestamp);

    // 1. Happy Path Authorization
    let auth_body = json!({
        "order_id": format!("ord_{}", timestamp),
        "customer_id": "cust_999",
        "amount_cents": 15000,
        "currency": "USD",
        "card": {
            "number": "4111111111111111", // Happy Path Card
            "exp_month": 12,
            "exp_year": 2030,
            "cvv": "123"
        }
    });

    let auth_data = make_request(
        &client,
        "1. Authorize Payment (Happy Path)",
        reqwest::Method::POST,
        GATEWAY_URL,
        Some(auth_body),
        Some(&idempotency_key),
        &mut results,
    ).await;

    // 2. Test Idempotency (Same Request)
    let idem_body = json!({
        "order_id": format!("ord_different_{}", timestamp), // Changed body to prove caching
        "customer_id": "cust_999",
        "amount_cents": 15000,
        "currency": "USD",
        "card": {
            "number": "4111111111111111",
            "exp_month": 12,
            "exp_year": 2030,
            "cvv": "123"
        }
    });

    make_request(
        &client,
        "2. Test Idempotency (Should return cached response immediately)",
        reqwest::Method::POST,
        GATEWAY_URL,
        Some(idem_body),
        Some(&idempotency_key), // Exact same key
        &mut results,
    ).await;

    if let Some(data) = auth_data {
        if let Some(payment_id) = data.get("id").and_then(|v| v.as_str()) {
            // 3. Fetch Payment Status
            make_request(
                &client,
                "3. Fetch Payment Status",
                reqwest::Method::GET,
                &format!("{}/{}", GATEWAY_URL, payment_id),
                None,
                None,
                &mut results,
            ).await;

            // 4. Capture Payment
            make_request(
                &client,
                "4. Capture Payment",
                reqwest::Method::POST,
                &format!("{}/{}/capture", GATEWAY_URL, payment_id),
                None,
                Some(&format!("idem_cap_{}", timestamp)),
                &mut results,
            ).await;
        }
    }

    // 5. Test Insufficient Funds
    let broke_body = json!({
        "order_id": format!("ord_broke_{}", timestamp),
        "customer_id": "cust_broke",
        "amount_cents": 50000,
        "currency": "USD",
        "card": {
            "number": "5555555555554444", // Insufficient Funds Card
            "exp_month": 9,
            "exp_year": 2030,
            "cvv": "789"
        }
    });

    make_request(
        &client,
        "5. Authorize Payment (Insufficient Funds)",
        reqwest::Method::POST,
        GATEWAY_URL,
        Some(broke_body),
        Some(&format!("idem_broke_{}", timestamp)),
        &mut results,
    ).await;

    // Save results to file
    std::fs::write(
        "test_results.json",
        serde_json::to_string_pretty(&results)?,
    )?;

    println!("\n✅ Rust tests completed! Results saved to test_results.json");

    Ok(())
}
