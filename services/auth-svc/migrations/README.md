# auth_svc migrations (sqlx)

Versioned SQL migrations for the `auth_svc` schema, applied at startup by
`infra/db/postgres.rs::run_migrations` (`sqlx::migrate!`). The pool pins
`search_path=auth_svc`, so unqualified objects land in this service's schema and
`_sqlx_migrations` is tracked there too.

Rules (DATA-STRATEGY.md §3): one schema per service; **no table may reference
another service's schema**.

- `0001_init.sql` — schema + `tenants`, `users`, `roles`, `permissions`,
  `user_roles`, `role_permissions`, `refresh_tokens`, `outbox_events` (outbox
  pattern, §3.2) + indexes; seeds the global `permissions` catalogue
  (SECURITY.md §4.3). Tenant-scoped roles + `role_permissions` are created per
  tenant by `create_tenant` / `register` (Phase 1.3).
- `0002_rbac_contacts_activities_leave.sql` — extends the `permissions`
  catalogue with CRM contacts/activities + HRM leave codes.
- `0003_super_admin.sql` — adds the platform `auth_tenant_manage` permission and
  seeds a `SUPER_ADMIN` role for the `system` tenant, enabling cross-tenant
  management via `GET`/`POST /api/v1/auth/tenants`.
