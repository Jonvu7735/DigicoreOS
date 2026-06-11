-- ai_svc schema bootstrap (DATA-STRATEGY.md §3.1-3.2).
-- One schema per service; no table references another service's schema.
-- Insight / embedding / model-config tables are added by later migrations.

CREATE SCHEMA IF NOT EXISTS ai_svc;

-- Transactional outbox (DATA-STRATEGY.md §3.2): ai-svc PRODUCES
-- AiInsightGenerated; the relay publishes unpublished rows to NATS.
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

-- Consumer idempotency: ai-svc also SUBSCRIBES (e.g. ReportSnapshotCreated) and
-- records applied event_ids so at-least-once re-deliveries are ignored.
CREATE TABLE IF NOT EXISTS processed_events (
    event_id     UUID PRIMARY KEY,
    subject      TEXT        NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
