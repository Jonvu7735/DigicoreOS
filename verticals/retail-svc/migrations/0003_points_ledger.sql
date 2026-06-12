-- Points ledger: one row per points movement (earn or redeem), written in the
-- same transaction as the balance change it records, so a customer's history
-- can never drift from their account balance.

CREATE TABLE IF NOT EXISTS loyalty_points_ledger (
    id            UUID PRIMARY KEY,
    tenant_id     TEXT        NOT NULL,
    customer_id   TEXT        NOT NULL,
    kind          TEXT        NOT NULL,   -- EARN | REDEEM
    points        BIGINT      NOT NULL,   -- positive magnitude
    balance_after BIGINT      NOT NULL,
    reason        TEXT,                   -- order id for an earn; null for a redeem
    at            TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS loyalty_points_ledger_customer_idx
    ON loyalty_points_ledger (tenant_id, customer_id, at DESC);
