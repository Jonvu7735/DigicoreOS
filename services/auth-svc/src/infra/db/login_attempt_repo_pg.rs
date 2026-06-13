//! `LoginAttemptRepository` backed by Postgres (`auth_svc.login_attempts`).
//!
//! Brute-force guard for /auth/login (SECURITY.md §5.2). The counter and lock
//! are stored per (lowercased) email so a lockout is shared across replicas.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

use crate::domain::identity::ports::{LoginAttemptRepository, LoginLockStatus};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::Email;
use crate::infra::db::map_db_err;

pub struct PgLoginAttemptRepo {
    pool: PgPool,
}

impl PgLoginAttemptRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Emails are matched case-insensitively (the lookup key is lowercased).
fn key(email: &Email) -> String {
    email.0.to_lowercase()
}

#[async_trait]
impl LoginAttemptRepository for PgLoginAttemptRepo {
    async fn status(&self, email: &Email) -> DomainResult<LoginLockStatus> {
        let row: Option<(i64, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT failed_count::bigint, locked_until FROM login_attempts WHERE email_lower = $1",
        )
        .bind(key(email))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        Ok(row
            .map(|(failed_count, locked_until)| LoginLockStatus {
                failed_count,
                locked_until,
            })
            .unwrap_or_default())
    }

    async fn record_failure(
        &self,
        email: &Email,
        now: DateTime<Utc>,
        threshold: i64,
        lock_for: Duration,
    ) -> DomainResult<LoginLockStatus> {
        // Atomic upsert + lock decision in one statement: increment the counter,
        // and once it reaches `threshold` set `locked_until` and reset the count
        // so the next window starts clean. `GREATEST(... , 1)` covers the insert.
        let (failed_count, locked_until): (i64, Option<DateTime<Utc>>) = sqlx::query_as(
            "INSERT INTO login_attempts (email_lower, failed_count, locked_until, last_failed_at) \
             VALUES ($1, 1, NULL, $2) \
             ON CONFLICT (email_lower) DO UPDATE SET \
                 failed_count = CASE \
                     WHEN login_attempts.failed_count + 1 >= $3 THEN 0 \
                     ELSE login_attempts.failed_count + 1 END, \
                 locked_until = CASE \
                     WHEN login_attempts.failed_count + 1 >= $3 THEN $4 \
                     ELSE login_attempts.locked_until END, \
                 last_failed_at = $2 \
             RETURNING failed_count::bigint, locked_until",
        )
        .bind(key(email))
        .bind(now)
        .bind(threshold)
        .bind(now + lock_for)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;

        Ok(LoginLockStatus {
            failed_count,
            locked_until,
        })
    }

    async fn reset(&self, email: &Email) -> DomainResult<()> {
        sqlx::query("DELETE FROM login_attempts WHERE email_lower = $1")
            .bind(key(email))
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
