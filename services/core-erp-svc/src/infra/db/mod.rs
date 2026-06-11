//! Postgres adapters for the `erp_core_svc` schema (DATA-STRATEGY.md §3.1).

pub mod inventory_repo_pg;
pub mod invoice_repo_pg;
pub mod order_repo_pg;
pub mod payment_repo_pg;
pub mod postgres;
pub mod product_repo_pg;

use crate::domain::shared::error::DomainError;

/// Map a read/query error to `DomainError::Internal` (logged, not leaked).
pub(crate) fn map_db_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(format!("db error: {e}"))
}

/// Map a write error, turning a unique-constraint violation into `Conflict`.
pub(crate) fn map_write_err(e: sqlx::Error) -> DomainError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return DomainError::Conflict(db.constraint().unwrap_or("unique").to_string());
        }
    }
    DomainError::Internal(format!("db error: {e}"))
}

#[cfg(test)]
pub(crate) mod testutil {
    //! Shared helpers for DB-backed tests. They run ONLY when `TEST_DATABASE_URL`
    //! is set (the CI `integration` job, against a real Postgres); otherwise the
    //! callers skip, so the default `cargo test` stays DB-free.

    use std::str::FromStr;

    use chrono::Utc;
    use platform_outbox::OutboxMessage;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use sqlx::PgPool;
    use uuid::Uuid;

    /// Connect to the test DB pinned to `erp_core_svc` and apply migrations, or
    /// `None` when `TEST_DATABASE_URL` is unset.
    pub(crate) async fn pool_or_skip() -> Option<PgPool> {
        let url = std::env::var("TEST_DATABASE_URL").ok()?;
        let opts = PgConnectOptions::from_str(&url)
            .expect("valid TEST_DATABASE_URL")
            .options([("search_path", "erp_core_svc")]);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .expect("connect to test db");
        crate::infra::db::postgres::run_migrations(&pool, "erp_core_svc")
            .await
            .expect("apply migrations");
        Some(pool)
    }

    /// A throwaway outbox message for tests that must pass one to a repo.
    pub(crate) fn fake_event(tenant: &str, event_type: &str) -> OutboxMessage {
        OutboxMessage {
            event_id: Uuid::now_v7(),
            occurred_at: Utc::now(),
            tenant_id: tenant.to_string(),
            aggregate_type: "test".into(),
            aggregate_id: "t".into(),
            event_type: event_type.to_string(),
            version: 1,
            subject: "platform.erp.test".into(),
            payload: serde_json::json!({}),
        }
    }
}
