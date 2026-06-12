//! Repository port for export shipments. The status-changing mutation also
//! enqueues an event into the outbox in the SAME transaction (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::domain::shipments::entities::{CargoLine, ExportShipment};

#[async_trait]
pub trait ShipmentRepository: Send + Sync {
    /// Insert a new shipment (no event — `Draft` is an internal state).
    async fn insert(&self, shipment: &ExportShipment) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ExportShipment>>;
    async fn find_in_tenant(
        &self,
        tenant: &TenantId,
        id: &Uuid,
    ) -> DomainResult<Option<ExportShipment>>;
    /// The shipment already fulfilling `order_id` in this tenant, if any — lets the
    /// inbound consumer draft shipments idempotently (at-least-once delivery).
    async fn find_by_order(
        &self,
        tenant: &TenantId,
        order_id: &str,
    ) -> DomainResult<Option<ExportShipment>>;
    /// Persist the shipment's new status and enqueue `event`, in one transaction.
    async fn save_status(
        &self,
        shipment: &ExportShipment,
        event: &OutboxMessage,
    ) -> DomainResult<()>;
}

/// Repository for a shipment's cargo lines (the packing-list rows).
#[async_trait]
pub trait CargoLineRepository: Send + Sync {
    async fn insert(&self, line: &CargoLine) -> DomainResult<()>;
    /// Lines on `shipment_id` within `tenant`, oldest first.
    async fn list_for_shipment(
        &self,
        tenant: &TenantId,
        shipment_id: &Uuid,
    ) -> DomainResult<Vec<CargoLine>>;
}
