CREATE TABLE payment_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id  UUID NOT NULL REFERENCES payments(id),
    from_status payment_status,
    to_status   payment_status NOT NULL,
    actor       TEXT NOT NULL,      -- 'client', 'bank', 'worker'
    metadata    JSONB,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
