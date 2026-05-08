# Payment Gateway (Rust)

A highly resilient distributed payment gateway built in Rust for the fictional e-commerce platform FicMart. This project acts as an intermediary layer between an internal order service and an external banking provider. It focuses on solving the core challenges of distributed financial systems: network unreliability, data consistency, strict state validation, and double-charge prevention.

## Architecture Overview

The system is designed as a Cargo Workspace containing three primary components that work in tandem to process the payment lifecycle (Authorize -> Capture -> Void / Refund).

### 1. Payment Gateway Service
The Gateway is the primary service handling incoming requests from the e-commerce platform. It is responsible for orchestrating the payment lifecycle and interacting with the bank.
* **Idempotency Layer**: Implements a strict "in-flight" database locking mechanism. If the e-commerce platform retries a request due to a network timeout, the gateway guarantees the bank is not hit twice and the original response is safely returned.
* **State Machine**: Enforces valid transitions. For example, a payment cannot be voided if it has already been captured, and it cannot be refunded if it hasn't been captured.
* **Bank Client**: Communicates with the mock bank via HTTP, utilizing exponential backoff and jitter to survive transient 5xx errors and network timeouts.
* **Recovery Worker**: A background Tokio task that periodically polls the database for "stuck" pending payments. If an authorization request timed out, the worker actively asks the bank for the true status and reconciles the gateway's database.

### 2. Mock Bank Service
The mock bank is a simulated credit card processor that behaves like a legacy financial institution.
* **Chaos Middleware**: Intentionally injects random latency (up to 2000ms) and random HTTP 500 Internal Server Errors to test the resilience of the Gateway.
* **State Validation**: Enforces its own strict rules (e.g., authorizations expire after 7 days, captures cannot exceed the authorized amount).
* **API Documentation**: Provides an interactive Swagger UI to explore and manually test the banking endpoints.

### 3. FICPAY Simulator (Dashboard)
A real-time dashboard that provides a visual simulation environment for the payment lifecycle.
* **Real-time Visualization**: Watch "packets" move between FicMart, the Gateway, and the Bank Core in real-time.
* **Chaos Engineering Panel**: Dynamically adjust bank latency and failure probability via the UI.
* **Traffic Monitor**: Inspect the raw HTTP requests and responses as they hit each node.

### 4. PostgreSQL Database
The state is managed in a single relational database containing three core tables:
* `payments`: Stores the current state, amount, and bank references for every order.
* `idempotency_requests`: Stores the cached HTTP responses and locks for concurrent duplicate requests.
* `payment_events`: An append-only audit trail logging every state transition for receipts and debugging.

## Project Structure

```text
payment-gateway/
├── Cargo.toml                  (Workspace Root)
├── docker-compose.yml          (Orchestrates Postgres, Gateway, and Mock Bank)
├── .env.example                (Environment variable templates)
│
├── common/                     (Shared library)
│   └── src/lib.rs              (Shared domain types: CardDetails, Money, Config)
│
├── gateway/                    (Payment Gateway Crate)
│   ├── migrations/             (SQLx database migrations)
│   ├── src/
│   │   ├── bin/test_gateway.rs (Rust script for end-to-end testing)
│   │   ├── bank/               (Bank HTTP client and backoff logic)
│   │   ├── db/                 (SQLx repository for transactional updates)
│   │   ├── domain/             (Pure state machine and status enums)
│   │   ├── idempotency/        (Axum extractors for header validation)
│   │   ├── routes/             (API endpoints for Authorize, Capture, etc.)
│   │   └── workers/            (Background recovery worker logic)
│
├── dashboard/                  (FICPAY Simulator Dashboard)
│   ├── src/
│   │   ├── App.jsx             (Core simulation logic and UI)
│   │   └── index.css           (Brutalist high-contrast theme)
│   └── Dockerfile              (Nginx production build)
│
└── mock-bank/                  (Mock Bank Crate)
    ├── src/
    │   ├── cards.rs            (Hardcoded test cards and balances)
    │   ├── chaos.rs            (Middleware for latency and failure injection)
    │   ├── docs.rs             (Utoipa OpenAPI schema generation)
    │   ├── routes.rs           (Bank endpoints)
    │   └── state.rs            (In-memory thread-safe state storage)
```

## Running the Project

### Option A: Docker Compose (Recommended)
This approach bypasses any local environment setup and automatically handles database creation and network routing.

```bash
docker compose up --build
```
Once started, the dashboard will be available at **http://localhost:5173**.

### Option B: Local Execution
Requires Rust and PostgreSQL to be installed locally.

1. Copy the environment variables:
   ```bash
   cp .env.example .env
   ```
2. Ensure your local PostgreSQL instance is running and update the credentials in the `.env` file. Create a database named `mockbank`.
3. Build the workspace:
   ```bash
   cargo build
   ```
4. Start the Mock Bank in Terminal 1:
   ```bash
   cargo run -p mock-bank
   ```
5. Start the Gateway in Terminal 2 (Migrations will automatically apply on boot):
   ```bash
   cargo run -p gateway
   ```
6. Start the Dashboard in Terminal 3:
   ```bash
   cd dashboard
   npm install
   npm run dev
   ```
   The dashboard will be available at **http://localhost:5173**.

## API Documentation and Testing

### Mock Bank Swagger UI
Once the mock bank is running, you can view the complete API documentation and OpenAPI specification by visiting:
**http://localhost:8787/docs**

### End-to-End Test Script
The repository includes a native Rust test binary that simulates a full checkout lifecycle to verify idempotency, state transitions, and error handling. Ensure both services are running, then execute:

```bash
cargo run -p gateway --bin test_gateway
```
The script will output the HTTP status codes, durations, and response bodies to the terminal and save a detailed log to `test_results.json`.

## Test Cards

When submitting authorization requests to the Gateway, the Mock Bank will respond differently based on the card number used:

| Card Number | CVV | Expiry | Simulated Scenario |
| :--- | :--- | :--- | :--- |
| `4111111111111111` | `123` | `12/2030` | Successful Authorization (High balance) |
| `4242424242424242` | `456` | `06/2030` | Successful Authorization (Low balance) |
| `5555555555554444` | `789` | `09/2030` | Insufficient Funds (Returns 402 status) |
| `5105105105105100` | `321` | `03/2020` | Expired Card (Returns 422 status) |
