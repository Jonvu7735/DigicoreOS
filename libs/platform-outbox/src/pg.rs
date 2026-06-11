//! Postgres outbox: the write helper (`insert_outbox`) for the service's
//! transaction, and `PgOutboxRepo` (read/clear) for the relay.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::message::{OutboxError, OutboxMessage, OutboxResult};
use crate::ports::OutboxRepository;

fn storage(e: sqlx::Error) -> OutboxError {
    OutboxError::Storage(e.to_string())
}

/// Insert one outbox row on the given (transaction) connection — call inside the
/// service's state-changing transaction so state + event commit together.
pub async fn insert_outbox(conn: &mut sqlx::PgConnection, msg: &OutboxMessage) -> OutboxResult<()> {
    sqlx::query(
        "INSERT INTO outbox_events \
         (id, occurred_at, tenant_id, aggregate_type, aggregate_id, event_type, version, subject, payload) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(msg.event_id)
    .bind(msg.occurred_at)
    .bind(&msg.tenant_id)
    .bind(&msg.aggregate_type)
    .bind(&msg.aggregate_id)
    .bind(&msg.event_type)
    .bind(msg.version)
    .bind(&msg.subject)
    .bind(&msg.payload)
    .execute(conn)
    .await
    .map_err(storage)?;
    Ok(())
}

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
    async fn fetch_unpublished(&self, limit: i64) -> OutboxResult<Vec<OutboxMessage>> {
        let rows: Vec<OutboxRow> = sqlx::query_as(
            "SELECT id, occurred_at, tenant_id, aggregate_type, aggregate_id, event_type, \
             version, subject, payload FROM outbox_events \
             WHERE published_at IS NULL ORDER BY created_at LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?;
        Ok(rows.into_iter().map(OutboxMessage::from).collect())
    }

    async fn mark_published(&self, event_id: &Uuid) -> OutboxResult<()> {
        sqlx::query(
            "UPDATE outbox_events SET published_at = now() WHERE id = $1 AND published_at IS NULL",
        )
        .bind(event_id)
        .execute(&self.pool)
        .await
        .map_err(storage)?;
        Ok(())
    }
}
