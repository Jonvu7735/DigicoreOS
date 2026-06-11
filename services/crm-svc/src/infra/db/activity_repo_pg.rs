//! `ActivityRepository` backed by Postgres (`crm_svc.activities`). Plain CRUD —
//! activities have no event contract, so there is no outbox here.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::activities::entities::{Activity, ActivityKind};
use crate::domain::activities::ports::ActivityRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type ActivityRow = (
    Uuid,
    String,
    Uuid,
    String,
    String,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
);

fn to_activity(r: ActivityRow) -> DomainResult<Activity> {
    let kind = ActivityKind::parse(&r.3)
        .ok_or_else(|| DomainError::Internal(format!("unknown activity kind: {}", r.3)))?;
    Ok(Activity {
        id: r.0,
        tenant_id: TenantId(r.1),
        customer_id: r.2,
        kind,
        subject: r.4,
        notes: r.5,
        occurred_at: r.6,
        created_at: r.7,
    })
}

const COLS: &str = "id, tenant_id, customer_id, kind, subject, notes, occurred_at, created_at";

pub struct PgActivityRepo {
    pool: PgPool,
}

impl PgActivityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ActivityRepository for PgActivityRepo {
    async fn insert(&self, activity: &Activity) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO activities \
             (id, tenant_id, customer_id, kind, subject, notes, occurred_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(activity.id)
        .bind(&activity.tenant_id.0)
        .bind(activity.customer_id)
        .bind(activity.kind.as_str())
        .bind(&activity.subject)
        .bind(&activity.notes)
        .bind(activity.occurred_at)
        .bind(activity.created_at)
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
    ) -> DomainResult<Vec<Activity>> {
        let rows: Vec<ActivityRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM activities WHERE tenant_id = $1 \
             ORDER BY occurred_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_activity).collect()
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Activity>> {
        let row: Option<ActivityRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM activities WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_activity).transpose()
    }

    async fn update(&self, activity: &Activity) -> DomainResult<()> {
        sqlx::query(
            "UPDATE activities SET kind = $3, subject = $4, notes = $5 \
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&activity.tenant_id.0)
        .bind(activity.id)
        .bind(activity.kind.as_str())
        .bind(&activity.subject)
        .bind(&activity.notes)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }
}
