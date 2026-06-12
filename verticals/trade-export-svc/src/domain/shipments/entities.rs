//! Export-shipment entity + status machine (maps to `trade_export_svc.export_shipments`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// Shipment lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipmentStatus {
    /// Created (manually or auto-drafted from a paid order), not yet booked.
    Draft,
    /// Booked with a carrier — emits `ShipmentBooked`.
    Booked,
    /// Handed to the carrier / left the warehouse.
    Dispatched,
    Cancelled,
}

impl ShipmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ShipmentStatus::Draft => "DRAFT",
            ShipmentStatus::Booked => "BOOKED",
            ShipmentStatus::Dispatched => "DISPATCHED",
            ShipmentStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "DRAFT" => Some(ShipmentStatus::Draft),
            "BOOKED" => Some(ShipmentStatus::Booked),
            "DISPATCHED" => Some(ShipmentStatus::Dispatched),
            "CANCELLED" => Some(ShipmentStatus::Cancelled),
            _ => None,
        }
    }

    /// Allowed transitions: DRAFT→BOOKED→DISPATCHED; DRAFT/BOOKED→CANCELLED.
    pub fn can_transition_to(self, next: ShipmentStatus) -> bool {
        use ShipmentStatus::*;
        matches!(
            (self, next),
            (Draft, Booked) | (Draft, Cancelled) | (Booked, Dispatched) | (Booked, Cancelled)
        )
    }
}

/// An export shipment: the vertical's aggregate root.
#[derive(Debug, Clone)]
pub struct ExportShipment {
    pub id: Uuid,
    pub tenant_id: TenantId,
    /// The ERP order this shipment fulfils, when auto-drafted from `order.paid`.
    pub order_id: Option<String>,
    /// Human-friendly reference code, e.g. `EXP-1A2B3C4D`.
    pub reference: String,
    /// ISO-3166 alpha-2 destination country (may be blank on an auto-draft).
    pub destination_country: String,
    /// Incoterm, e.g. `FOB`, `CIF` (may be blank on an auto-draft).
    pub incoterm: String,
    pub status: ShipmentStatus,
    pub created_at: DateTime<Utc>,
}

impl ExportShipment {
    /// Derive a reference code from the shipment id (first 8 hex chars).
    pub fn reference_for(id: &Uuid) -> String {
        format!("EXP-{}", id.simple().to_string()[..8].to_uppercase())
    }
}
