//! Postgres pool setup, pinned to this service's schema (`ai_svc`).

use std::str::FromStr;

use anyhow::Context;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

use crate::bootstrap::config::AppConfig;

/// Lazily-connecting pool pinned to `ai_svc` via `search_path`.
pub fn connect_lazy(config: &AppConfig) -> anyhow::Result<PgPool> {
    let options = PgConnectOptions::from_str(&config.database_url)?
        .options([("search_path", config.database_schema.as_str())]);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect_lazy_with(options);

    Ok(pool)
}

/// Apply pending migrations into the `ai_svc` schema at startup.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("failed to apply ai_svc migrations")?;
    Ok(())
}
