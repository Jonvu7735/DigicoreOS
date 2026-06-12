//! `EmployeesProjection` backed by Postgres (`reporting_svc.employee_facts`).
//!
//! Idempotency: `employee_id` is the primary key and inserts use
//! `ON CONFLICT DO NOTHING`, so re-delivery of the same `EmployeeHired` is a
//! no-op.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::employees::entities::{NewEmployeeFact, ReportedEmployee};
use crate::domain::employees::ports::EmployeesProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type EmployeeRow = (String, String, String, String, DateTime<Utc>);

pub struct PgEmployeesRepo {
    pool: PgPool,
}

impl PgEmployeesRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmployeesProjection for PgEmployeesRepo {
    async fn apply_employee_hired(&self, fact: &NewEmployeeFact) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO employee_facts \
             (employee_id, tenant_id, full_name, position, created_at) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (employee_id) DO NOTHING",
        )
        .bind(&fact.employee_id)
        .bind(&fact.tenant_id.0)
        .bind(&fact.full_name)
        .bind(&fact.position)
        .bind(fact.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn list(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ReportedEmployee>> {
        let rows: Vec<EmployeeRow> = sqlx::query_as(
            "SELECT employee_id, tenant_id, full_name, position, created_at \
             FROM employee_facts WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|r| ReportedEmployee {
                employee_id: r.0,
                tenant_id: TenantId(r.1),
                full_name: r.2,
                position: r.3,
                created_at: r.4,
            })
            .collect())
    }

    async fn count(&self, tenant: &TenantId) -> DomainResult<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT count(*)::bigint FROM employee_facts WHERE tenant_id = $1")
                .bind(&tenant.0)
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_err)?;
        Ok(count)
    }
}

#[cfg(test)]
mod db_integration {
    //! DB-backed tests; run only when `TEST_DATABASE_URL` is set (CI `integration`).

    use std::str::FromStr;

    use chrono::Utc;
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

    fn fact(tenant: &str, employee_id: &str, position: &str) -> NewEmployeeFact {
        NewEmployeeFact {
            employee_id: employee_id.into(),
            tenant_id: TenantId(tenant.into()),
            full_name: "Jane Doe".into(),
            position: position.into(),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn employee_hired_is_idempotent_and_lists() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgEmployeesRepo::new(pool);
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let eid = format!("e-{}", Uuid::now_v7());

        repo.apply_employee_hired(&fact(&tenant.0, &eid, "Engineer"))
            .await
            .unwrap();
        // Re-deliver the SAME employee_id: must not duplicate.
        repo.apply_employee_hired(&fact(&tenant.0, &eid, "Engineer"))
            .await
            .unwrap();
        // A distinct employee adds a row.
        repo.apply_employee_hired(&fact(&tenant.0, &format!("e-{}", Uuid::now_v7()), "Sales"))
            .await
            .unwrap();

        let listed = repo.list(&tenant, 50, 0).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(repo.count(&tenant).await.unwrap(), 2);
    }
}
