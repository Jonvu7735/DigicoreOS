//! Postgres pool setup, pinned to this service's schema.
//!
//! RULE (DATA-STRATEGY.md): auth-svc connects ONLY to schema `auth_svc` on the
//! shared instance. The `search_path` is forced here so no query can
//! accidentally read another service's schema.

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

use crate::bootstrap::config::AppConfig;

/// Create a lazily-connecting pool. The first real connection happens on first
/// use, so the service can start before Postgres is reachable; `/ready`
/// reflects true connectivity.
pub fn connect_lazy(config: &AppConfig) -> anyhow::Result<PgPool> {
    let options = PgConnectOptions::from_str(&config.database_url)?
        .options([("search_path", config.database_schema.as_str())]);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        // Keep readiness probes fast: fail in 3s instead of the 30s default
        // when Postgres is unreachable (K8s probe timeouts are short).
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect_lazy_with(options);

    Ok(pool)
}

// TODO(Phase 1.2): run `sqlx::migrate!("./migrations")` at startup once the
// initial schema migration (tenants, users, roles, permissions, user_roles,
// role_permissions, refresh_tokens, outbox_events) is written.
