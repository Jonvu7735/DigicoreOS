//! Customer use-cases. Creating/updating a customer emits a CRM event.

use std::sync::Arc;

use event_models::crm::{CrmEvent, CustomerCreated, CustomerUpdated};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::customers::entities::Customer;
use crate::domain::customers::ports::CustomerRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, TenantId};

pub struct CustomerService {
    repo: Arc<dyn CustomerRepository>,
    clock: Arc<dyn Clock>,
}

impl CustomerService {
    pub fn new(repo: Arc<dyn CustomerRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    pub async fn create(
        &self,
        tenant_id: &TenantId,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        segment: Option<String>,
    ) -> DomainResult<Customer> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("name is required".into()));
        }

        let customer = Customer {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            name,
            email: normalize(email),
            phone: normalize(phone),
            segment: normalize(segment),
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.created_event(&customer))?;
        self.repo.create(&customer, &event).await?;
        Ok(customer)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Customer>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Customer> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("customer {id}")))
    }

    pub async fn update(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
        segment: Option<String>,
    ) -> DomainResult<Customer> {
        let mut customer = self.get(tenant_id, id).await?;
        if let Some(name) = name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(DomainError::Validation("name cannot be empty".into()));
            }
            customer.name = name;
        }
        if let Some(email) = email {
            customer.email = normalize(Some(email));
        }
        if let Some(phone) = phone {
            customer.phone = normalize(Some(phone));
        }
        if let Some(segment) = segment {
            customer.segment = normalize(Some(segment));
        }

        let event = outbox_message(&self.updated_event(&customer))?;
        self.repo.update(&customer, &event).await?;
        Ok(customer)
    }

    fn created_event(&self, c: &Customer) -> CrmEvent {
        CrmEvent::CustomerCreated(CustomerCreated {
            header: self.header(c, "CustomerCreated"),
            customer_id: c.id.to_string(),
            name: c.name.clone(),
            email: c.email.clone(),
            phone: c.phone.clone(),
            segment: c.segment.clone(),
        })
    }

    fn updated_event(&self, c: &Customer) -> CrmEvent {
        CrmEvent::CustomerUpdated(CustomerUpdated {
            header: self.header(c, "CustomerUpdated"),
            customer_id: c.id.to_string(),
            name: c.name.clone(),
            email: c.email.clone(),
            phone: c.phone.clone(),
            segment: c.segment.clone(),
        })
    }

    fn header(&self, c: &Customer, event_type: &str) -> EventHeader {
        EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            c.tenant_id.0.clone(),
            "customer",
            c.id.to_string(),
            event_type,
            1,
        )
    }
}

/// Trim an optional free-text field; an empty string becomes `None`.
fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
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
        items: Mutex<Vec<Customer>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl CustomerRepository for FakeRepo {
        async fn create(&self, customer: &Customer, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(customer.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Customer>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Customer>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.id == *id)
                .cloned())
        }
        async fn update(&self, customer: &Customer, event: &OutboxMessage) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|c| c.id == customer.id) {
                *slot = customer.clone();
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

    fn service() -> (CustomerService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (
            CustomerService::new(repo.clone(), Arc::new(StubClock)),
            repo,
        )
    }

    #[tokio::test]
    async fn create_emits_customer_created_and_normalizes() {
        let (svc, repo) = service();
        let customer = svc
            .create(
                &TenantId("t1".into()),
                "  Acme Co  ".into(),
                Some("  ".into()), // blank email -> None
                Some("+123".into()),
                None,
            )
            .await
            .unwrap();
        assert_eq!(customer.name, "Acme Co");
        assert_eq!(customer.email, None);
        assert_eq!(customer.phone.as_deref(), Some("+123"));
        assert_eq!(*repo.events.lock().unwrap(), vec!["CustomerCreated"]);
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let (svc, _) = service();
        assert!(matches!(
            svc.create(&TenantId("t1".into()), "   ".into(), None, None, None)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn update_changes_fields_and_emits_customer_updated() {
        let (svc, repo) = service();
        let c = svc
            .create(&TenantId("t1".into()), "Old".into(), None, None, None)
            .await
            .unwrap();
        let updated = svc
            .update(
                &TenantId("t1".into()),
                &c.id,
                Some("New".into()),
                Some("new@x.io".into()),
                None,
                Some("vip".into()),
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.email.as_deref(), Some("new@x.io"));
        assert_eq!(updated.segment.as_deref(), Some("vip"));
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["CustomerCreated", "CustomerUpdated"]
        );
    }

    #[tokio::test]
    async fn get_unknown_is_not_found() {
        let (svc, _) = service();
        assert!(matches!(
            svc.get(&TenantId("t1".into()), &Uuid::now_v7())
                .await
                .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }
}
