//! `InventoryProjection` backed by Postgres (`reporting_svc.stock_facts`).
//!
//! Idempotency: each `StockAdjusted` is claimed in `processed_events` inside the
//! same transaction as the running-sum update, so an at-least-once re-delivery
//! is a no-op (the signed delta is additive and would otherwise double-count).

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::inventory::entities::{StockAdjustment, StockLevel};
use crate::domain::inventory::ports::InventoryProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

pub struct PgInventoryRepo {
    pool: PgPool,
}

impl PgInventoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InventoryProjection for PgInventoryRepo {
    async fn apply_stock_adjusted(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        adj: &StockAdjustment,
    ) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        // Claim the event_id; if already present this is a re-delivery -> skip.
        let claimed = sqlx::query(
            "INSERT INTO processed_events (event_id, subject) VALUES ($1, $2) \
             ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(event_id)
        .bind("platform.erp.inventory.stock_adjusted")
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?
        .rows_affected();

        if claimed == 0 {
            tx.commit().await.map_err(map_db_err)?;
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO stock_facts (tenant_id, product_id, warehouse_id, quantity, updated_at) \
             VALUES ($1, $2, $3, $4, now()) \
             ON CONFLICT (tenant_id, product_id, warehouse_id) DO UPDATE SET \
                quantity = stock_facts.quantity + EXCLUDED.quantity, \
                updated_at = now()",
        )
        .bind(&tenant.0)
        .bind(&adj.product_id)
        .bind(&adj.warehouse_id)
        .bind(adj.delta)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn summary(&self, tenant: &TenantId) -> DomainResult<Vec<StockLevel>> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT product_id, warehouse_id, quantity FROM stock_facts \
             WHERE tenant_id = $1 ORDER BY product_id, warehouse_id",
        )
        .bind(&tenant.0)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|(product_id, warehouse_id, quantity)| StockLevel {
                product_id,
                warehouse_id,
                quantity,
            })
            .collect())
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

    fn adj(product: &str, warehouse: &str, delta: i64) -> StockAdjustment {
        StockAdjustment {
            product_id: product.into(),
            warehouse_id: warehouse.into(),
            delta,
        }
    }

    #[tokio::test]
    async fn stock_adjusted_is_idempotent_and_sums() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgInventoryRepo::new(pool);
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let product = format!("p-{}", Uuid::now_v7());
        let event_id = Uuid::now_v7();

        repo.apply_stock_adjusted(event_id, &tenant, &adj(&product, "w1", 100))
            .await
            .unwrap();
        // Re-deliver the SAME event_id: must NOT double-count.
        repo.apply_stock_adjusted(event_id, &tenant, &adj(&product, "w1", 100))
            .await
            .unwrap();
        // A distinct event applies (here a negative delta = shipment out).
        repo.apply_stock_adjusted(Uuid::now_v7(), &tenant, &adj(&product, "w1", -30))
            .await
            .unwrap();

        let levels = repo.summary(&tenant).await.unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].product_id, product);
        assert_eq!(levels[0].warehouse_id, "w1");
        assert_eq!(levels[0].quantity, 70);
    }
}
