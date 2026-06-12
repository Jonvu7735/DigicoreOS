-- Deals read model (reporting_svc.deal_facts): one row per deal, tracking its
-- CURRENT pipeline stage. Projected from DealCreated (initial stage) and
-- DealStageChanged (stage transitions); see infra/db/deals_repo_pg.rs.
--
-- DealCreated is idempotent on deal_id (ON CONFLICT DO NOTHING). Stage changes
-- are applied monotonically by event time (updated_at), so an out-of-order or
-- duplicate DealStageChanged never regresses the stage. The CRM funnel report
-- is `count(*) GROUP BY stage`.
CREATE TABLE IF NOT EXISTS deal_facts (
    deal_id         TEXT PRIMARY KEY,
    tenant_id       TEXT        NOT NULL,
    customer_id     TEXT        NOT NULL,
    amount_estimate BIGINT      NOT NULL,   -- minor currency units
    stage           TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS deal_facts_tenant_stage_idx ON deal_facts (tenant_id, stage);
