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
pub async fn run_migrations(pool: &PgPool, schema: &str) -> anyhow::Result<()> {
    ensure_schema(pool, schema).await?;
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .with_context(|| format!("failed to apply {schema} migrations"))?;
    Ok(())
}

/// Create the service schema if absent, before sqlx creates its
/// `_sqlx_migrations` table there (the pool's `search_path` points only at this
/// schema, so on a fresh database the schema must exist first). `schema` must be
/// a plain SQL identifier — it is interpolated into DDL.
async fn ensure_schema(pool: &PgPool, schema: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !schema.is_empty()
            && schema
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "invalid schema name: {schema}"
    );
    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {schema}"))
        .execute(pool)
        .await
        .with_context(|| format!("failed to create schema {schema}"))?;
    Ok(())
}
