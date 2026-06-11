-- AI insights (ai_svc.insights). Tenant-scoped generated analyses.
CREATE TABLE IF NOT EXISTS insights (
    id         UUID PRIMARY KEY,
    tenant_id  TEXT        NOT NULL,
    category   TEXT        NOT NULL,          -- e.g. sales_anomaly, snapshot_digest
    summary    TEXT        NOT NULL,
    source_ref TEXT,                          -- trigger reference (e.g. snapshot id)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS insights_tenant_idx ON insights (tenant_id, created_at DESC);
