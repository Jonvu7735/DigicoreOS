//! Shipment use-cases. Handlers and the inbound consumer call these; these call
//! ports. No HTTP/SQL here.

use std::sync::Arc;

use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};
use crate::domain::shipments::entities::{CargoLine, ExportShipment, ShipmentStatus};
use crate::domain::shipments::events::{shipment_outbox, subjects, ShipmentEvent};
use crate::domain::shipments::ports::{CargoLineRepository, ShipmentRepository};

/// Validated input for [`ShipmentService::add_cargo_line`].
pub struct NewCargoLine {
    pub description: String,
    pub hs_code: Option<String>,
    pub quantity: i64,
    pub unit: String,
    pub net_weight_kg: Option<f64>,
}

pub struct ShipmentService {
    repo: Arc<dyn ShipmentRepository>,
    cargo: Arc<dyn CargoLineRepository>,
    clock: Arc<dyn Clock>,
}

impl ShipmentService {
    pub fn new(
        repo: Arc<dyn ShipmentRepository>,
        cargo: Arc<dyn CargoLineRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, cargo, clock }
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

    /// Add a cargo line (packing-list row) to a shipment. Only allowed while the
    /// shipment still accepts changes (DRAFT/BOOKED) — once dispatched/cancelled
    /// the manifest is frozen.
    pub async fn add_cargo_line(
        &self,
        tenant_id: &TenantId,
        shipment_id: &Uuid,
        input: NewCargoLine,
    ) -> DomainResult<CargoLine> {
        let shipment = self.get(tenant_id, shipment_id).await?;
        if !shipment.status.accepts_cargo_changes() {
            return Err(DomainError::Validation(format!(
                "cannot add cargo to a {} shipment",
                shipment.status.as_str()
            )));
        }

        let description = input.description.trim().to_string();
        if description.is_empty() {
            return Err(DomainError::Validation("description is required".into()));
        }
        if input.quantity <= 0 {
            return Err(DomainError::Validation(
                "quantity must be greater than 0".into(),
            ));
        }
        let unit = input.unit.trim().to_uppercase();
        if unit.is_empty() || unit.len() > 8 {
            return Err(DomainError::Validation(
                "unit is required (max 8 chars)".into(),
            ));
        }
        let hs_code = match input.hs_code {
            Some(code) if !code.trim().is_empty() => {
                let code = code.trim().to_string();
                if !(6..=10).contains(&code.len()) || !code.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(DomainError::Validation(
                        "hs_code must be 6–10 digits".into(),
                    ));
                }
                Some(code)
            }
            _ => None,
        };
        if let Some(w) = input.net_weight_kg {
            if !w.is_finite() || w < 0.0 {
                return Err(DomainError::Validation(
                    "net_weight_kg must be a non-negative number".into(),
                ));
            }
        }

