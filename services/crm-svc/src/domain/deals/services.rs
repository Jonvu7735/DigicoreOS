//! Deal use-cases. Creating a deal and moving it along the pipeline emit events.

use std::sync::Arc;

use event_models::crm::{CrmEvent, DealCreated, DealStageChanged};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::customers::ports::CustomerRepository;
use crate::domain::deals::entities::{Deal, DealStage};
use crate::domain::deals::ports::DealRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, Money, TenantId};

pub struct DealService {
    repo: Arc<dyn DealRepository>,
    customers: Arc<dyn CustomerRepository>,
    clock: Arc<dyn Clock>,
}

impl DealService {
    pub fn new(
        repo: Arc<dyn DealRepository>,
        customers: Arc<dyn CustomerRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            customers,
            clock,
        }
    }

    pub async fn create_deal(
        &self,
        tenant_id: &TenantId,
        customer_id: Uuid,
        title: String,
        amount_estimate: i64,
    ) -> DomainResult<Deal> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("title is required".into()));
        }
        if amount_estimate < 0 {
            return Err(DomainError::Validation(
                "amount_estimate must be >= 0".into(),
            ));
        }
        self.customers
            .find_in_tenant(tenant_id, &customer_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("customer {customer_id}")))?;

        let deal = Deal {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            customer_id,
            title,
            amount_estimate: Money(amount_estimate),
            stage: DealStage::Lead,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.created_event(&deal))?;
        self.repo.create(&deal, &event).await?;
        Ok(deal)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Deal>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Deal> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("deal {id}")))
    }

    pub async fn change_stage(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        new_stage: DealStage,
    ) -> DomainResult<Deal> {
        let mut deal = self.get(tenant_id, id).await?;
        if !deal.stage.can_transition_to(new_stage) {
            return Err(DomainError::Validation(format!(
                "cannot move deal from {} to {}",
                deal.stage.as_str(),
                new_stage.as_str()
            )));
        }
        let old_stage = deal.stage;
        deal.stage = new_stage;
        let event = outbox_message(&self.stage_changed_event(&deal, old_stage))?;
        self.repo.save_stage(&deal, &event).await?;
        Ok(deal)
    }

    fn created_event(&self, deal: &Deal) -> CrmEvent {
        CrmEvent::DealCreated(DealCreated {
            header: self.header(deal, "DealCreated"),
            deal_id: deal.id.to_string(),
            customer_id: deal.customer_id.to_string(),
            amount_estimate: deal.amount_estimate.0,
            stage: deal.stage.as_str().to_string(),
        })
    }

    fn stage_changed_event(&self, deal: &Deal, old: DealStage) -> CrmEvent {
        CrmEvent::DealStageChanged(DealStageChanged {
            header: self.header(deal, "DealStageChanged"),
            deal_id: deal.id.to_string(),
            old_stage: old.as_str().to_string(),
            new_stage: deal.stage.as_str().to_string(),
        })
    }

    fn header(&self, deal: &Deal, event_type: &str) -> EventHeader {
        EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            deal.tenant_id.0.clone(),
            "deal",
            deal.id.to_string(),
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
    use crate::domain::customers::entities::Customer;

    #[derive(Default)]
    struct FakeDeals {
        items: Mutex<Vec<Deal>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl DealRepository for FakeDeals {
        async fn create(&self, deal: &Deal, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(deal.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(&self, _t: &TenantId, _l: i64, _o: i64) -> DomainResult<Vec<Deal>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Deal>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|d| d.id == *id)
                .cloned())
        }
        async fn save_stage(&self, deal: &Deal, event: &OutboxMessage) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|d| d.id == deal.id) {
                *slot = deal.clone();
            }
            self.events.lock().unwrap().push(event.event_type.clone());
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

    fn service(customer_exists: bool) -> (DealService, Arc<FakeDeals>) {
        let repo = Arc::new(FakeDeals::default());
        let svc = DealService::new(
            repo.clone(),
            Arc::new(FakeCustomers {
                exists: customer_exists,
            }),
            Arc::new(StubClock),
        );
        (svc, repo)
    }

    #[tokio::test]
    async fn create_deal_emits_deal_created_at_lead() {
        let (svc, repo) = service(true);
        let deal = svc
            .create_deal(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "Big deal".into(),
                5000,
            )
            .await
            .unwrap();
        assert_eq!(deal.stage, DealStage::Lead);
        assert_eq!(*repo.events.lock().unwrap(), vec!["DealCreated"]);
    }

    #[tokio::test]
    async fn create_deal_unknown_customer_is_not_found() {
        let (svc, _) = service(false);
        assert!(matches!(
            svc.create_deal(&TenantId("t1".into()), Uuid::now_v7(), "X".into(), 1)
                .await
                .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn change_stage_valid_then_rejects_invalid() {
        let (svc, repo) = service(true);
        let deal = svc
            .create_deal(&TenantId("t1".into()), Uuid::now_v7(), "X".into(), 1)
            .await
            .unwrap();
        let qualified = svc
            .change_stage(&TenantId("t1".into()), &deal.id, DealStage::Qualified)
            .await
            .unwrap();
        assert_eq!(qualified.stage, DealStage::Qualified);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["DealCreated", "DealStageChanged"]
        );
        // QUALIFIED -> WON skips PROPOSAL: not allowed.
        assert!(matches!(
            svc.change_stage(&TenantId("t1".into()), &deal.id, DealStage::Won)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
