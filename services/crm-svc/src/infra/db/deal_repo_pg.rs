//! `DealRepository` backed by Postgres (`crm_svc.deals`). State + outbox event
//! commit in one transaction (platform_outbox::insert_outbox).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::deals::entities::{Deal, DealStage};
use crate::domain::deals::ports::DealRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

type DealRow = (Uuid, String, Uuid, String, i64, String, DateTime<Utc>);

fn to_deal(r: DealRow) -> DomainResult<Deal> {
    let stage = DealStage::parse(&r.5)
        .ok_or_else(|| DomainError::Internal(format!("unknown deal stage: {}", r.5)))?;
    Ok(Deal {
        id: r.0,
        tenant_id: TenantId(r.1),
        customer_id: r.2,
        title: r.3,
        amount_estimate: Money(r.4),
        stage,
        created_at: r.6,
    })
}

const COLS: &str = "id, tenant_id, customer_id, title, amount_estimate, stage, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgDealRepo {
    pool: PgPool,
}

impl PgDealRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DealRepository for PgDealRepo {
    async fn create(&self, deal: &Deal, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO deals \
             (id, tenant_id, customer_id, title, amount_estimate, stage, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(deal.id)
        .bind(&deal.tenant_id.0)
        .bind(deal.customer_id)
        .bind(&deal.title)
        .bind(deal.amount_estimate.0)
        .bind(deal.stage.as_str())
        .bind(deal.created_at)
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
    ) -> DomainResult<Vec<Deal>> {
        let rows: Vec<DealRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deals WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_deal).collect()
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Deal>> {
        let row: Option<DealRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deals WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_deal).transpose()
    }

    async fn save_stage(&self, deal: &Deal, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query("UPDATE deals SET stage = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(&deal.tenant_id.0)
            .bind(deal.id)
            .bind(deal.stage.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}
