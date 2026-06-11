//! `ContactRepository` backed by Postgres (`crm_svc.contacts`). Plain CRUD —
//! contacts have no event contract, so there is no outbox here.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::contacts::entities::Contact;
use crate::domain::contacts::ports::ContactRepository;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type ContactRow = (
    Uuid,
    String,
    Uuid,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    DateTime<Utc>,
);

fn to_contact(r: ContactRow) -> Contact {
    Contact {
        id: r.0,
        tenant_id: TenantId(r.1),
        customer_id: r.2,
        name: r.3,
        email: r.4,
        phone: r.5,
        title: r.6,
        created_at: r.7,
    }
}

const COLS: &str = "id, tenant_id, customer_id, name, email, phone, title, created_at";

pub struct PgContactRepo {
    pool: PgPool,
}

impl PgContactRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ContactRepository for PgContactRepo {
    async fn insert(&self, contact: &Contact) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO contacts (id, tenant_id, customer_id, name, email, phone, title, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(contact.id)
        .bind(&contact.tenant_id.0)
        .bind(contact.customer_id)
        .bind(&contact.name)
        .bind(&contact.email)
        .bind(&contact.phone)
        .bind(&contact.title)
        .bind(contact.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Contact>> {
        let rows: Vec<ContactRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM contacts WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_contact).collect())
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Contact>> {
        let row: Option<ContactRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM contacts WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_contact))
    }

    async fn update(&self, contact: &Contact) -> DomainResult<()> {
        sqlx::query(
            "UPDATE contacts SET name = $3, email = $4, phone = $5, title = $6 \
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&contact.tenant_id.0)
        .bind(contact.id)
        .bind(&contact.name)
        .bind(&contact.email)
        .bind(&contact.phone)
        .bind(&contact.title)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }
}
