-- Sales summary read model (reporting_svc.sales_summary). A per-tenant rollup
-- projected from the OrderPaid event stream (see infra/db/sales_repo_pg.rs).
CREATE TABLE IF NOT EXISTS sales_summary (
    tenant_id     TEXT PRIMARY KEY,
    total_paid    BIGINT      NOT NULL DEFAULT 0,   -- minor currency units
    payment_count BIGINT      NOT NULL DEFAULT 0,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
