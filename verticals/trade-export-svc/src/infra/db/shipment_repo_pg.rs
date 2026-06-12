//! `ShipmentRepository` backed by Postgres (`trade_export_svc.export_shipments`).
//! The status change writes state + an outbox event in one transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::domain::shipments::entities::{ExportShipment, ShipmentStatus};
use crate::domain::shipments::ports::ShipmentRepository;
use crate::infra::db::{map_db_err, map_write_err};

type ShipmentRow = (
    Uuid,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    DateTime<Utc>,
);

fn to_shipment(r: ShipmentRow) -> DomainResult<ExportShipment> {
    let status = ShipmentStatus::parse(&r.6)
        .ok_or_else(|| DomainError::Internal(format!("unknown shipment status: {}", r.6)))?;
    Ok(ExportShipment {
        id: r.0,
        tenant_id: TenantId(r.1),
        order_id: r.2,
        reference: r.3,
        destination_country: r.4,
        incoterm: r.5,
        status,
        created_at: r.7,
    })
}

const COLS: &str =
    "id, tenant_id, order_id, reference, destination_country, incoterm, status, created_at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgShipmentRepo {
    pool: PgPool,
}

impl PgShipmentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShipmentRepository for PgShipmentRepo {
    async fn insert(&self, shipment: &ExportShipment) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO export_shipments \
             (id, tenant_id, order_id, reference, destination_country, incoterm, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(shipment.id)
        .bind(&shipment.tenant_id.0)
        .bind(&shipment.order_id)
        .bind(&shipment.reference)
        .bind(&shipment.destination_country)
        .bind(&shipment.incoterm)
        .bind(shipment.status.as_str())
        .bind(shipment.created_at)
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
    ) -> DomainResult<Vec<ExportShipment>> {
        let rows: Vec<ShipmentRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM export_shipments WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_shipment).collect()
    }

    async fn find_in_tenant(
        &self,
        tenant: &TenantId,
        id: &Uuid,
    ) -> DomainResult<Option<ExportShipment>> {
        let row: Option<ShipmentRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM export_shipments WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_shipment).transpose()
    }

    async fn find_by_order(
        &self,
        tenant: &TenantId,
        order_id: &str,
    ) -> DomainResult<Option<ExportShipment>> {
        let row: Option<ShipmentRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM export_shipments WHERE tenant_id = $1 AND order_id = $2 LIMIT 1"
        ))
        .bind(&tenant.0)
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.map(to_shipment).transpose()
    }

    async fn save_status(
        &self,
        shipment: &ExportShipment,
        event: &OutboxMessage,
    ) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query("UPDATE export_shipments SET status = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(&shipment.tenant_id.0)
            .bind(shipment.id)
            .bind(shipment.status.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}
