//! Repository port for export shipments. The status-changing mutation also
//! enqueues an event into the outbox in the SAME transaction (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::domain::shipments::entities::ExportShipment;

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
