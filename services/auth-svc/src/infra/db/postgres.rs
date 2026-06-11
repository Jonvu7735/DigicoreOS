//! Postgres pool setup, pinned to this service's schema.
//!
//! RULE (DATA-STRATEGY.md): auth-svc connects ONLY to schema `auth_svc` on the
//! shared instance. The `search_path` is forced here so no query can
//! accidentally read another service's schema.

use std::str::FromStr;

use anyhow::Context;
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

/// Apply pending migrations from `./migrations` into the `auth_svc` schema.
///
/// Run once at startup (bootstrap/wiring.rs). This needs a live database, so a
/// failure here is fatal: the service cannot serve auth without its schema.
/// `_sqlx_migrations` is created inside `auth_svc` because the pool's
/// `search_path` is pinned to it.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("failed to apply auth_svc migrations")?;
    Ok(())
}
