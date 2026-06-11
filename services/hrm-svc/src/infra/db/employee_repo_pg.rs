//! `EmployeeRepository` backed by Postgres (`hrm_svc.employees`). State + outbox
//! event commit in one transaction (platform_outbox::insert_outbox).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::employees::entities::{Employee, EmploymentStatus};
use crate::domain::employees::ports::EmployeeRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type EmployeeRow = (
    Uuid,
    String,
    String,
    String,
    Option<String>,
    String,
    DateTime<Utc>,
);

fn to_employee(r: EmployeeRow) -> DomainResult<Employee> {
    let status = EmploymentStatus::parse(&r.5)
        .ok_or_else(|| DomainError::Internal(format!("unknown employment status: {}", r.5)))?;
    Ok(Employee {
        id: r.0,
        tenant_id: TenantId(r.1),
        full_name: r.2,
        position: r.3,
        email: r.4,
        status,
        created_at: r.6,
    })
}

const COLS: &str = "id, tenant_id, full_name, position, email, status, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgEmployeeRepo {
    pool: PgPool,
}

impl PgEmployeeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmployeeRepository for PgEmployeeRepo {
    async fn create(&self, employee: &Employee, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "INSERT INTO employees \
             (id, tenant_id, full_name, position, email, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(employee.id)
        .bind(&employee.tenant_id.0)
        .bind(&employee.full_name)
        .bind(&employee.position)
        .bind(&employee.email)
        .bind(employee.status.as_str())
        .bind(employee.created_at)
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
    ) -> DomainResult<Vec<Employee>> {
        let rows: Vec<EmployeeRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM employees WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_employee).collect()
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Employee>> {
        let row: Option<EmployeeRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM employees WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_employee).transpose()
    }

    async fn save_status(&self, employee: &Employee, event: &OutboxMessage) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query("UPDATE employees SET status = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(&employee.tenant_id.0)
            .bind(employee.id)
            .bind(employee.status.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}
