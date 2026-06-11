-- Report snapshots (reporting_svc.snapshots). Frozen read-model captures;
-- creating one emits ReportSnapshotCreated (via the outbox).
CREATE TABLE IF NOT EXISTS snapshots (
    id            UUID PRIMARY KEY,
    tenant_id     TEXT        NOT NULL,
    snapshot_type TEXT        NOT NULL,          -- e.g. sales
    payload       JSONB       NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS snapshots_tenant_idx ON snapshots (tenant_id, created_at DESC);
