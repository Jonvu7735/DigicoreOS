//! `OutboxRepository` backed by Postgres (`auth_svc.outbox_events`). Read side
//! for the relay worker (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::identity::outbox::OutboxMessage;
use crate::domain::identity::ports::OutboxRepository;
use crate::domain::shared::error::DomainResult;
use crate::infra::db::map_db_err;

#[derive(sqlx::FromRow)]
struct OutboxRow {
    id: Uuid,
    occurred_at: DateTime<Utc>,
    tenant_id: String,
    aggregate_type: String,
    aggregate_id: String,
    event_type: String,
    version: i32,
    subject: String,
    payload: serde_json::Value,
}

impl From<OutboxRow> for OutboxMessage {
    fn from(r: OutboxRow) -> Self {
        OutboxMessage {
            event_id: r.id,
            occurred_at: r.occurred_at,
            tenant_id: r.tenant_id,
            aggregate_type: r.aggregate_type,
            aggregate_id: r.aggregate_id,
            event_type: r.event_type,
            version: r.version,
            subject: r.subject,
            payload: r.payload,
        }
    }
}

pub struct PgOutboxRepo {
    pool: PgPool,
}

impl PgOutboxRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboxRepository for PgOutboxRepo {
    async fn fetch_unpublished(&self, limit: i64) -> DomainResult<Vec<OutboxMessage>> {
        let rows: Vec<OutboxRow> = sqlx::query_as(
            "SELECT id, occurred_at, tenant_id, aggregate_type, aggregate_id, event_type, \
             version, subject, payload FROM outbox_events \
             WHERE published_at IS NULL ORDER BY created_at LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(OutboxMessage::from).collect())
    }

    async fn mark_published(&self, event_id: &Uuid) -> DomainResult<()> {
        sqlx::query(
            "UPDATE outbox_events SET published_at = now() WHERE id = $1 AND published_at IS NULL",
        )
        .bind(event_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }
}
