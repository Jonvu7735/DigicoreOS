//! `InsightRepository` backed by Postgres (`ai_svc.insights`). The insight row
//! and its outbox event commit in one transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::insights::entities::Insight;
use crate::domain::insights::ports::InsightRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type InsightRow = (Uuid, String, String, String, Option<String>, DateTime<Utc>);

fn to_insight(r: InsightRow) -> Insight {
    Insight {
        id: r.0,
        tenant_id: TenantId(r.1),
        category: r.2,
        summary: r.3,
        source_ref: r.4,
        created_at: r.5,
    }
}

const COLS: &str = "id, tenant_id, category, summary, source_ref, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgInsightRepo {
    pool: PgPool,
}

impl PgInsightRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InsightRepository for PgInsightRepo {
    async fn create(&self, insight: &Insight, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO insights (id, tenant_id, category, summary, source_ref, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(insight.id)
        .bind(&insight.tenant_id.0)
        .bind(&insight.category)
        .bind(&insight.summary)
        .bind(&insight.source_ref)
        .bind(insight.created_at)
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
    ) -> DomainResult<Vec<Insight>> {
        let rows: Vec<InsightRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM insights WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_insight).collect())
    }
}
