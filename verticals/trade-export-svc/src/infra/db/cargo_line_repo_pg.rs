//! `CargoLineRepository` backed by Postgres (`trade_export_svc.export_cargo_lines`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::domain::shipments::entities::CargoLine;
use crate::domain::shipments::ports::CargoLineRepository;
use crate::infra::db::{map_db_err, map_write_err};

type CargoRow = (
    Uuid,
    Uuid,
    String,
    String,
    Option<String>,
    i64,
    String,
    Option<f64>,
    DateTime<Utc>,
);

fn to_line(r: CargoRow) -> CargoLine {
    CargoLine {
        id: r.0,
        shipment_id: r.1,
        tenant_id: TenantId(r.2),
        description: r.3,
        hs_code: r.4,
        quantity: r.5,
        unit: r.6,
        net_weight_kg: r.7,
        created_at: r.8,
    }
}

const COLS: &str =
    "id, shipment_id, tenant_id, description, hs_code, quantity, unit, net_weight_kg, created_at";

pub struct PgCargoLineRepo {
    pool: PgPool,
}

impl PgCargoLineRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CargoLineRepository for PgCargoLineRepo {
    async fn insert(&self, line: &CargoLine) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO export_cargo_lines \
             (id, shipment_id, tenant_id, description, hs_code, quantity, unit, net_weight_kg, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(line.id)
        .bind(line.shipment_id)
        .bind(&line.tenant_id.0)
        .bind(&line.description)
        .bind(&line.hs_code)
        .bind(line.quantity)
        .bind(&line.unit)
        .bind(line.net_weight_kg)
        .bind(line.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn list_for_shipment(
        &self,
        tenant: &TenantId,
        shipment_id: &Uuid,
    ) -> DomainResult<Vec<CargoLine>> {
        let rows: Vec<CargoRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM export_cargo_lines \
             WHERE tenant_id = $1 AND shipment_id = $2 ORDER BY created_at ASC"
        ))
        .bind(&tenant.0)
        .bind(shipment_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_line).collect())
    }
}
