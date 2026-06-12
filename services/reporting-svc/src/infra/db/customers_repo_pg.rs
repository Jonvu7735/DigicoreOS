//! `CustomersProjection` backed by Postgres (`reporting_svc.customer_facts`).
//!
//! Idempotency: `customer_id` is the primary key and inserts use
//! `ON CONFLICT DO NOTHING`, so re-delivery of the same `CustomerCreated` is a
//! no-op.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::customers::entities::{NewCustomerFact, ReportedCustomer};
use crate::domain::customers::ports::CustomersProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type CustomerRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

pub struct PgCustomersRepo {
    pool: PgPool,
}

impl PgCustomersRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomersProjection for PgCustomersRepo {
    async fn apply_customer_created(&self, fact: &NewCustomerFact) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO customer_facts \
             (customer_id, tenant_id, name, email, segment, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (customer_id) DO NOTHING",
        )
        .bind(&fact.customer_id)
        .bind(&fact.tenant_id.0)
        .bind(&fact.name)
        .bind(&fact.email)
        .bind(&fact.segment)
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
    ) -> DomainResult<Vec<ReportedCustomer>> {
        let rows: Vec<CustomerRow> = sqlx::query_as(
            "SELECT customer_id, tenant_id, name, email, segment, created_at \
             FROM customer_facts WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|r| ReportedCustomer {
                customer_id: r.0,
                tenant_id: TenantId(r.1),
                name: r.2,
                email: r.3,
                segment: r.4,
                created_at: r.5,
            })
            .collect())
    }

    async fn count(&self, tenant: &TenantId) -> DomainResult<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT count(*)::bigint FROM customer_facts WHERE tenant_id = $1")
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

    fn fact(tenant: &str, customer_id: &str, segment: &str) -> NewCustomerFact {
        NewCustomerFact {
            customer_id: customer_id.into(),
            tenant_id: TenantId(tenant.into()),
            name: "Acme Co".into(),
            email: Some("ops@acme.test".into()),
            segment: Some(segment.into()),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn customer_created_is_idempotent_and_lists() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgCustomersRepo::new(pool);
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let cid = format!("c-{}", Uuid::now_v7());

        repo.apply_customer_created(&fact(&tenant.0, &cid, "VIP"))
            .await
            .unwrap();
        // Re-deliver the SAME customer_id: must not duplicate.
        repo.apply_customer_created(&fact(&tenant.0, &cid, "VIP"))
            .await
            .unwrap();
        // A distinct customer adds a row.
        repo.apply_customer_created(&fact(&tenant.0, &format!("c-{}", Uuid::now_v7()), "SMB"))
            .await
            .unwrap();

        let listed = repo.list(&tenant, 50, 0).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(repo.count(&tenant).await.unwrap(), 2);
    }
}
