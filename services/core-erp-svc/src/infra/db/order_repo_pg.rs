//! `OrderRepository` backed by Postgres (`erp_core_svc.orders`). State + outbox
//! event commit in one transaction (platform_outbox::insert_outbox).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::orders::entities::{Order, OrderStatus};
use crate::domain::orders::ports::OrderRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

type OrderRow = (Uuid, String, String, i64, String, String, DateTime<Utc>);

fn to_order(r: OrderRow) -> DomainResult<Order> {
    let status = OrderStatus::parse(&r.5)
        .ok_or_else(|| DomainError::Internal(format!("unknown order status: {}", r.5)))?;
    Ok(Order {
        id: r.0,
        tenant_id: TenantId(r.1),
        customer_id: r.2,
        total_amount: Money(r.3),
        currency: r.4,
        status,
        created_at: r.6,
    })
}

const COLS: &str = "id, tenant_id, customer_id, total_amount, currency, status, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgOrderRepo {
    pool: PgPool,
}

impl PgOrderRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderRepository for PgOrderRepo {
    async fn create(&self, order: &Order, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO orders \
             (id, tenant_id, customer_id, total_amount, currency, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(order.id)
        .bind(&order.tenant_id.0)
        .bind(&order.customer_id)
        .bind(order.total_amount.0)
        .bind(&order.currency)
        .bind(order.status.as_str())
        .bind(order.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Order>> {
        let rows: Vec<OrderRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM orders WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_order).collect()
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Order>> {
        let row: Option<OrderRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM orders WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_order).transpose()
    }

    async fn save_status(&self, order: &Order, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query("UPDATE orders SET status = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(&order.tenant_id.0)
            .bind(order.id)
            .bind(order.status.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod db_integration {
    use uuid::Uuid;

    use super::*;
    use crate::domain::orders::entities::{Order, OrderStatus};
    use crate::domain::orders::ports::OrderRepository;
    use crate::domain::shared::types::Money;
    use crate::infra::db::testutil::{fake_event, pool_or_skip};

    #[tokio::test]
    async fn create_then_status_change_persists_with_outbox() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgOrderRepo::new(pool.clone());
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let mut order = Order {
            id: Uuid::now_v7(),
            tenant_id: tenant.clone(),
            customer_id: "cust-1".into(),
            total_amount: Money(5000),
            currency: "USD".into(),
            status: OrderStatus::New,
            created_at: Utc::now(),
        };

        // create + a status transition, each writing state + an outbox event in
        // one transaction.
        repo.create(&order, &fake_event(&tenant.0, "OrderCreated"))
            .await
            .unwrap();
        order.status = OrderStatus::Confirmed;
        repo.save_status(&order, &fake_event(&tenant.0, "OrderStatusChanged"))
            .await
            .unwrap();

        let fetched = repo
            .find_in_tenant(&tenant, &order.id)
            .await
            .unwrap()
            .expect("order persisted");
        assert_eq!(fetched.status, OrderStatus::Confirmed);
        assert_eq!(fetched.total_amount.0, 5000);
        assert_eq!(fetched.currency, "USD");

        let (outbox,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM outbox_events WHERE tenant_id = $1")
                .bind(&tenant.0)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(outbox, 2);
    }
}
