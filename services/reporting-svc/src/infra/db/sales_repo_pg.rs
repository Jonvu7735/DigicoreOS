//! `SalesProjection` backed by Postgres (`reporting_svc.sales_summary`).
//!
//! Idempotency: each `OrderPaid` is recorded in `processed_events` inside the
//! same transaction as the rollup update, so an at-least-once re-delivery is a
//! no-op.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::sales::entities::SalesSummary;
use crate::domain::sales::ports::SalesProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

pub struct PgSalesRepo {
    pool: PgPool,
}

impl PgSalesRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SalesProjection for PgSalesRepo {
    async fn apply_order_paid(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        amount_paid: i64,
    ) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        // Claim the event_id; if already present this is a re-delivery -> skip.
        let claimed = sqlx::query(
            "INSERT INTO processed_events (event_id, subject) VALUES ($1, $2) \
             ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(event_id)
        .bind("platform.erp.order.paid")
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?
        .rows_affected();

        if claimed == 0 {
            // Already applied; commit the (empty) tx and return.
            tx.commit().await.map_err(map_db_err)?;
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO sales_summary (tenant_id, total_paid, payment_count, updated_at) \
             VALUES ($1, $2, 1, now()) \
             ON CONFLICT (tenant_id) DO UPDATE SET \
                total_paid = sales_summary.total_paid + EXCLUDED.total_paid, \
                payment_count = sales_summary.payment_count + 1, \
                updated_at = now()",
        )
        .bind(&tenant.0)
        .bind(amount_paid)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn get_summary(&self, tenant: &TenantId) -> DomainResult<SalesSummary> {
        let row: Option<(i64, i64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT total_paid, payment_count, updated_at FROM sales_summary WHERE tenant_id = $1",
        )
        .bind(&tenant.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        Ok(match row {
            Some((total, count, updated)) => SalesSummary {
                tenant_id: tenant.clone(),
                total_paid: Money(total),
                payment_count: count,
                updated_at: Some(updated),
            },
            None => SalesSummary {
                tenant_id: tenant.clone(),
                total_paid: Money(0),
                payment_count: 0,
                updated_at: None,
            },
        })
    }
}
