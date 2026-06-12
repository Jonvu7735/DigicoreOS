//! `LoyaltyIngestor`: the vertical's inbound projection. It subscribes (via
//! `platform-events`) to `platform.>` and reacts to core ERP events — when an
//! order is CREATED it accrues loyalty points for the customer. The vertical
//! consumes the core through business events only, never its code or schema.

use std::sync::Arc;

use async_trait::async_trait;
use event_models::erp::{subjects, OrderCreated};
use platform_events::{HandlerError, HandlerResult, InboundEventHandler};
use uuid::Uuid;

use crate::domain::loyalty::services::LoyaltyService;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// The narrow slice of loyalty behaviour the consumer needs (a port so the
/// ingestor can be unit-tested with a fake).
#[async_trait]
pub trait OrderPointsAccruer: Send + Sync {
    async fn accrue_for_order(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        customer_id: &str,
        order_id: &str,
        total_amount_minor: i64,
    ) -> DomainResult<bool>;
}

#[async_trait]
impl OrderPointsAccruer for LoyaltyService {
    async fn accrue_for_order(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        customer_id: &str,
        order_id: &str,
        total_amount_minor: i64,
    ) -> DomainResult<bool> {
        // Disambiguate to the inherent method of the same name.
        LoyaltyService::accrue_for_order(
            self,
            event_id,
            tenant,
            customer_id,
            order_id,
            total_amount_minor,
        )
        .await
    }
}

pub struct LoyaltyIngestor {
    accruer: Arc<dyn OrderPointsAccruer>,
}

impl LoyaltyIngestor {
    pub fn new(accruer: Arc<dyn OrderPointsAccruer>) -> Self {
        Self { accruer }
    }
}

#[async_trait]
impl InboundEventHandler for LoyaltyIngestor {
    async fn handle(&self, subject: &str, payload: &[u8]) -> HandlerResult<()> {
        if subject == subjects::ORDER_CREATED {
            let event: OrderCreated = serde_json::from_slice(payload)
                .map_err(|e| HandlerError(format!("malformed OrderCreated payload: {e}")))?;
            self.accruer
                .accrue_for_order(
                    event.header.event_id,
                    &TenantId(event.header.tenant_id),
                    &event.customer_id,
                    &event.order_id,
                    event.total_amount,
                )
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
    use event_models::erp::{ErpEvent, OrderCreated};
    use event_models::EventHeader;

    use super::*;

    #[derive(Default)]
    struct FakeAccruer {
        calls: Mutex<Vec<(String, String, String, i64)>>,
    }
    #[async_trait]
    impl OrderPointsAccruer for FakeAccruer {
        async fn accrue_for_order(
            &self,
            _event_id: Uuid,
            tenant: &TenantId,
            customer_id: &str,
            order_id: &str,
            total_amount_minor: i64,
        ) -> DomainResult<bool> {
            self.calls.lock().unwrap().push((
                tenant.0.clone(),
                customer_id.to_string(),
                order_id.to_string(),
                total_amount_minor,
            ));
            Ok(true)
        }
    }

    fn order_created_bytes(tenant: &str, customer: &str, total: i64) -> Vec<u8> {
        let header = EventHeader::new(
            Uuid::now_v7(),
            Utc::now(),
            tenant.to_string(),
            "order",
            "o1",
            "OrderCreated",
            1,
        );
        let event = ErpEvent::OrderCreated(OrderCreated {
            header,
            order_id: "o1".into(),
            customer_id: customer.to_string(),
            total_amount: total,
            currency: "USD".into(),
            status: "NEW".into(),
        });
        event.payload_json().unwrap()
    }

    #[tokio::test]
    async fn accrues_on_order_created() {
        let accruer = Arc::new(FakeAccruer::default());
        let ingestor = LoyaltyIngestor::new(accruer.clone());

        ingestor
            .handle(
                subjects::ORDER_CREATED,
                &order_created_bytes("t1", "cust9", 5000),
            )
            .await
            .unwrap();

        let calls = accruer.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            (
                "t1".to_string(),
                "cust9".to_string(),
                "o1".to_string(),
                5000
            )
        );
    }

    #[tokio::test]
    async fn ignores_other_subjects() {
        let accruer = Arc::new(FakeAccruer::default());
        let ingestor = LoyaltyIngestor::new(accruer.clone());

        ingestor
            .handle(
                "platform.erp.order.paid",
                &order_created_bytes("t1", "c", 1),
            )
            .await
            .unwrap();

        assert!(accruer.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_malformed_payload() {
        let accruer = Arc::new(FakeAccruer::default());
        let ingestor = LoyaltyIngestor::new(accruer);

        let err = ingestor
            .handle(subjects::ORDER_CREATED, b"not json")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("malformed OrderCreated"));
    }
}
