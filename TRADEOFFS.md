# Tradeoffs & Design Decisions

## Architecture
- **Cargo Workspace**: Used a workspace to co-locate the `gateway`, `mock-bank`, and `common` crates. This facilitates sharing types and dependencies while keeping the services separate.
- **Trait-based Bank Client**: The `BankClient` trait allows for easy mocking in tests and provides a clean interface for different bank implementations.

## State Management
- **Database Enums**: Used PostgreSQL enums for `payment_status` and `idempotency_status` to ensure data integrity at the database level.
- **Append-only Events**: The `payment_events` table provides a full audit trail of all state transitions, which is critical for a payment system.

## Failure Handling
- **Retry Strategy**: Implemented exponential backoff with jitter for bank calls to handle transient network issues and bank 5xx errors.
- **Recovery Worker**: A background task reconciles payments that get stuck in the `pending` state due to crashes between the bank call and the database commit.

## Idempotency
- **In-flight Record Pattern**: Uses a database-backed idempotency layer to handle concurrent requests and ensure that the same operation is not executed twice.
- **Merchant Scoping**: Idempotency keys are scoped per merchant, allowing different merchants to use the same key strings without collision.

## What I'd do differently / Future Improvements
- **Circuit Breaker**: Add a circuit breaker to the bank client to avoid hammering the bank when it's down.
- **Webhook Callbacks**: Implement a webhook system to notify clients (like FicMart) of payment status changes asynchronously.
- **Metrics**: Add Prometheus metrics for monitoring request rates, error rates, and bank latency.
