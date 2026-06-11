-- ERP product catalogue (erp_core_svc.products). Tenant-scoped.
CREATE TABLE IF NOT EXISTS products (
    id         UUID PRIMARY KEY,
    tenant_id  TEXT        NOT NULL,
    sku        TEXT        NOT NULL,
    name       TEXT        NOT NULL,
    price      BIGINT      NOT NULL,            -- minor currency units
    currency   TEXT        NOT NULL,            -- ISO 4217
    is_active  BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, sku)
);
CREATE INDEX IF NOT EXISTS products_tenant_idx ON products (tenant_id);
