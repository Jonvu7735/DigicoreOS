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

    /// Cargo may only be edited before the goods leave: while DRAFT or BOOKED.
    /// Once DISPATCHED (gone) or CANCELLED (void) the packing list is frozen.
    pub fn accepts_cargo_changes(self) -> bool {
        matches!(self, ShipmentStatus::Draft | ShipmentStatus::Booked)
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

/// A line of cargo on a shipment — one row of the packing list / commercial
/// invoice: what the goods are, how many, and how heavy. Child of
/// [`ExportShipment`]; deleted with its parent.
#[derive(Debug, Clone)]
pub struct CargoLine {
    pub id: Uuid,
    pub shipment_id: Uuid,
    pub tenant_id: TenantId,
    pub description: String,
    /// Harmonized System tariff code (6–10 digits), when the goods are classified.
    pub hs_code: Option<String>,
    pub quantity: i64,
    /// Unit of measure, e.g. `CTN`, `PCS`, `PLT`, `KG`.
    pub unit: String,
    pub net_weight_kg: Option<f64>,
    pub created_at: DateTime<Utc>,
}
