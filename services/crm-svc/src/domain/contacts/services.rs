//! Contact use-cases (event-less CRUD; a contact belongs to a customer).

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::contacts::entities::Contact;
use crate::domain::contacts::ports::ContactRepository;
use crate::domain::customers::ports::CustomerRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};

pub struct ContactService {
    repo: Arc<dyn ContactRepository>,
    customers: Arc<dyn CustomerRepository>,
    clock: Arc<dyn Clock>,
}

impl ContactService {
    pub fn new(
        repo: Arc<dyn ContactRepository>,
        customers: Arc<dyn CustomerRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            customers,
            clock,
        }
    }

    pub async fn create(
        &self,
        tenant_id: &TenantId,
        customer_id: Uuid,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        title: Option<String>,
    ) -> DomainResult<Contact> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("name is required".into()));
        }
        self.customers
            .find_in_tenant(tenant_id, &customer_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("customer {customer_id}")))?;

        let contact = Contact {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            customer_id,
            name,
            email: normalize(email),
            phone: normalize(phone),
            title: normalize(title),
            created_at: self.clock.now_utc(),
        };
        self.repo.insert(&contact).await?;
        Ok(contact)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Contact>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Contact> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("contact {id}")))
    }

    pub async fn update(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
        title: Option<String>,
    ) -> DomainResult<Contact> {
        let mut contact = self.get(tenant_id, id).await?;
        if let Some(name) = name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(DomainError::Validation("name cannot be empty".into()));
            }
            contact.name = name;
        }
        if let Some(email) = email {
            contact.email = normalize(Some(email));
        }
        if let Some(phone) = phone {
            contact.phone = normalize(Some(phone));
        }
        if let Some(title) = title {
            contact.title = normalize(Some(title));
        }
        self.repo.update(&contact).await?;
        Ok(contact)
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
    use crate::domain::customers::entities::Customer;

    #[derive(Default)]
    struct FakeContacts {
        items: Mutex<Vec<Contact>>,
    }
    #[async_trait]
    impl ContactRepository for FakeContacts {
        async fn insert(&self, contact: &Contact) -> DomainResult<()> {
            self.items.lock().unwrap().push(contact.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Contact>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Contact>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.id == *id)
                .cloned())
        }
        async fn update(&self, contact: &Contact) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|c| c.id == contact.id) {
                *slot = contact.clone();
            }
            Ok(())
        }
    }

    struct FakeCustomers {
        exists: bool,
    }
    #[async_trait]
    impl CustomerRepository for FakeCustomers {
        async fn create(&self, _c: &Customer, _e: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Customer>> {
            Ok(vec![])
        }
        async fn find_in_tenant(&self, t: &TenantId, id: &Uuid) -> DomainResult<Option<Customer>> {
            Ok(self.exists.then(|| Customer {
                id: *id,
                tenant_id: t.clone(),
                name: "Acme".into(),
                email: None,
                phone: None,
                segment: None,
                created_at: Utc::now(),
            }))
        }
        async fn update(&self, _c: &Customer, _e: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service(customer_exists: bool) -> (ContactService, Arc<FakeContacts>) {
        let repo = Arc::new(FakeContacts::default());
        let svc = ContactService::new(
            repo.clone(),
            Arc::new(FakeCustomers {
                exists: customer_exists,
            }),
            Arc::new(StubClock),
        );
        (svc, repo)
    }

    #[tokio::test]
    async fn create_validates_and_inserts() {
        let (svc, repo) = service(true);
        let contact = svc
            .create(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "  Jane  ".into(),
                Some("jane@acme.io".into()),
                None,
                Some("CTO".into()),
            )
            .await
            .unwrap();
        assert_eq!(contact.name, "Jane");
        assert_eq!(contact.title.as_deref(), Some("CTO"));
        assert_eq!(repo.items.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_unknown_customer_is_not_found() {
        let (svc, _) = service(false);
        assert!(matches!(
            svc.create(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "X".into(),
                None,
                None,
                None
            )
            .await
            .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let (svc, _) = service(true);
        let c = svc
            .create(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "Old".into(),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let updated = svc
            .update(
                &TenantId("t1".into()),
                &c.id,
                Some("New".into()),
                Some("new@acme.io".into()),
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.email.as_deref(), Some("new@acme.io"));
    }
}
