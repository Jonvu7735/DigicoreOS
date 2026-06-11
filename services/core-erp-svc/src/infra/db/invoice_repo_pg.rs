//! `InvoiceRepository` backed by Postgres (`erp_core_svc.invoices`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::invoices::entities::{Invoice, InvoiceStatus};
use crate::domain::invoices::ports::InvoiceRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

type InvoiceRow = (Uuid, String, Uuid, i64, String, String, DateTime<Utc>);

fn to_invoice(r: InvoiceRow) -> DomainResult<Invoice> {
    let status = InvoiceStatus::parse(&r.5)
        .ok_or_else(|| DomainError::Internal(format!("unknown invoice status: {}", r.5)))?;
    Ok(Invoice {
        id: r.0,
        tenant_id: TenantId(r.1),
        order_id: r.2,
        amount: Money(r.3),
        currency: r.4,
        status,
        created_at: r.6,
    })
}

const COLS: &str = "id, tenant_id, order_id, amount, currency, status, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgInvoiceRepo {
    pool: PgPool,
}

impl PgInvoiceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvoiceRepository for PgInvoiceRepo {
    async fn create(&self, invoice: &Invoice, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO invoices \
             (id, tenant_id, order_id, amount, currency, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(invoice.id)
        .bind(&invoice.tenant_id.0)
        .bind(invoice.order_id)
        .bind(invoice.amount.0)
        .bind(&invoice.currency)
        .bind(invoice.status.as_str())
        .bind(invoice.created_at)
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
    ) -> DomainResult<Vec<Invoice>> {
        let rows: Vec<InvoiceRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM invoices WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_invoice).collect()
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Invoice>> {
        let row: Option<InvoiceRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM invoices WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_invoice).transpose()
    }

    async fn update_status(&self, invoice: &Invoice) -> DomainResult<()> {
        sqlx::query("UPDATE invoices SET status = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(&invoice.tenant_id.0)
            .bind(invoice.id)
            .bind(invoice.status.as_str())
            .execute(&self.pool)
            .await
            .map_err(map_write_err)?;
        Ok(())
    }
}
