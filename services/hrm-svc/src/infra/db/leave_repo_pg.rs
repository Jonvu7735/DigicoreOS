//! `LeaveRepository` backed by Postgres (`hrm_svc.leave_requests`). Plain CRUD —
//! leave has no event contract, so there is no outbox here.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::leave::entities::{LeaveRequest, LeaveStatus};
use crate::domain::leave::ports::LeaveRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type LeaveRow = (
    Uuid,
    String,
    Uuid,
    NaiveDate,
    NaiveDate,
    Option<String>,
    String,
    DateTime<Utc>,
);

fn to_request(r: LeaveRow) -> DomainResult<LeaveRequest> {
    let status = LeaveStatus::parse(&r.6)
        .ok_or_else(|| DomainError::Internal(format!("unknown leave status: {}", r.6)))?;
    Ok(LeaveRequest {
        id: r.0,
        tenant_id: TenantId(r.1),
        employee_id: r.2,
        start_date: r.3,
        end_date: r.4,
        reason: r.5,
        status,
        created_at: r.7,
    })
}

const COLS: &str = "id, tenant_id, employee_id, start_date, end_date, reason, status, created_at";

pub struct PgLeaveRepo {
    pool: PgPool,
}

impl PgLeaveRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LeaveRepository for PgLeaveRepo {
    async fn insert(&self, request: &LeaveRequest) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO leave_requests \
             (id, tenant_id, employee_id, start_date, end_date, reason, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(request.id)
        .bind(&request.tenant_id.0)
        .bind(request.employee_id)
        .bind(request.start_date)
        .bind(request.end_date)
        .bind(&request.reason)
        .bind(request.status.as_str())
        .bind(request.created_at)
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
    ) -> DomainResult<Vec<LeaveRequest>> {
        let rows: Vec<LeaveRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM leave_requests WHERE tenant_id = $1 \
             ORDER BY start_date DESC, created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_request).collect()
    }

    async fn find_in_tenant(
        &self,
        tenant: &TenantId,
        id: &Uuid,
    ) -> DomainResult<Option<LeaveRequest>> {
        let row: Option<LeaveRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM leave_requests WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_request).transpose()
    }

    async fn update_status(&self, request: &LeaveRequest) -> DomainResult<()> {
        sqlx::query("UPDATE leave_requests SET status = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(&request.tenant_id.0)
            .bind(request.id)
            .bind(request.status.as_str())
            .execute(&self.pool)
            .await
            .map_err(map_write_err)?;
        Ok(())
    }
}
