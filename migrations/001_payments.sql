CREATE TYPE payment_status AS ENUM (
    'pending', 'authorized', 'captured', 'voided', 'refunded'
);

CREATE TABLE payments (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idempotency_key     TEXT UNIQUE NOT NULL,
    order_id            TEXT NOT NULL,
    customer_id         TEXT NOT NULL,
    amount_cents        BIGINT NOT NULL CHECK (amount_cents > 0),
    currency            CHAR(3) NOT NULL DEFAULT 'USD',
    status              payment_status NOT NULL DEFAULT 'pending',
    bank_auth_id        TEXT,
    bank_capture_id     TEXT,
    bank_void_id        TEXT,
    bank_refund_id      TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    authorized_at       TIMESTAMPTZ,
    captured_at         TIMESTAMPTZ,
    voided_at           TIMESTAMPTZ,
    refunded_at         TIMESTAMPTZ,
    locked_until        TIMESTAMPTZ
);
