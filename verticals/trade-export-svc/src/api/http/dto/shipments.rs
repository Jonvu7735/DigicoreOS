//! Shipment DTOs (`/api/v1/trade-export/shipments`).

use serde::{Deserialize, Serialize};

use crate::domain::shipments::entities::ExportShipment;

#[derive(Debug, Deserialize)]
pub struct CreateShipmentRequest {
    /// ISO-3166 alpha-2 destination country.
    pub destination_country: String,
    /// Incoterm, e.g. `FOB`, `CIF`.
    pub incoterm: String,
    /// Optional ERP order this shipment fulfils.
    #[serde(default)]
    pub order_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShipmentResponse {
    pub id: String,
    pub tenant_id: String,
    pub order_id: Option<String>,
    pub reference: String,
    pub destination_country: String,
    pub incoterm: String,
    pub status: String,
    pub created_at: String,
}

impl From<ExportShipment> for ShipmentResponse {
    fn from(s: ExportShipment) -> Self {
        Self {
            id: s.id.to_string(),
            tenant_id: s.tenant_id.0,
            order_id: s.order_id,
            reference: s.reference,
            destination_country: s.destination_country,
            incoterm: s.incoterm,
            status: s.status.as_str().to_string(),
            created_at: s.created_at.to_rfc3339(),
        }
    }
}
