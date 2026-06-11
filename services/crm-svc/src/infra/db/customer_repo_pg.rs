//! `CustomerRepository` backed by Postgres (`crm_svc.customers`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::customers::entities::Customer;
use crate::domain::customers::ports::CustomerRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type CustomerRow = (
    Uuid,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

fn to_customer(r: CustomerRow) -> Customer {
    Customer {
        id: r.0,
        tenant_id: TenantId(r.1),
        name: r.2,
        email: r.3,
        phone: r.4,
        segment: r.5,
        created_at: r.6,
    }
}

const COLS: &str = "id, tenant_id, name, email, phone, segment, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgCustomerRepo {
    pool: PgPool,
}

impl PgCustomerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerRepository for PgCustomerRepo {
    async fn create(&self, customer: &Customer, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO customers (id, tenant_id, name, email, phone, segment, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(customer.id)
        .bind(&customer.tenant_id.0)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.segment)
        .bind(customer.created_at)
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
    ) -> DomainResult<Vec<Customer>> {
        let rows: Vec<CustomerRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM customers WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_customer).collect())
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Customer>> {
        let row: Option<CustomerRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM customers WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_customer))
    }

    async fn update(&self, customer: &Customer, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "UPDATE customers SET name = $3, email = $4, phone = $5, segment = $6 \
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&customer.tenant_id.0)
        .bind(customer.id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.segment)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}
