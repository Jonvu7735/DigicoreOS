//! `OrdersProjection` backed by Postgres (`reporting_svc.order_facts`).
//!
//! Idempotency: `order_id` is the primary key and inserts use
//! `ON CONFLICT DO NOTHING`, so re-delivery of the same `OrderCreated` is a no-op.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::orders::entities::{NewOrderFact, OrdersOverview, ReportedOrder};
use crate::domain::orders::ports::OrdersProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

type OrderRow = (String, String, String, i64, String, String, DateTime<Utc>);

pub struct PgOrdersRepo {
    pool: PgPool,
}

impl PgOrdersRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrdersProjection for PgOrdersRepo {
    async fn apply_order_created(&self, fact: &NewOrderFact) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO order_facts \
             (order_id, tenant_id, customer_id, total_amount, currency, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (order_id) DO NOTHING",
        )
        .bind(&fact.order_id)
        .bind(&fact.tenant_id.0)
        .bind(&fact.customer_id)
        .bind(fact.total_amount)
        .bind(&fact.currency)
        .bind(&fact.status)
        .bind(fact.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn list(
        &self,
        tenant: &TenantId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ReportedOrder>> {
        let rows: Vec<OrderRow> = sqlx::query_as(
            "SELECT order_id, tenant_id, customer_id, total_amount, currency, status, created_at \
             FROM order_facts WHERE tenant_id = $1 \
             AND ($2::timestamptz IS NULL OR created_at >= $2) \
             AND ($3::timestamptz IS NULL OR created_at < $3) \
             ORDER BY created_at DESC LIMIT $4 OFFSET $5",
        )
        .bind(&tenant.0)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|r| ReportedOrder {
                order_id: r.0,
                tenant_id: TenantId(r.1),
                customer_id: r.2,
                total_amount: Money(r.3),
                currency: r.4,
                status: r.5,
                created_at: r.6,
            })
            .collect())
    }

    async fn overview(&self, tenant: &TenantId) -> DomainResult<OrdersOverview> {
        let (count, total): (i64, i64) = sqlx::query_as(
            "SELECT count(*)::bigint, coalesce(sum(total_amount), 0)::bigint \
             FROM order_facts WHERE tenant_id = $1",
        )
        .bind(&tenant.0)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(OrdersOverview {
            order_count: count,
            total_amount: Money(total),
        })
    }
}

#[cfg(test)]
mod db_integration {
    //! DB-backed tests; run only when `TEST_DATABASE_URL` is set (CI `integration`).

    use std::str::FromStr;

    use chrono::{Duration, Utc};
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

    fn fact(tenant: &str, order_id: &str, amount: i64) -> NewOrderFact {
        NewOrderFact {
            order_id: order_id.into(),
            tenant_id: TenantId(tenant.into()),
            customer_id: "c1".into(),
            total_amount: amount,
            currency: "USD".into(),
            status: "NEW".into(),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn order_created_is_idempotent_and_lists() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgOrdersRepo::new(pool);
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let oid = format!("o-{}", Uuid::now_v7());

        repo.apply_order_created(&fact(&tenant.0, &oid, 2500))
            .await
            .unwrap();
        // Re-deliver the SAME order_id: must not duplicate.
        repo.apply_order_created(&fact(&tenant.0, &oid, 2500))
            .await
            .unwrap();
        // A distinct order adds a row.
        repo.apply_order_created(&fact(&tenant.0, &format!("o-{}", Uuid::now_v7()), 500))
            .await
            .unwrap();

        let listed = repo.list(&tenant, None, None, 50, 0).await.unwrap();
        assert_eq!(listed.len(), 2);
        // Date filter: a future-only window excludes today's orders.
        let tomorrow = Utc::now() + Duration::days(1);
        assert!(repo
            .list(&tenant, Some(tomorrow), None, 50, 0)
            .await
            .unwrap()
            .is_empty());
        // ...and an up-to-yesterday window excludes them too.
        let yesterday = Utc::now() - Duration::days(1);
        assert!(repo
            .list(&tenant, None, Some(yesterday), 50, 0)
            .await
            .unwrap()
            .is_empty());
        let ov = repo.overview(&tenant).await.unwrap();
        assert_eq!(ov.order_count, 2);
        assert_eq!(ov.total_amount.0, 3000);
    }
}
