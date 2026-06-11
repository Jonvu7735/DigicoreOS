//! Snapshot use-cases. Creating a snapshot freezes a read model and emits
//! `ReportSnapshotCreated` (consumed by ai-svc).

use std::sync::Arc;

use event_models::reporting::{ReportSnapshotCreated, ReportingEvent};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::sales::ports::SalesProjection;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, TenantId};
use crate::domain::snapshots::entities::Snapshot;
use crate::domain::snapshots::ports::SnapshotRepository;

pub struct SnapshotService {
    repo: Arc<dyn SnapshotRepository>,
    sales: Arc<dyn SalesProjection>,
    clock: Arc<dyn Clock>,
}

impl SnapshotService {
    pub fn new(
        repo: Arc<dyn SnapshotRepository>,
        sales: Arc<dyn SalesProjection>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, sales, clock }
    }

    /// Capture the named read model for a tenant, persist it, and emit
    /// `ReportSnapshotCreated`. Only `sales` is supported until more read models
    /// land.
    pub async fn create_snapshot(
        &self,
        tenant_id: &TenantId,
        snapshot_type: String,
    ) -> DomainResult<Snapshot> {
        let snapshot_type = snapshot_type.trim().to_lowercase();
        let payload = match snapshot_type.as_str() {
            "sales" => {
                let s = self.sales.get_summary(tenant_id).await?;
                serde_json::json!({
                    "total_paid": s.total_paid.0,
                    "payment_count": s.payment_count,
                    "updated_at": s.updated_at.map(|t| t.to_rfc3339()),
                })
            }
            other => {
                return Err(DomainError::Validation(format!(
                    "unsupported snapshot_type '{other}' (supported: sales)"
                )));
            }
        };

        let snapshot = Snapshot {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            snapshot_type,
            payload,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.created_event(&snapshot))?;
        self.repo.create(&snapshot, &event).await?;
        Ok(snapshot)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Snapshot>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    fn created_event(&self, snapshot: &Snapshot) -> ReportingEvent {
        ReportingEvent::ReportSnapshotCreated(ReportSnapshotCreated {
            header: EventHeader::new(
                Uuid::now_v7(),
                self.clock.now_utc(),
                snapshot.tenant_id.0.clone(),
                "report_snapshot",
                snapshot.id.to_string(),
                "ReportSnapshotCreated",
                1,
            ),
            snapshot_id: snapshot.id.to_string(),
            snapshot_type: snapshot.snapshot_type.clone(),
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
    use crate::domain::sales::entities::SalesSummary;
    use crate::domain::shared::types::Money;

    #[derive(Default)]
    struct FakeSnapshots {
        items: Mutex<Vec<Snapshot>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl SnapshotRepository for FakeSnapshots {
        async fn create(&self, snapshot: &Snapshot, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(snapshot.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Snapshot>> {
            Ok(self.items.lock().unwrap().clone())
        }
    }

    struct StubSales;
    #[async_trait]
    impl SalesProjection for StubSales {
        async fn apply_order_paid(
            &self,
            _event_id: Uuid,
            _tenant: &TenantId,
            _amount_paid: i64,
        ) -> DomainResult<()> {
            Ok(())
        }
        async fn get_summary(&self, tenant: &TenantId) -> DomainResult<SalesSummary> {
            Ok(SalesSummary {
                tenant_id: tenant.clone(),
                total_paid: Money(12_500),
                payment_count: 3,
                updated_at: None,
            })
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (SnapshotService, Arc<FakeSnapshots>) {
        let repo = Arc::new(FakeSnapshots::default());
        let svc = SnapshotService::new(repo.clone(), Arc::new(StubSales), Arc::new(StubClock));
        (svc, repo)
    }

    #[tokio::test]
    async fn create_sales_snapshot_emits_event_and_captures_payload() {
        let (svc, repo) = service();
        let snap = svc
            .create_snapshot(&TenantId("t1".into()), "SALES".into())
            .await
            .unwrap();
        assert_eq!(snap.snapshot_type, "sales");
        assert_eq!(snap.payload["total_paid"], 12_500);
        assert_eq!(snap.payload["payment_count"], 3);
        assert_eq!(*repo.events.lock().unwrap(), vec!["ReportSnapshotCreated"]);
    }

    #[tokio::test]
    async fn create_rejects_unknown_type() {
        let (svc, _) = service();
        assert!(matches!(
            svc.create_snapshot(&TenantId("t1".into()), "weather".into())
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
