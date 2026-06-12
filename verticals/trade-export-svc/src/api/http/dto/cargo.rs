//! Cargo-line DTOs (`/api/v1/trade-export/shipments/{shipment_id}/cargo`).

use serde::{Deserialize, Serialize};

use crate::domain::shipments::entities::CargoLine;
use crate::domain::shipments::services::NewCargoLine;

#[derive(Debug, Deserialize)]
pub struct AddCargoLineRequest {
    /// What the goods are.
    pub description: String,
    /// Harmonized System tariff code (6-10 digits), optional.
    #[serde(default)]
    pub hs_code: Option<String>,
    /// Number of units; must be > 0.
    pub quantity: i64,
    /// Unit of measure, e.g. `CTN`, `PCS`, `PLT`.
    pub unit: String,
    #[serde(default)]
    pub net_weight_kg: Option<f64>,
}

impl From<AddCargoLineRequest> for NewCargoLine {
    fn from(r: AddCargoLineRequest) -> Self {
        Self {
            description: r.description,
            hs_code: r.hs_code,
            quantity: r.quantity,
            unit: r.unit,
            net_weight_kg: r.net_weight_kg,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CargoLineResponse {
    pub id: String,
    pub shipment_id: String,
    pub description: String,
    pub hs_code: Option<String>,
    pub quantity: i64,
    pub unit: String,
    pub net_weight_kg: Option<f64>,
    pub created_at: String,
}

impl From<CargoLine> for CargoLineResponse {
    fn from(l: CargoLine) -> Self {
        Self {
            id: l.id.to_string(),
            shipment_id: l.shipment_id.to_string(),
            description: l.description,
            hs_code: l.hs_code,
            quantity: l.quantity,
            unit: l.unit,
            net_weight_kg: l.net_weight_kg,
            created_at: l.created_at.to_rfc3339(),
        }
    }
}
