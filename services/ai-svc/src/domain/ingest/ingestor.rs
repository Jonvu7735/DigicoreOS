//! `EventIngestor`: reacts to platform events by generating insights. Today it
//! consumes `ReportSnapshotCreated`; new triggers extend the `handle` match.

use std::sync::Arc;

use async_trait::async_trait;
use event_models::reporting::{subjects, ReportSnapshotCreated};
use platform_events::{HandlerError, HandlerResult, InboundEventHandler};

use crate::domain::insights::services::InsightService;
use crate::domain::shared::types::TenantId;

pub struct EventIngestor {
    insights: Arc<InsightService>,
}

impl EventIngestor {
    pub fn new(insights: Arc<InsightService>) -> Self {
        Self { insights }
    }
}

#[async_trait]
impl InboundEventHandler for EventIngestor {
    async fn handle(&self, subject: &str, payload: &[u8]) -> HandlerResult<()> {
        if subject == subjects::SNAPSHOT_CREATED {
            let event: ReportSnapshotCreated = serde_json::from_slice(payload)
                .map_err(|e| HandlerError(format!("malformed ReportSnapshotCreated: {e}")))?;
            let tenant = TenantId(event.header.tenant_id);
            let context = serde_json::json!({
                "snapshot_id": event.snapshot_id,
                "snapshot_type": event.snapshot_type,
            });
            self.insights
                .generate(
                    &tenant,
                    Some("snapshot_digest".into()),
                    context,
                    Some(event.snapshot_id),
                )
                .await
                .map_err(|e| HandlerError(e.to_string()))?;
        }
        // Other subjects are not yet acted on; ignored so the consumer keeps
        // draining the bus.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;
    use event_models::reporting::ReportingEvent;
    use event_models::EventHeader;
    use platform_outbox::OutboxMessage;
    use uuid::Uuid;

    use super::*;
    use crate::domain::insights::entities::Insight;
    use crate::domain::insights::ports::{
        GeneratedInsight, GenerationRequest, InsightGenerator, InsightRepository,
    };
    use crate::domain::shared::error::DomainResult;
    use crate::domain::shared::types::Clock;
    use chrono::DateTime;

    #[derive(Default)]
    struct FakeRepo {
        items: Mutex<Vec<Insight>>,
    }
    #[async_trait]
    impl InsightRepository for FakeRepo {
        async fn create(&self, insight: &Insight, _event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(insight.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Insight>> {
            Ok(self.items.lock().unwrap().clone())
        }
    }

    struct StubGen;
    #[async_trait]
    impl InsightGenerator for StubGen {
        async fn generate(&self, req: &GenerationRequest) -> DomainResult<GeneratedInsight> {
            Ok(GeneratedInsight {
                category: req.category_hint.clone().unwrap_or_default(),
                summary: "stub".into(),
            })
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn snapshot_bytes(tenant: &str, stype: &str) -> Vec<u8> {
        let header = EventHeader::new(
            Uuid::now_v7(),
            Utc::now(),
            tenant.to_string(),
            "report_snapshot",
            "snap-1",
            "ReportSnapshotCreated",
            1,
        );
        let event = ReportingEvent::ReportSnapshotCreated(ReportSnapshotCreated {
            header,
            snapshot_id: "snap-1".into(),
            snapshot_type: stype.into(),
        });
        event.payload_json().unwrap()
    }

    #[tokio::test]
    async fn snapshot_event_generates_insight() {
        let repo = Arc::new(FakeRepo::default());
        let svc = Arc::new(InsightService::new(
            repo.clone(),
            Arc::new(StubGen),
            Arc::new(StubClock),
        ));
        let ingestor = EventIngestor::new(svc);

        ingestor
            .handle(subjects::SNAPSHOT_CREATED, &snapshot_bytes("t1", "sales"))
            .await
            .unwrap();

        let items = repo.items.lock().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_ref.as_deref(), Some("snap-1"));
        assert_eq!(items[0].category, "snapshot_digest");
    }

    #[tokio::test]
    async fn unrelated_subject_is_ignored() {
        let repo = Arc::new(FakeRepo::default());
        let svc = Arc::new(InsightService::new(
            repo.clone(),
            Arc::new(StubGen),
            Arc::new(StubClock),
        ));
        let ingestor = EventIngestor::new(svc);

        ingestor
            .handle("platform.erp.order.created", b"{}")
            .await
            .unwrap();
        assert!(repo.items.lock().unwrap().is_empty());
    }
}
