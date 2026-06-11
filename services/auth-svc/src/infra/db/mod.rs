//! Postgres adapters for the `auth_svc` schema (DATA-STRATEGY.md §3.1).

pub mod postgres;
pub mod provisioning_repo_pg;
pub mod refresh_token_repo_pg;
pub mod role_repo_pg;
pub mod tenant_repo_pg;
pub mod user_repo_pg;

use crate::domain::shared::error::DomainError;

/// Map a read/query error to `DomainError::Internal` (logged, not leaked).
pub(crate) fn map_db_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(format!("db error: {e}"))
}

/// Map a write error, turning a unique-constraint violation into
/// `DomainError::Conflict` (e.g. duplicate email) and anything else into
/// `Internal`. The constraint name (when present) names the conflicting field.
pub(crate) fn map_write_err(e: sqlx::Error) -> DomainError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return DomainError::Conflict(db.constraint().unwrap_or("unique").to_string());
        }
    }
    DomainError::Internal(format!("db error: {e}"))
}
