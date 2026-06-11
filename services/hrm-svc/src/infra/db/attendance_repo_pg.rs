//! `AttendanceRepository` backed by Postgres (`hrm_svc.attendance`). State +
//! outbox event commit in one transaction (platform_outbox::insert_outbox).

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::attendance::entities::AttendanceRecord;
use crate::domain::attendance::ports::AttendanceRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type AttendanceRow = (
    Uuid,
    String,
    Uuid,
    NaiveDate,
    Option<NaiveTime>,
    Option<NaiveTime>,
    DateTime<Utc>,
);

fn to_record(r: AttendanceRow) -> AttendanceRecord {
    AttendanceRecord {
        id: r.0,
        tenant_id: TenantId(r.1),
        employee_id: r.2,
        date: r.3,
        check_in: r.4,
        check_out: r.5,
        created_at: r.6,
    }
}

const COLS: &str = "id, tenant_id, employee_id, date, check_in, check_out, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgAttendanceRepo {
    pool: PgPool,
}

impl PgAttendanceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttendanceRepository for PgAttendanceRepo {
    async fn create(&self, record: &AttendanceRecord, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO attendance \
             (id, tenant_id, employee_id, date, check_in, check_out, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(record.id)
        .bind(&record.tenant_id.0)
        .bind(record.employee_id)
        .bind(record.date)
        .bind(record.check_in)
        .bind(record.check_out)
        .bind(record.created_at)
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
    ) -> DomainResult<Vec<AttendanceRecord>> {
        let rows: Vec<AttendanceRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM attendance WHERE tenant_id = $1 \
             ORDER BY date DESC, created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_record).collect())
    }

    async fn find_in_tenant(
        &self,
        tenant: &TenantId,
        id: &Uuid,
    ) -> DomainResult<Option<AttendanceRecord>> {
        let row: Option<AttendanceRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM attendance WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_record))
    }
}
