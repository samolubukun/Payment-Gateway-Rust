CREATE TYPE idempotency_status AS ENUM ('in_flight', 'complete');

CREATE TABLE idempotency_requests (
    key             TEXT PRIMARY KEY,
    merchant_id     TEXT NOT NULL,
    request_hash    TEXT NOT NULL,
    response_status INT,
    response_body   JSONB,
    status          idempotency_status NOT NULL DEFAULT 'in_flight',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '24 hours'
);
