//! Payment use-cases. Recording a payment emits `OrderPaid`.

use std::sync::Arc;

use event_models::erp::{ErpEvent, OrderPaid};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::orders::ports::OrderRepository;
use crate::domain::payments::entities::Payment;
use crate::domain::payments::ports::PaymentRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, Money, TenantId};

pub struct PaymentService {
    repo: Arc<dyn PaymentRepository>,
    orders: Arc<dyn OrderRepository>,
    clock: Arc<dyn Clock>,
}

impl PaymentService {
    pub fn new(
        repo: Arc<dyn PaymentRepository>,
        orders: Arc<dyn OrderRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            orders,
            clock,
        }
    }

    pub async fn record_payment(
        &self,
        tenant_id: &TenantId,
        order_id: &Uuid,
        amount: i64,
        method: String,
    ) -> DomainResult<Payment> {
        let method = method.trim().to_string();
        if amount <= 0 {
            return Err(DomainError::Validation("amount must be > 0".into()));
        }
        if method.is_empty() {
            return Err(DomainError::Validation("payment_method is required".into()));
        }
        // The order must exist in the caller's tenant.
        self.orders
            .find_in_tenant(tenant_id, order_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("order {order_id}")))?;

        let payment = Payment {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            order_id: *order_id,
            amount: Money(amount),
            method,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.order_paid_event(&payment))?;
        self.repo.create(&payment, &event).await?;
        Ok(payment)
    }

    pub async fn list_payments(
        &self,
        tenant_id: &TenantId,
        order_id: &Uuid,
    ) -> DomainResult<Vec<Payment>> {
        self.repo.list_for_order(tenant_id, order_id).await
    }

    fn order_paid_event(&self, payment: &Payment) -> ErpEvent {
        let header = EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            payment.tenant_id.0.clone(),
            "order",
            payment.order_id.to_string(),
            "OrderPaid",
            1,
        );
        ErpEvent::OrderPaid(OrderPaid {
            header,
            order_id: payment.order_id.to_string(),
            amount_paid: payment.amount.0,
            payment_method: payment.method.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use platform_outbox::OutboxMessage;

    use super::*;
    use crate::domain::orders::entities::{Order, OrderStatus};

    #[derive(Default)]
    struct FakePayments {
        items: Mutex<Vec<Payment>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl PaymentRepository for FakePayments {
        async fn create(&self, payment: &Payment, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(payment.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_for_order(
            &self,
            _t: &TenantId,
            order_id: &Uuid,
        ) -> DomainResult<Vec<Payment>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.order_id == *order_id)
                .cloned()
                .collect())
        }
    }

    /// Order repo that reports a fixed order presence.
    struct FakeOrders {
        exists: bool,
    }
    #[async_trait]
    impl OrderRepository for FakeOrders {
        async fn create(&self, _o: &Order, _e: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Order>> {
            Ok(vec![])
        }
        async fn find_in_tenant(&self, t: &TenantId, id: &Uuid) -> DomainResult<Option<Order>> {
            Ok(self.exists.then(|| Order {
                id: *id,
                tenant_id: t.clone(),
                customer_id: "c".into(),
                total_amount: Money(100),
                currency: "USD".into(),
                status: OrderStatus::New,
                created_at: Utc::now(),
            }))
        }
        async fn save_status(&self, _o: &Order, _e: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service(order_exists: bool) -> (PaymentService, Arc<FakePayments>) {
        let repo = Arc::new(FakePayments::default());
        let svc = PaymentService::new(
            repo.clone(),
            Arc::new(FakeOrders {
                exists: order_exists,
            }),
            Arc::new(StubClock),
        );
        (svc, repo)
    }

    #[tokio::test]
    async fn record_payment_emits_order_paid() {
        let (svc, repo) = service(true);
        let payment = svc
            .record_payment(&TenantId("t1".into()), &Uuid::now_v7(), 2500, "card".into())
            .await
            .unwrap();
        assert_eq!(payment.amount.0, 2500);
        assert_eq!(*repo.events.lock().unwrap(), vec!["OrderPaid".to_string()]);
    }

    #[tokio::test]
    async fn record_payment_unknown_order_is_not_found() {
        let (svc, _) = service(false);
        assert!(matches!(
            svc.record_payment(&TenantId("t1".into()), &Uuid::now_v7(), 100, "cash".into())
                .await
                .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn record_payment_rejects_non_positive_amount() {
        let (svc, _) = service(true);
        assert!(matches!(
            svc.record_payment(&TenantId("t1".into()), &Uuid::now_v7(), 0, "card".into())
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
