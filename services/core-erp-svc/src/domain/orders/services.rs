//! Order use-cases. Builds `ErpEvent`s and persists state + event atomically.

use std::sync::Arc;

use event_models::erp::{ErpEvent, OrderCreated, OrderStatusChanged};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::orders::entities::{Order, OrderStatus};
use crate::domain::orders::ports::OrderRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, Money, TenantId};

pub struct OrderService {
    repo: Arc<dyn OrderRepository>,
    clock: Arc<dyn Clock>,
}

impl OrderService {
    pub fn new(repo: Arc<dyn OrderRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    pub async fn create_order(
        &self,
        tenant_id: &TenantId,
        customer_id: String,
        total_amount: i64,
        currency: String,
    ) -> DomainResult<Order> {
        let customer_id = customer_id.trim().to_string();
        let currency = currency.trim().to_uppercase();
        if customer_id.is_empty() {
            return Err(DomainError::Validation("customer_id is required".into()));
        }
        if total_amount < 0 {
            return Err(DomainError::Validation("total_amount must be >= 0".into()));
        }
        if currency.len() != 3 {
            return Err(DomainError::Validation(
                "currency must be a 3-letter code".into(),
            ));
        }

        let order = Order {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            customer_id,
            total_amount: Money(total_amount),
            currency,
            status: OrderStatus::New,
            created_at: self.clock.now_utc(),
        };
        let event = crate::domain::shared::events::outbox_message(&self.created_event(&order))?;
        self.repo.create(&order, &event).await?;
        Ok(order)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Order>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Order> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("order {id}")))
    }

    pub async fn change_status(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        new_status: OrderStatus,
    ) -> DomainResult<Order> {
        let mut order = self.get(tenant_id, id).await?;
        if !order.status.can_transition_to(new_status) {
            return Err(DomainError::Validation(format!(
                "cannot move order from {} to {}",
                order.status.as_str(),
                new_status.as_str()
            )));
        }
        let old_status = order.status;
        order.status = new_status;
        let event = crate::domain::shared::events::outbox_message(
            &self.status_changed_event(&order, old_status),
        )?;
        self.repo.save_status(&order, &event).await?;
        Ok(order)
    }

    fn created_event(&self, order: &Order) -> ErpEvent {
        ErpEvent::OrderCreated(OrderCreated {
            header: self.header(order, "OrderCreated"),
            order_id: order.id.to_string(),
            customer_id: order.customer_id.clone(),
            total_amount: order.total_amount.0,
            currency: order.currency.clone(),
            status: order.status.as_str().to_string(),
        })
    }

    fn status_changed_event(&self, order: &Order, old: OrderStatus) -> ErpEvent {
        ErpEvent::OrderStatusChanged(OrderStatusChanged {
            header: self.header(order, "OrderStatusChanged"),
            order_id: order.id.to_string(),
            old_status: old.as_str().to_string(),
            new_status: order.status.as_str().to_string(),
        })
    }

    fn header(&self, order: &Order, event_type: &str) -> EventHeader {
        EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            order.tenant_id.0.clone(),
            "order",
            order.id.to_string(),
            event_type,
            1,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use platform_outbox::OutboxMessage;

    use super::*;

    #[derive(Default)]
    struct FakeRepo {
        orders: Mutex<Vec<Order>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl OrderRepository for FakeRepo {
        async fn create(&self, order: &Order, event: &OutboxMessage) -> DomainResult<()> {
            self.orders.lock().unwrap().push(order.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Order>> {
            Ok(self.orders.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Order>> {
            Ok(self
                .orders
                .lock()
                .unwrap()
                .iter()
                .find(|o| o.id == *id)
                .cloned())
        }
        async fn save_status(&self, order: &Order, event: &OutboxMessage) -> DomainResult<()> {
            let mut orders = self.orders.lock().unwrap();
            if let Some(slot) = orders.iter_mut().find(|o| o.id == order.id) {
                *slot = order.clone();
            }
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (OrderService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (OrderService::new(repo.clone(), Arc::new(StubClock)), repo)
    }

    #[tokio::test]
    async fn create_order_emits_order_created() {
        let (svc, repo) = service();
        let order = svc
            .create_order(&TenantId("t1".into()), "cust1".into(), 5000, "usd".into())
            .await
            .unwrap();
        assert_eq!(order.status, OrderStatus::New);
        assert_eq!(order.currency, "USD");
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["OrderCreated".to_string()]
        );
    }

    #[tokio::test]
    async fn create_order_rejects_bad_input() {
        let (svc, _) = service();
        assert!(matches!(
            svc.create_order(&TenantId("t1".into()), " ".into(), 1, "USD".into())
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn change_status_valid_transition_emits_event() {
        let (svc, repo) = service();
        let order = svc
            .create_order(&TenantId("t1".into()), "c".into(), 1, "USD".into())
            .await
            .unwrap();
        let confirmed = svc
            .change_status(&TenantId("t1".into()), &order.id, OrderStatus::Confirmed)
            .await
            .unwrap();
        assert_eq!(confirmed.status, OrderStatus::Confirmed);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["OrderCreated".to_string(), "OrderStatusChanged".to_string()]
        );
    }

    #[tokio::test]
    async fn change_status_rejects_invalid_transition() {
        let (svc, _) = service();
        let order = svc
            .create_order(&TenantId("t1".into()), "c".into(), 1, "USD".into())
            .await
            .unwrap();
        // NEW -> COMPLETED is not allowed.
        assert!(matches!(
            svc.change_status(&TenantId("t1".into()), &order.id, OrderStatus::Completed)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
