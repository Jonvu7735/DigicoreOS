-- Platform super-admin: a SUPER_ADMIN role + auth_tenant_manage permission,
-- seeded for a single "system" tenant, so platform operators can manage all
-- tenants (GET/POST /api/v1/auth/tenants).
--
-- SUPER_ADMIN is PLATFORM-ONLY: it is intentionally NOT one of the per-tenant
-- DEFAULT_ROLES seeded at registration, so no customer tenant ever grants the
-- cross-tenant auth_tenant_manage capability. A super-admin USER is provisioned
-- by assigning this role (e.g. via user_roles). Mirrors platform_auth::rbac
-- (ALL_PERMISSIONS + permissions_for("SUPER_ADMIN")).

-- 1) New platform-level permission in the global catalogue.
INSERT INTO permissions (code, description) VALUES
    ('auth_tenant_manage', 'Manage all tenants (platform super-admin)')
ON CONFLICT (code) DO NOTHING;

-- 2) The system tenant — home of the SUPER_ADMIN role; not a customer tenant.
INSERT INTO tenants (id, name, plan, is_active) VALUES
    ('system', 'Platform', 'system', TRUE)
ON CONFLICT (id) DO NOTHING;

-- 3) SUPER_ADMIN role for the system tenant (fixed id for idempotent grants).
INSERT INTO roles (id, tenant_id, name, description) VALUES
    ('00000000-0000-0000-0000-000000000001'::uuid, 'system', 'SUPER_ADMIN',
     'Platform super-administrator — manages all tenants')
ON CONFLICT (tenant_id, name) DO NOTHING;

-- 4) Grant the full catalogue (incl. auth_tenant_manage) to SUPER_ADMIN.
INSERT INTO role_permissions (role_id, permission_code)
SELECT '00000000-0000-0000-0000-000000000001'::uuid, code FROM permissions
ON CONFLICT (role_id, permission_code) DO NOTHING;
