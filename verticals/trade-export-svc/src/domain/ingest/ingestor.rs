//! `ShipmentIngestor`: the vertical's inbound projection. It subscribes (via
//! `platform-events`) to `platform.>` and reacts to core ERP events — when an
//! order is PAID it drafts an export shipment. This is how the vertical consumes
//! the core: business events only, never the core's code or schema.

use std::sync::Arc;

use async_trait::async_trait;
use event_models::erp::{subjects, OrderPaid};
use platform_events::{HandlerError, HandlerResult, InboundEventHandler};

use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::domain::shipments::entities::ExportShipment;
use crate::domain::shipments::services::ShipmentService;

/// The narrow slice of shipment behaviour the consumer needs (kept as a port so
/// the ingestor can be unit-tested with a fake).
#[async_trait]
pub trait OrderShipmentDrafter: Send + Sync {
    async fn draft_from_order(
        &self,
        tenant: &TenantId,
        order_id: &str,
    ) -> DomainResult<Option<ExportShipment>>;
}

#[async_trait]
impl OrderShipmentDrafter for ShipmentService {
    async fn draft_from_order(
        &self,
        tenant: &TenantId,
        order_id: &str,
    ) -> DomainResult<Option<ExportShipment>> {
        // Disambiguate to the inherent method of the same name.
        ShipmentService::draft_from_order(self, tenant, order_id).await
    }
}

pub struct ShipmentIngestor {
    drafter: Arc<dyn OrderShipmentDrafter>,
}

impl ShipmentIngestor {
    pub fn new(drafter: Arc<dyn OrderShipmentDrafter>) -> Self {
        Self { drafter }
    }
}

#[async_trait]
impl InboundEventHandler for ShipmentIngestor {
    async fn handle(&self, subject: &str, payload: &[u8]) -> HandlerResult<()> {
        if subject == subjects::ORDER_PAID {
            let event: OrderPaid = serde_json::from_slice(payload)
                .map_err(|e| HandlerError(format!("malformed OrderPaid payload: {e}")))?;
            self.drafter
                .draft_from_order(&TenantId(event.header.tenant_id), &event.order_id)
                .await
                .map_err(|e| HandlerError(e.to_string()))?;
        }
        // Other subjects aren't acted on; ignored so the consumer keeps draining.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;
    use event_models::erp::{ErpEvent, OrderPaid};
    use event_models::EventHeader;
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct FakeDrafter {
        drafted: Mutex<Vec<(String, String)>>,
    }
    #[async_trait]
    impl OrderShipmentDrafter for FakeDrafter {
        async fn draft_from_order(
            &self,
            tenant: &TenantId,
            order_id: &str,
        ) -> DomainResult<Option<ExportShipment>> {
            self.drafted
                .lock()
                .unwrap()
                .push((tenant.0.clone(), order_id.to_string()));
            Ok(None)
        }
    }

    fn order_paid_bytes(tenant: &str, order_id: &str) -> Vec<u8> {
        let header = EventHeader::new(
            Uuid::now_v7(),
            Utc::now(),
            tenant.to_string(),
            "order",
            order_id.to_string(),
            "OrderPaid",
            1,
        );
        let event = ErpEvent::OrderPaid(OrderPaid {
            header,
            order_id: order_id.to_string(),
            amount_paid: 1000,
            payment_method: "card".into(),
        });
        event.payload_json().unwrap()
    }

    #[tokio::test]
    async fn drafts_shipment_on_order_paid() {
        let drafter = Arc::new(FakeDrafter::default());
        let ingestor = ShipmentIngestor::new(drafter.clone());

        ingestor
            .handle(subjects::ORDER_PAID, &order_paid_bytes("t1", "order-9"))
            .await
            .unwrap();

        let drafted = drafter.drafted.lock().unwrap();
        assert_eq!(drafted.len(), 1);
        assert_eq!(drafted[0], ("t1".to_string(), "order-9".to_string()));
    }

    #[tokio::test]
    async fn ignores_other_subjects() {
        let drafter = Arc::new(FakeDrafter::default());
        let ingestor = ShipmentIngestor::new(drafter.clone());

        ingestor
            .handle("platform.erp.order.created", &order_paid_bytes("t1", "o"))
            .await
            .unwrap();

        assert!(drafter.drafted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_malformed_payload() {
        let drafter = Arc::new(FakeDrafter::default());
        let ingestor = ShipmentIngestor::new(drafter);

        let err = ingestor
            .handle(subjects::ORDER_PAID, b"not json")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("malformed OrderPaid"));
    }
}
