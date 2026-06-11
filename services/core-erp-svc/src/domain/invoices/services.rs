//! Invoice use-cases. Issuing an invoice emits `InvoiceIssued`.

use std::sync::Arc;

use event_models::erp::{ErpEvent, InvoiceIssued};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::invoices::entities::{Invoice, InvoiceStatus};
use crate::domain::invoices::ports::InvoiceRepository;
use crate::domain::orders::ports::OrderRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, Money, TenantId};

pub struct InvoiceService {
    repo: Arc<dyn InvoiceRepository>,
    orders: Arc<dyn OrderRepository>,
    clock: Arc<dyn Clock>,
}

impl InvoiceService {
    pub fn new(
        repo: Arc<dyn InvoiceRepository>,
        orders: Arc<dyn OrderRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            orders,
            clock,
        }
    }

    pub async fn issue_invoice(
        &self,
        tenant_id: &TenantId,
        order_id: Uuid,
        amount: i64,
        currency: String,
    ) -> DomainResult<Invoice> {
        let currency = currency.trim().to_uppercase();
        if amount <= 0 {
            return Err(DomainError::Validation("amount must be > 0".into()));
        }
        if currency.len() != 3 {
            return Err(DomainError::Validation(
                "currency must be a 3-letter code".into(),
            ));
        }
        self.orders
            .find_in_tenant(tenant_id, &order_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("order {order_id}")))?;

        let invoice = Invoice {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            order_id,
            amount: Money(amount),
            currency,
            status: InvoiceStatus::Issued,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.invoice_issued_event(&invoice))?;
        self.repo.create(&invoice, &event).await?;
        Ok(invoice)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Invoice>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Invoice> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("invoice {id}")))
    }

    pub async fn cancel(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Invoice> {
        let mut invoice = self.get(tenant_id, id).await?;
        if invoice.status != InvoiceStatus::Issued {
            return Err(DomainError::Validation(
                "only an issued invoice can be cancelled".into(),
            ));
        }
        invoice.status = InvoiceStatus::Cancelled;
        self.repo.update_status(&invoice).await?;
        Ok(invoice)
    }

    fn invoice_issued_event(&self, invoice: &Invoice) -> ErpEvent {
        let header = EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            invoice.tenant_id.0.clone(),
            "invoice",
            invoice.id.to_string(),
            "InvoiceIssued",
            1,
        );
        ErpEvent::InvoiceIssued(InvoiceIssued {
            header,
            invoice_id: invoice.id.to_string(),
            order_id: invoice.order_id.to_string(),
            amount: invoice.amount.0,
            currency: invoice.currency.clone(),
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
    struct FakeInvoices {
        items: Mutex<Vec<Invoice>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl InvoiceRepository for FakeInvoices {
        async fn create(&self, invoice: &Invoice, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(invoice.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Invoice>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Invoice>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|i| i.id == *id)
                .cloned())
        }
        async fn update_status(&self, invoice: &Invoice) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|i| i.id == invoice.id) {
                *slot = invoice.clone();
            }
            Ok(())
        }
    }

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

    fn service(order_exists: bool) -> (InvoiceService, Arc<FakeInvoices>) {
        let repo = Arc::new(FakeInvoices::default());
        let svc = InvoiceService::new(
            repo.clone(),
            Arc::new(FakeOrders {
                exists: order_exists,
            }),
            Arc::new(StubClock),
        );
        (svc, repo)
    }

    #[tokio::test]
    async fn issue_invoice_emits_invoice_issued() {
        let (svc, repo) = service(true);
        let invoice = svc
            .issue_invoice(&TenantId("t1".into()), Uuid::now_v7(), 9999, "usd".into())
            .await
            .unwrap();
        assert_eq!(invoice.status, InvoiceStatus::Issued);
        assert_eq!(invoice.currency, "USD");
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["InvoiceIssued".to_string()]
        );
    }

    #[tokio::test]
    async fn issue_invoice_unknown_order_is_not_found() {
        let (svc, _) = service(false);
        assert!(matches!(
            svc.issue_invoice(&TenantId("t1".into()), Uuid::now_v7(), 10, "USD".into())
                .await
                .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn cancel_issued_invoice_then_rejects_double_cancel() {
        let (svc, _) = service(true);
        let invoice = svc
            .issue_invoice(&TenantId("t1".into()), Uuid::now_v7(), 10, "USD".into())
            .await
            .unwrap();
        let cancelled = svc
            .cancel(&TenantId("t1".into()), &invoice.id)
            .await
            .unwrap();
        assert_eq!(cancelled.status, InvoiceStatus::Cancelled);
        assert!(matches!(
            svc.cancel(&TenantId("t1".into()), &invoice.id)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
