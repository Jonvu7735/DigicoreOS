-- Loyalty accounts: the vertical's aggregate, one row per (tenant, customer).

CREATE TABLE IF NOT EXISTS loyalty_accounts (
    tenant_id            TEXT        NOT NULL,
    customer_id          TEXT        NOT NULL,
    points_balance       BIGINT      NOT NULL DEFAULT 0,
    lifetime_spend_minor BIGINT      NOT NULL DEFAULT 0,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, customer_id)
);

-- Idempotency ledger for the inbound consumer: an order event is accrued at most
-- once, even under at-least-once redelivery (the accrue tx inserts here first).
CREATE TABLE IF NOT EXISTS processed_events (
    event_id     UUID PRIMARY KEY,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
