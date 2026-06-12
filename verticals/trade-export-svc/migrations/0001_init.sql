-- trade_export_svc schema bootstrap (DATA-STRATEGY.md §3.1-3.2).
-- One schema per service; no table references another service's schema.

CREATE SCHEMA IF NOT EXISTS trade_export_svc;

-- Transactional outbox (DATA-STRATEGY.md §3.2): events written in the same tx as
-- state; a relay worker publishes unpublished rows to NATS.
CREATE TABLE IF NOT EXISTS outbox_events (
    id             UUID PRIMARY KEY,
    occurred_at    TIMESTAMPTZ NOT NULL,
    tenant_id      TEXT        NOT NULL,
    aggregate_type TEXT        NOT NULL,
    aggregate_id   TEXT        NOT NULL,
    event_type     TEXT        NOT NULL,
    version        INT         NOT NULL,
    subject        TEXT        NOT NULL,
    payload        JSONB       NOT NULL,
    published_at   TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS outbox_unpublished_idx
    ON outbox_events (created_at) WHERE published_at IS NULL;
