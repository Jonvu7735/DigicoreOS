//! Shipment use-cases. Handlers and the inbound consumer call these; these call
//! ports. No HTTP/SQL here.

use std::sync::Arc;

use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};
use crate::domain::shipments::entities::{ExportShipment, ShipmentStatus};
use crate::domain::shipments::events::{booked_outbox, ShipmentBooked};
use crate::domain::shipments::ports::ShipmentRepository;

pub struct ShipmentService {
    repo: Arc<dyn ShipmentRepository>,
    clock: Arc<dyn Clock>,
}

impl ShipmentService {
    pub fn new(repo: Arc<dyn ShipmentRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Create a shipment explicitly (e.g. an export not tied to an ERP order).
    pub async fn create(
        &self,
        tenant_id: &TenantId,
        destination_country: String,
        incoterm: String,
        order_id: Option<String>,
    ) -> DomainResult<ExportShipment> {
        let destination_country = destination_country.trim().to_uppercase();
        let incoterm = incoterm.trim().to_uppercase();
        if destination_country.len() != 2 {
            return Err(DomainError::Validation(
                "destination_country must be a 2-letter ISO code".into(),
            ));
        }
        if incoterm.is_empty() {
            return Err(DomainError::Validation("incoterm is required".into()));
        }
        let order_id = order_id
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty());

        let id = Uuid::now_v7();
        let shipment = ExportShipment {
            id,
            tenant_id: tenant_id.clone(),
            order_id,
            reference: ExportShipment::reference_for(&id),
            destination_country,
            incoterm,
            status: ShipmentStatus::Draft,
            created_at: self.clock.now_utc(),
        };
        self.repo.insert(&shipment).await?;
        Ok(shipment)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ExportShipment>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<ExportShipment> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("shipment {id}")))
    }

    /// Book a draft shipment with a carrier; emits `ShipmentBooked`.
    pub async fn book(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<ExportShipment> {
        let mut shipment = self.get(tenant_id, id).await?;
        if !shipment.status.can_transition_to(ShipmentStatus::Booked) {
            return Err(DomainError::Validation(format!(
                "cannot book a shipment in status {}",
                shipment.status.as_str()
            )));
        }
        shipment.status = ShipmentStatus::Booked;
        let event = booked_outbox(&self.booked_event(&shipment))?;
        self.repo.save_status(&shipment, &event).await?;
        Ok(shipment)
    }

    /// Idempotently draft a shipment for a paid ERP order (called by the inbound
    /// consumer). Returns `None` when a shipment for the order already exists, so
    /// at-least-once redelivery never creates duplicates.
    pub async fn draft_from_order(
        &self,
        tenant_id: &TenantId,
        order_id: &str,
    ) -> DomainResult<Option<ExportShipment>> {
        if self
            .repo
            .find_by_order(tenant_id, order_id)
            .await?
            .is_some()
        {
            return Ok(None);
        }
        let id = Uuid::now_v7();
        let shipment = ExportShipment {
            id,
            tenant_id: tenant_id.clone(),
            order_id: Some(order_id.to_string()),
            reference: ExportShipment::reference_for(&id),
            destination_country: String::new(),
            incoterm: String::new(),
            status: ShipmentStatus::Draft,
            created_at: self.clock.now_utc(),
        };
        self.repo.insert(&shipment).await?;
        Ok(Some(shipment))
    }

    fn booked_event(&self, shipment: &ExportShipment) -> ShipmentBooked {
        ShipmentBooked {
            header: EventHeader::new(
                Uuid::now_v7(),
                self.clock.now_utc(),
                shipment.tenant_id.0.clone(),
                "export_shipment",
                shipment.id.to_string(),
                "ShipmentBooked",
                1,
            ),
            shipment_id: shipment.id.to_string(),
            reference: shipment.reference.clone(),
            destination_country: shipment.destination_country.clone(),
            order_id: shipment.order_id.clone(),
        }
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
        items: Mutex<Vec<ExportShipment>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl ShipmentRepository for FakeRepo {
        async fn insert(&self, shipment: &ExportShipment) -> DomainResult<()> {
            self.items.lock().unwrap().push(shipment.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<ExportShipment>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(
            &self,
            _t: &TenantId,
            id: &Uuid,
        ) -> DomainResult<Option<ExportShipment>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.id == *id)
                .cloned())
        }
        async fn find_by_order(
            &self,
            _t: &TenantId,
            order_id: &str,
        ) -> DomainResult<Option<ExportShipment>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.order_id.as_deref() == Some(order_id))
                .cloned())
        }
        async fn save_status(
            &self,
            shipment: &ExportShipment,
            event: &OutboxMessage,
        ) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|s| s.id == shipment.id) {
                *slot = shipment.clone();
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

    fn service() -> (ShipmentService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (
            ShipmentService::new(repo.clone(), Arc::new(StubClock)),
            repo,
        )
    }

    #[tokio::test]
    async fn create_validates_and_inserts() {
        let (svc, repo) = service();
        let s = svc
            .create(&TenantId("t1".into()), "vn".into(), "fob".into(), None)
            .await
            .unwrap();
        assert_eq!(s.destination_country, "VN"); // upper-cased
        assert_eq!(s.incoterm, "FOB");
        assert_eq!(s.status, ShipmentStatus::Draft);
        assert!(s.reference.starts_with("EXP-"));
        assert_eq!(repo.items.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_rejects_bad_country() {
        let (svc, _) = service();
        assert!(matches!(
            svc.create(&TenantId("t1".into()), "VNM".into(), "FOB".into(), None)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn book_transitions_and_emits_event() {
        let (svc, repo) = service();
        let s = svc
            .create(&TenantId("t1".into()), "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        let booked = svc.book(&TenantId("t1".into()), &s.id).await.unwrap();
        assert_eq!(booked.status, ShipmentStatus::Booked);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["ShipmentBooked".to_string()]
        );
    }

    #[tokio::test]
    async fn book_rejects_already_booked() {
        let (svc, _) = service();
        let s = svc
            .create(&TenantId("t1".into()), "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        svc.book(&TenantId("t1".into()), &s.id).await.unwrap();
        // BOOKED -> BOOKED is not a valid transition.
        assert!(matches!(
            svc.book(&TenantId("t1".into()), &s.id).await.unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn draft_from_order_is_idempotent() {
        let (svc, repo) = service();
        let tenant = TenantId("t1".into());
        let first = svc.draft_from_order(&tenant, "order-1").await.unwrap();
        assert!(first.is_some());
        // Redelivery of the same order produces no duplicate.
        let second = svc.draft_from_order(&tenant, "order-1").await.unwrap();
        assert!(second.is_none());
        assert_eq!(repo.items.lock().unwrap().len(), 1);
        let drafted = &repo.items.lock().unwrap()[0];
        assert_eq!(drafted.order_id.as_deref(), Some("order-1"));
        assert_eq!(drafted.status, ShipmentStatus::Draft);
    }
}
