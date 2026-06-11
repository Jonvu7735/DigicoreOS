//! `InventoryRepository` backed by Postgres (`erp_core_svc.stock_levels` /
//! `stock_adjustments`). Upsert + movement + outbox commit in one transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::inventory::entities::{StockAdjustment, StockLevel};
use crate::domain::inventory::ports::InventoryRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

type LevelRow = (String, Uuid, String, i64);
type AdjRow = (Uuid, String, Uuid, String, i64, String, DateTime<Utc>);

pub struct PgInventoryRepo {
    pool: PgPool,
}

impl PgInventoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InventoryRepository for PgInventoryRepo {
    async fn adjust(
        &self,
        adjustment: &StockAdjustment,
        event: &OutboxMessage,
    ) -> DomainResult<i64> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        let (quantity,): (i64,) = sqlx::query_as(
            "INSERT INTO stock_levels (tenant_id, product_id, warehouse_id, quantity) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (tenant_id, product_id, warehouse_id) \
             DO UPDATE SET quantity = stock_levels.quantity + $4 \
             RETURNING quantity",
        )
        .bind(&adjustment.tenant_id.0)
        .bind(adjustment.product_id)
        .bind(&adjustment.warehouse_id)
        .bind(adjustment.delta)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_err)?;

        if quantity < 0 {
            // tx drops here -> rollback.
            return Err(DomainError::Validation(format!(
                "insufficient stock: adjustment would make quantity {quantity}"
            )));
        }

        sqlx::query(
            "INSERT INTO stock_adjustments \
             (id, tenant_id, product_id, warehouse_id, delta, reason, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(adjustment.id)
        .bind(&adjustment.tenant_id.0)
        .bind(adjustment.product_id)
        .bind(&adjustment.warehouse_id)
        .bind(adjustment.delta)
        .bind(&adjustment.reason)
        .bind(adjustment.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(quantity)
    }

    async fn list_stock(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<StockLevel>> {
        let rows: Vec<LevelRow> = sqlx::query_as(
            "SELECT tenant_id, product_id, warehouse_id, quantity FROM stock_levels \
             WHERE tenant_id = $1 ORDER BY product_id, warehouse_id LIMIT $2 OFFSET $3",
        )
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|r| StockLevel {
                tenant_id: TenantId(r.0),
                product_id: r.1,
                warehouse_id: r.2,
                quantity: r.3,
            })
            .collect())
    }

    async fn list_adjustments(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<StockAdjustment>> {
        let rows: Vec<AdjRow> = sqlx::query_as(
            "SELECT id, tenant_id, product_id, warehouse_id, delta, reason, created_at \
             FROM stock_adjustments WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|r| StockAdjustment {
                id: r.0,
                tenant_id: TenantId(r.1),
                product_id: r.2,
                warehouse_id: r.3,
                delta: r.4,
                reason: r.5,
                created_at: r.6,
            })
            .collect())
    }
}

#[cfg(test)]
mod db_integration {
    use uuid::Uuid;

    use super::*;
    use crate::domain::inventory::entities::StockAdjustment;
    use crate::domain::inventory::ports::InventoryRepository;
    use crate::infra::db::testutil::{fake_event, pool_or_skip};

    fn adj(tenant: &TenantId, product: Uuid, delta: i64) -> StockAdjustment {
        StockAdjustment {
            id: Uuid::now_v7(),
            tenant_id: tenant.clone(),
            product_id: product,
            warehouse_id: "main".into(),
            delta,
            reason: "test".into(),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn adjust_accumulates_then_rolls_back_negative() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgInventoryRepo::new(pool.clone());
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let product = Uuid::now_v7();

        // UPSERT accumulates: +10 then +5 -> 15.
        assert_eq!(
            repo.adjust(
                &adj(&tenant, product, 10),
                &fake_event(&tenant.0, "StockAdjusted")
            )
            .await
            .unwrap(),
            10
        );
        assert_eq!(
            repo.adjust(
                &adj(&tenant, product, 5),
                &fake_event(&tenant.0, "StockAdjusted")
            )
            .await
            .unwrap(),
            15
        );

        // -100 would drive quantity negative -> rejected, and the WHOLE tx
        // (level UPSERT + adjustment row + outbox) is rolled back.
        let err = repo
            .adjust(
                &adj(&tenant, product, -100),
                &fake_event(&tenant.0, "StockAdjusted"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));

        // Level unchanged at 15; only the 2 successful adjustments persisted.
        let levels = repo.list_stock(&tenant, 10, 0).await.unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].quantity, 15);
        assert_eq!(
            repo.list_adjustments(&tenant, 10, 0).await.unwrap().len(),
            2
        );

        // And exactly 2 outbox events were committed (the rolled-back one is gone).
        let (outbox,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM outbox_events WHERE tenant_id = $1")
                .bind(&tenant.0)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(outbox, 2);
    }
}
