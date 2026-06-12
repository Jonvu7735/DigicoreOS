//! `AttendanceProjection` backed by Postgres (`reporting_svc.attendance_facts`).
//!
//! Idempotency: the natural key `(tenant_id, employee_id, work_date)` dedupes
//! re-deliveries; a same-day follow-up event (e.g. carrying `check_out`) merges
//! via `COALESCE` without adding a row.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::attendance::entities::{AttendanceSummary, NewAttendanceFact};
use crate::domain::attendance::ports::AttendanceProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

pub struct PgAttendanceRepo {
    pool: PgPool,
}

impl PgAttendanceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttendanceProjection for PgAttendanceRepo {
    async fn apply_attendance_recorded(&self, rec: &NewAttendanceFact) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO attendance_facts \
             (tenant_id, employee_id, work_date, check_in, check_out) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (tenant_id, employee_id, work_date) DO UPDATE SET \
                check_in = COALESCE(EXCLUDED.check_in, attendance_facts.check_in), \
                check_out = COALESCE(EXCLUDED.check_out, attendance_facts.check_out)",
        )
        .bind(&rec.tenant_id.0)
        .bind(&rec.employee_id)
        .bind(&rec.work_date)
        .bind(&rec.check_in)
        .bind(&rec.check_out)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn summary(&self, tenant: &TenantId) -> DomainResult<AttendanceSummary> {
        let (record_count, present_employees): (i64, i64) = sqlx::query_as(
            "SELECT count(*)::bigint, count(DISTINCT employee_id)::bigint \
             FROM attendance_facts WHERE tenant_id = $1",
        )
        .bind(&tenant.0)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(AttendanceSummary {
            record_count,
            present_employees,
        })
    }
}

#[cfg(test)]
mod db_integration {
    //! DB-backed tests; run only when `TEST_DATABASE_URL` is set (CI `integration`).

    use std::str::FromStr;

    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use uuid::Uuid;

    use super::*;

    async fn pool_or_skip() -> Option<PgPool> {
        let url = std::env::var("TEST_DATABASE_URL").ok()?;
        let opts = PgConnectOptions::from_str(&url)
            .expect("valid TEST_DATABASE_URL")
            .options([("search_path", "reporting_svc")]);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .expect("connect to test db");
        crate::infra::db::postgres::run_migrations(&pool, "reporting_svc")
            .await
            .expect("apply migrations");
        Some(pool)
    }

    fn rec(tenant: &str, employee: &str, date: &str) -> NewAttendanceFact {
        NewAttendanceFact {
            tenant_id: TenantId(tenant.into()),
            employee_id: employee.into(),
            work_date: date.into(),
            check_in: Some("09:00:00".into()),
            check_out: None,
        }
    }

    #[tokio::test]
    async fn attendance_is_idempotent_and_rolls_up() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgAttendanceRepo::new(pool);
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let emp = format!("e-{}", Uuid::now_v7());

        repo.apply_attendance_recorded(&rec(&tenant.0, &emp, "2026-06-12"))
            .await
            .unwrap();
        // Same (employee, date) carrying a check_out: merges, no new row.
        repo.apply_attendance_recorded(&NewAttendanceFact {
            check_out: Some("17:30:00".into()),
            ..rec(&tenant.0, &emp, "2026-06-12")
        })
        .await
        .unwrap();
        // Same employee, different day adds a record.
        repo.apply_attendance_recorded(&rec(&tenant.0, &emp, "2026-06-13"))
            .await
            .unwrap();

        let summary = repo.summary(&tenant).await.unwrap();
        assert_eq!(summary.record_count, 2);
        assert_eq!(summary.present_employees, 1);
    }
}
