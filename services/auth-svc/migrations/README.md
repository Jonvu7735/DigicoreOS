# auth_svc migrations (sqlx)

TODO(Phase 1.2): `0001_init.sql` creating, inside schema `auth_svc`:
`tenants`, `users`, `roles`, `permissions`, `user_roles`, `role_permissions`,
`refresh_tokens`, and `outbox_events` (outbox pattern, DATA-STRATEGY.md §3.2).
No table may reference another service's schema.
