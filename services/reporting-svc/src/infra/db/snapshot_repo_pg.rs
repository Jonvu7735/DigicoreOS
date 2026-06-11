//! `SnapshotRepository` backed by Postgres (`reporting_svc.snapshots`). The
//! snapshot row and its outbox event commit in one transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::domain::snapshots::entities::Snapshot;
use crate::domain::snapshots::ports::SnapshotRepository;
use crate::infra::db::{map_db_err, map_write_err};

type SnapshotRow = (Uuid, String, String, serde_json::Value, DateTime<Utc>);

fn to_snapshot(r: SnapshotRow) -> Snapshot {
    Snapshot {
        id: r.0,
        tenant_id: TenantId(r.1),
        snapshot_type: r.2,
        payload: r.3,
        created_at: r.4,
    }
}

const COLS: &str = "id, tenant_id, snapshot_type, payload, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgSnapshotRepo {
    pool: PgPool,
}

impl PgSnapshotRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SnapshotRepository for PgSnapshotRepo {
    async fn create(&self, snapshot: &Snapshot, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO snapshots (id, tenant_id, snapshot_type, payload, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(snapshot.id)
        .bind(&snapshot.tenant_id.0)
        .bind(&snapshot.snapshot_type)
        .bind(&snapshot.payload)
        .bind(snapshot.created_at)
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
    ) -> DomainResult<Vec<Snapshot>> {
        let rows: Vec<SnapshotRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM snapshots WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_snapshot).collect())
    }
}
