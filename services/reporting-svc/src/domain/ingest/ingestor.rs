//! `EventIngestor`: decodes a platform event (by subject) and routes it to the
//! relevant read-model projection. New read models extend the `handle` match.

use std::sync::Arc;

use async_trait::async_trait;
use event_models::erp::{subjects, OrderCreated, OrderPaid};
use platform_events::{HandlerError, HandlerResult, InboundEventHandler};

use crate::domain::orders::entities::NewOrderFact;
use crate::domain::orders::ports::OrdersProjection;
use crate::domain::sales::ports::SalesProjection;
use crate::domain::shared::types::TenantId;

pub struct EventIngestor {
    sales: Arc<dyn SalesProjection>,
    orders: Arc<dyn OrdersProjection>,
}

impl EventIngestor {
    pub fn new(sales: Arc<dyn SalesProjection>, orders: Arc<dyn OrdersProjection>) -> Self {
        Self { sales, orders }
    }
}

#[async_trait]
impl InboundEventHandler for EventIngestor {
    async fn handle(&self, subject: &str, payload: &[u8]) -> HandlerResult<()> {
        if subject == subjects::ORDER_PAID {
            let event: OrderPaid = serde_json::from_slice(payload)
                .map_err(|e| HandlerError(format!("malformed OrderPaid payload: {e}")))?;
            self.sales
                .apply_order_paid(
                    event.header.event_id,
                    &TenantId(event.header.tenant_id),
                    event.amount_paid,
                )
                .await
                .map_err(|e| HandlerError(e.to_string()))?;
        } else if subject == subjects::ORDER_CREATED {
            let event: OrderCreated = serde_json::from_slice(payload)
                .map_err(|e| HandlerError(format!("malformed OrderCreated payload: {e}")))?;
            self.orders
                .apply_order_created(&NewOrderFact {
                    order_id: event.order_id,
                    tenant_id: TenantId(event.header.tenant_id),
                    customer_id: event.customer_id,
                    total_amount: event.total_amount,
                    currency: event.currency,
                    status: event.status,
                    occurred_at: event.header.occurred_at,
                })
                .await
                .map_err(|e| HandlerError(e.to_string()))?;
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
    use crate::domain::orders::entities::{OrdersOverview, ReportedOrder};
    use crate::domain::sales::entities::SalesSummary;
    use crate::domain::shared::error::DomainResult;
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

    #[derive(Default)]
    struct FakeOrders {
        applied: Mutex<Vec<(String, i64)>>,
    }
    #[async_trait]
    impl OrdersProjection for FakeOrders {
        async fn apply_order_created(&self, fact: &NewOrderFact) -> DomainResult<()> {
            self.applied
                .lock()
                .unwrap()
                .push((fact.order_id.clone(), fact.total_amount));
            Ok(())
        }
        async fn list(&self, _t: &TenantId, _l: i64, _o: i64) -> DomainResult<Vec<ReportedOrder>> {
            Ok(vec![])
        }
        async fn overview(&self, _t: &TenantId) -> DomainResult<OrdersOverview> {
            Ok(OrdersOverview {
                order_count: 0,
                total_amount: Money(0),
            })
        }
    }

    fn ingestor() -> (EventIngestor, Arc<FakeSales>, Arc<FakeOrders>) {
        let sales = Arc::new(FakeSales::default());
        let orders = Arc::new(FakeOrders::default());
        (
            EventIngestor::new(sales.clone(), orders.clone()),
            sales,
            orders,
        )
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

    fn order_created_bytes(tenant: &str, order_id: &str, total: i64) -> Vec<u8> {
        let header = EventHeader::new(
            Uuid::now_v7(),
            Utc::now(),
            tenant.to_string(),
            "order",
            order_id.to_string(),
            "OrderCreated",
            1,
        );
        let event = ErpEvent::OrderCreated(event_models::erp::OrderCreated {
            header,
            order_id: order_id.into(),
            customer_id: "c1".into(),
            total_amount: total,
            currency: "USD".into(),
            status: "NEW".into(),
        });
        event.payload_json().unwrap()
    }

    #[tokio::test]
    async fn applies_order_paid_to_sales() {
        let (ingestor, sales, _orders) = ingestor();
        let (event_id, bytes) = order_paid_bytes("t1", 4200);

        ingestor.handle(subjects::ORDER_PAID, &bytes).await.unwrap();

        let applied = sales.applied.lock().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], (event_id, "t1".to_string(), 4200));
    }

    #[tokio::test]
    async fn applies_order_created_to_orders() {
        let (ingestor, _sales, orders) = ingestor();

        ingestor
            .handle(
                subjects::ORDER_CREATED,
                &order_created_bytes("t1", "o9", 5000),
            )
            .await
            .unwrap();

        let applied = orders.applied.lock().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], ("o9".to_string(), 5000));
    }

    #[tokio::test]
    async fn ignores_unprojected_subjects() {
        let (ingestor, sales, orders) = ingestor();

        // A subject we don't project yet is a no-op (not an error).
        ingestor
            .handle(
                "platform.erp.order.status_changed",
                &order_paid_bytes("t1", 100).1,
            )
            .await
            .unwrap();
        assert!(sales.applied.lock().unwrap().is_empty());
        assert!(orders.applied.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_malformed_payload() {
        let (ingestor, _s, _o) = ingestor();
        let err = ingestor
            .handle(subjects::ORDER_PAID, b"not json")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("malformed OrderPaid"));
    }
}
