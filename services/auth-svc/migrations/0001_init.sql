-- auth_svc schema – identity provider tables.
-- Refs: SERVICE-auth-svc.md §4, SECURITY.md §3, DATA-STRATEGY.md §3.1–3.2.
-- Rules: one schema per service; NO table references another service's schema.
-- The connection pool pins search_path=auth_svc (infra/db/postgres.rs); we also
-- create the schema here so the migration is self-contained.

CREATE SCHEMA IF NOT EXISTS auth_svc;

-- Tenants (SaaS customers). id is TEXT to match JWT `tenant_id` / event headers.
CREATE TABLE IF NOT EXISTS tenants (
    id         TEXT PRIMARY KEY,
    name       TEXT        NOT NULL,
    plan       TEXT        NOT NULL DEFAULT 'free',
    is_active  BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Users are global; tenant membership is expressed via tenant-scoped roles.
CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY,
    email         TEXT        NOT NULL,
    display_name  TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    is_active     BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Case-insensitive uniqueness for login by email.
CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_uidx ON users (lower(email));

-- Roles are scoped to a tenant (multi-tenant RBAC – SECURITY.md §3).
CREATE TABLE IF NOT EXISTS roles (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,            -- OWNER | ADMIN | MANAGER | STAFF | VIEWER
    description TEXT,
    UNIQUE (tenant_id, name)
);
CREATE INDEX IF NOT EXISTS roles_tenant_idx ON roles (tenant_id);

-- Global permission catalogue (resource_action codes – SECURITY.md §4.3).
CREATE TABLE IF NOT EXISTS permissions (
    code        TEXT PRIMARY KEY,
    description TEXT
);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_id         UUID NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_code TEXT NOT NULL REFERENCES permissions (code) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_code)
);

-- Refresh tokens: only the hash is stored (SECURITY.md – never store raw tokens).
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id         UUID PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    tenant_id  TEXT        NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS refresh_tokens_user_idx ON refresh_tokens (user_id);

-- Outbox (DATA-STRATEGY.md §3.2): events written in the SAME tx as state; a
-- relay worker (Phase 1.5) publishes unpublished rows to NATS.
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
-- Relay worker scans only unpublished rows, oldest first.
CREATE INDEX IF NOT EXISTS outbox_unpublished_idx
    ON outbox_events (created_at) WHERE published_at IS NULL;

-- Seed the global permission catalogue (SECURITY.md §4.3). Tenant-scoped roles
-- and their role_permissions are created per tenant by create_tenant/register.
INSERT INTO permissions (code, description) VALUES
    ('auth_user_read',          'Read users in the tenant'),
    ('auth_user_create',        'Create users in the tenant'),
    ('auth_user_update',        'Update users in the tenant'),
    ('auth_user_assign_role',   'Assign roles to users'),
    ('auth_tenant_read',        'Read tenant information'),
    ('auth_tenant_update_plan', 'Change the tenant subscription plan'),
    ('erp_order_read',          'Read orders'),
    ('erp_order_create',        'Create orders'),
    ('erp_order_update',        'Update orders'),
    ('erp_order_cancel',        'Cancel orders'),
    ('erp_invoice_read',        'Read invoices'),
    ('erp_invoice_create',      'Create invoices'),
    ('erp_invoice_cancel',      'Cancel invoices'),
    ('erp_product_read',        'Read products'),
    ('erp_product_create',      'Create products'),
    ('erp_product_update',      'Update products'),
    ('crm_customer_read',       'Read customers'),
    ('crm_customer_create',     'Create customers'),
    ('crm_customer_update',     'Update customers'),
    ('crm_deal_read',           'Read deals'),
    ('crm_deal_create',         'Create deals'),
    ('crm_deal_update',         'Update deals'),
    ('crm_deal_move_stage',     'Move a deal to another pipeline stage'),
    ('hrm_employee_read',       'Read employees'),
    ('hrm_employee_create',     'Create employees'),
    ('hrm_employee_update',     'Update employees'),
    ('hrm_attendance_read',     'Read attendance records'),
    ('hrm_attendance_create',   'Record attendance'),
    ('reporting_dashboard_view','View reporting dashboards'),
    ('reporting_report_export', 'Export reports'),
    ('ai_assistant_use',        'Use the AI assistant'),
    ('ai_config_manage',        'Manage AI configuration')
ON CONFLICT (code) DO NOTHING;
