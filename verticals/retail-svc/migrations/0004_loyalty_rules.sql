-- Per-tenant loyalty program rules. Absent row = platform defaults
-- (1 point per 100 minor units; SILVER >= 100_000; GOLD >= 1_000_000).

CREATE TABLE IF NOT EXISTS loyalty_rules (
    tenant_id       TEXT PRIMARY KEY,
    minor_per_point BIGINT      NOT NULL,
    silver_min      BIGINT      NOT NULL,
    gold_min        BIGINT      NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
