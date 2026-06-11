//! `PaymentRepository` backed by Postgres (`erp_core_svc.payments`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::payments::entities::Payment;
use crate::domain::payments::ports::PaymentRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

type PaymentRow = (Uuid, String, Uuid, i64, String, DateTime<Utc>);

fn to_payment(r: PaymentRow) -> Payment {
    Payment {
        id: r.0,
        tenant_id: TenantId(r.1),
        order_id: r.2,
        amount: Money(r.3),
        method: r.4,
        created_at: r.5,
    }
}

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgPaymentRepo {
    pool: PgPool,
}

impl PgPaymentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentRepository for PgPaymentRepo {
    async fn create(&self, payment: &Payment, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO payments (id, tenant_id, order_id, amount, method, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(payment.id)
        .bind(&payment.tenant_id.0)
        .bind(payment.order_id)
        .bind(payment.amount.0)
        .bind(&payment.method)
        .bind(payment.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn list_for_order(
        &self,
        tenant: &TenantId,
        order_id: &Uuid,
    ) -> DomainResult<Vec<Payment>> {
        let rows: Vec<PaymentRow> = sqlx::query_as(
            "SELECT id, tenant_id, order_id, amount, method, created_at FROM payments \
             WHERE tenant_id = $1 AND order_id = $2 ORDER BY created_at",
        )
        .bind(&tenant.0)
        .bind(order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_payment).collect())
    }
}