        let line = CargoLine {
            id: Uuid::now_v7(),
            shipment_id: *shipment_id,
            tenant_id: tenant_id.clone(),
            description,
            hs_code,
            quantity: input.quantity,
            unit,
            net_weight_kg: input.net_weight_kg,
            created_at: self.clock.now_utc(),
        };
        self.cargo.insert(&line).await?;
        Ok(line)
    }

    /// List a shipment's cargo lines (after confirming it exists in the tenant).
    pub async fn list_cargo_lines(
        &self,
        tenant_id: &TenantId,
        shipment_id: &Uuid,
    ) -> DomainResult<Vec<CargoLine>> {
        self.get(tenant_id, shipment_id).await?;
        self.cargo.list_for_shipment(tenant_id, shipment_id).await
    }

    /// Book a draft shipment with a carrier (DRAFT→BOOKED); emits `ShipmentBooked`.
    pub async fn book(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<ExportShipment> {
        self.transition(
            tenant_id,
            id,
            ShipmentStatus::Booked,
            "ShipmentBooked",
            subjects::SHIPMENT_BOOKED,
        )
        .await
    }

    /// Mark a booked shipment dispatched (BOOKED→DISPATCHED); emits `ShipmentDispatched`.
    pub async fn dispatch(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<ExportShipment> {
        self.transition(
            tenant_id,
            id,
            ShipmentStatus::Dispatched,
            "ShipmentDispatched",
            subjects::SHIPMENT_DISPATCHED,
        )
        .await
    }

    /// Cancel a draft or booked shipment (→CANCELLED); emits `ShipmentCancelled`.
    pub async fn cancel(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<ExportShipment> {
        self.transition(
            tenant_id,
            id,
            ShipmentStatus::Cancelled,
            "ShipmentCancelled",
            subjects::SHIPMENT_CANCELLED,
        )
        .await
    }

    /// Validate a status transition against the machine, persist it, and enqueue
    /// the matching event in one transaction (via the repo's `save_status`).
    async fn transition(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        next: ShipmentStatus,
        event_type: &'static str,
        subject: &'static str,
    ) -> DomainResult<ExportShipment> {
        let mut shipment = self.get(tenant_id, id).await?;
        if !shipment.status.can_transition_to(next) {
            return Err(DomainError::Validation(format!(
                "cannot move shipment from {} to {}",
                shipment.status.as_str(),
                next.as_str()
            )));
        }
        shipment.status = next;
        let event = shipment_outbox(&self.status_event(&shipment, event_type), subject)?;
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

    fn status_event(&self, shipment: &ExportShipment, event_type: &'static str) -> ShipmentEvent {
        ShipmentEvent {
            header: EventHeader::new(
                Uuid::now_v7(),
                self.clock.now_utc(),
                shipment.tenant_id.0.clone(),
                "export_shipment",
                shipment.id.to_string(),
                event_type,
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

    #[derive(Default)]
    struct FakeCargoRepo {
        lines: Mutex<Vec<CargoLine>>,
    }
    #[async_trait]
    impl CargoLineRepository for FakeCargoRepo {
        async fn insert(&self, line: &CargoLine) -> DomainResult<()> {
            self.lines.lock().unwrap().push(line.clone());
            Ok(())
        }
        async fn list_for_shipment(
            &self,
            _t: &TenantId,
            shipment_id: &Uuid,
        ) -> DomainResult<Vec<CargoLine>> {
            Ok(self
                .lines
                .lock()
                .unwrap()
                .iter()
                .filter(|l| l.shipment_id == *shipment_id)
                .cloned()
                .collect())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (ShipmentService, Arc<FakeRepo>) {
        let (svc, repo, _cargo) = service_full();
        (svc, repo)
    }

    fn service_full() -> (ShipmentService, Arc<FakeRepo>, Arc<FakeCargoRepo>) {
        let repo = Arc::new(FakeRepo::default());
        let cargo = Arc::new(FakeCargoRepo::default());
        (
            ShipmentService::new(repo.clone(), cargo.clone(), Arc::new(StubClock)),
            repo,
            cargo,
        )
    }

    fn sample_cargo() -> NewCargoLine {
        NewCargoLine {
            description: "  Dried mango 500g  ".into(),
            hs_code: Some("08045000".into()),
            quantity: 120,
            unit: "ctn".into(),
            net_weight_kg: Some(360.0),
        }
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
    async fn full_lifecycle_book_then_dispatch() {
        let (svc, repo) = service();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        svc.book(&t, &s.id).await.unwrap();
        let dispatched = svc.dispatch(&t, &s.id).await.unwrap();
        assert_eq!(dispatched.status, ShipmentStatus::Dispatched);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec![
                "ShipmentBooked".to_string(),
                "ShipmentDispatched".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn dispatch_rejects_a_draft() {
        let (svc, _) = service();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        // DRAFT -> DISPATCHED is invalid (must book first).
        assert!(matches!(
            svc.dispatch(&t, &s.id).await.unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn cancel_from_draft_then_blocks_dispatch() {
        let (svc, repo) = service();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        let cancelled = svc.cancel(&t, &s.id).await.unwrap();
        assert_eq!(cancelled.status, ShipmentStatus::Cancelled);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["ShipmentCancelled".to_string()]
        );
        // A cancelled shipment is terminal.
        assert!(matches!(
            svc.dispatch(&t, &s.id).await.unwrap_err(),
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

    #[tokio::test]
    async fn add_cargo_line_validates_and_persists() {
        let (svc, _repo, cargo) = service_full();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        let line = svc.add_cargo_line(&t, &s.id, sample_cargo()).await.unwrap();
        assert_eq!(line.description, "Dried mango 500g"); // trimmed
        assert_eq!(line.unit, "CTN"); // upper-cased
        assert_eq!(line.quantity, 120);
        assert_eq!(line.hs_code.as_deref(), Some("08045000"));
        assert_eq!(line.shipment_id, s.id);
        assert_eq!(cargo.lines.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn add_cargo_line_can_book_then_still_add() {
        // BOOKED still accepts cargo changes.
        let (svc, _repo, _cargo) = service_full();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        svc.book(&t, &s.id).await.unwrap();
        assert!(svc.add_cargo_line(&t, &s.id, sample_cargo()).await.is_ok());
    }

    #[tokio::test]
    async fn add_cargo_rejects_dispatched() {
        let (svc, _repo, _cargo) = service_full();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        svc.book(&t, &s.id).await.unwrap();
        svc.dispatch(&t, &s.id).await.unwrap();
        // DISPATCHED freezes the manifest.
        assert!(matches!(
            svc.add_cargo_line(&t, &s.id, sample_cargo())
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn add_cargo_rejects_bad_input() {
        let (svc, _repo, _cargo) = service_full();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();

        let zero_qty = NewCargoLine {
            quantity: 0,
            ..sample_cargo()
        };
        assert!(matches!(
            svc.add_cargo_line(&t, &s.id, zero_qty).await.unwrap_err(),
            DomainError::Validation(_)
        ));

        let bad_hs = NewCargoLine {
            hs_code: Some("0804.50".into()), // dots aren't digits
            ..sample_cargo()
        };
        assert!(matches!(
            svc.add_cargo_line(&t, &s.id, bad_hs).await.unwrap_err(),
            DomainError::Validation(_)
        ));

        let blank_desc = NewCargoLine {
            description: "   ".into(),
            ..sample_cargo()
        };
        assert!(matches!(
            svc.add_cargo_line(&t, &s.id, blank_desc).await.unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn list_cargo_lines_returns_added_for_shipment() {
        let (svc, _repo, _cargo) = service_full();
        let t = TenantId("t1".into());
        let s = svc
            .create(&t, "US".into(), "CIF".into(), None)
            .await
            .unwrap();
        svc.add_cargo_line(&t, &s.id, sample_cargo()).await.unwrap();
        svc.add_cargo_line(
            &t,
            &s.id,
            NewCargoLine {
                description: "Cashew nuts".into(),
                hs_code: None,
                quantity: 40,
                unit: "BAG".into(),
                net_weight_kg: None,
            },
        )
        .await
        .unwrap();
        let lines = svc.list_cargo_lines(&t, &s.id).await.unwrap();
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn list_cargo_lines_unknown_shipment_is_404() {
        let (svc, _repo, _cargo) = service_full();
        let t = TenantId("t1".into());
        assert!(matches!(
            svc.list_cargo_lines(&t, &Uuid::now_v7()).await.unwrap_err(),
            DomainError::NotFound(_)
        ));
    }
}
