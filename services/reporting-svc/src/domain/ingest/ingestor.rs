//! `EventIngestor`: decodes a platform event (by subject) and routes it to the
//! relevant read-model projection. New read models extend the `handle` match.

use std::sync::Arc;

use async_trait::async_trait;
use event_models::erp::{subjects, OrderPaid};

use crate::domain::ingest::ports::InboundEventHandler;
use crate::domain::sales::ports::SalesProjection;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;

pub struct EventIngestor {
    sales: Arc<dyn SalesProjection>,
}

impl EventIngestor {
    pub fn new(sales: Arc<dyn SalesProjection>) -> Self {
        Self { sales }
    }
}

#[async_trait]
impl InboundEventHandler for EventIngestor {
    async fn handle(&self, subject: &str, payload: &[u8]) -> DomainResult<()> {
        if subject == subjects::ORDER_PAID {
            let event: OrderPaid = serde_json::from_slice(payload).map_err(|e| {
                DomainError::Validation(format!("malformed OrderPaid payload: {e}"))
            })?;
            self.sales
                .apply_order_paid(
                    event.header.event_id,
                    &TenantId(event.header.tenant_id),
                    event.amount_paid,
                )
                .await?;
        }
        // Other subjects are not yet projected; ignored so the consumer keeps
        // draining the bus.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;
    use event_models::erp::ErpEvent;
    use event_models::EventHeader;
    use uuid::Uuid;

    use super::*;
    use crate::domain::sales::entities::SalesSummary;
    use crate::domain::shared::types::Money;

    #[derive(Default)]
    struct FakeSales {
        applied: Mutex<Vec<(Uuid, String, i64)>>,
    }
    #[async_trait]
    impl SalesProjection for FakeSales {
        async fn apply_order_paid(
            &self,
            event_id: Uuid,
            tenant: &TenantId,
            amount_paid: i64,
        ) -> DomainResult<()> {
            self.applied
                .lock()
                .unwrap()
                .push((event_id, tenant.0.clone(), amount_paid));
            Ok(())
        }
        async fn get_summary(&self, tenant: &TenantId) -> DomainResult<SalesSummary> {
            Ok(SalesSummary {
                tenant_id: tenant.clone(),
                total_paid: Money(0),
                payment_count: 0,
                updated_at: None,
            })
        }
    }

    fn order_paid_bytes(tenant: &str, amount: i64) -> (Uuid, Vec<u8>) {
        let event_id = Uuid::now_v7();
        let header = EventHeader::new(
            event_id,
            Utc::now(),
            tenant.to_string(),
            "order",
            "o1",
            "OrderPaid",
            1,
        );
        let event = ErpEvent::OrderPaid(OrderPaid {
            header,
            order_id: "o1".into(),
            amount_paid: amount,
            payment_method: "card".into(),
        });
        (event_id, event.payload_json().unwrap())
    }

    #[tokio::test]
    async fn applies_order_paid_to_sales() {
        let sales = Arc::new(FakeSales::default());
        let ingestor = EventIngestor::new(sales.clone());
        let (event_id, bytes) = order_paid_bytes("t1", 4200);

        ingestor.handle(subjects::ORDER_PAID, &bytes).await.unwrap();

        let applied = sales.applied.lock().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], (event_id, "t1".to_string(), 4200));
    }

    #[tokio::test]
    async fn ignores_unprojected_subjects() {
        let sales = Arc::new(FakeSales::default());
        let ingestor = EventIngestor::new(sales.clone());
        let (_id, bytes) = order_paid_bytes("t1", 100);

        // A subject we don't project yet is a no-op (not an error).
        ingestor
            .handle("platform.erp.order.created", &bytes)
            .await
            .unwrap();
        assert!(sales.applied.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_malformed_payload() {
        let sales = Arc::new(FakeSales::default());
        let ingestor = EventIngestor::new(sales);
        let err = ingestor
            .handle(subjects::ORDER_PAID, b"not json")
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
