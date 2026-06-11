//! Insight use-cases. Generating an insight emits `AiInsightGenerated`.

use std::sync::Arc;

use event_models::ai::{AiEvent, AiInsightGenerated};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::insights::entities::Insight;
use crate::domain::insights::ports::{GenerationRequest, InsightGenerator, InsightRepository};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, TenantId};

pub struct InsightService {
    repo: Arc<dyn InsightRepository>,
    generator: Arc<dyn InsightGenerator>,
    clock: Arc<dyn Clock>,
}

impl InsightService {
    pub fn new(
        repo: Arc<dyn InsightRepository>,
        generator: Arc<dyn InsightGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            generator,
            clock,
        }
    }

    /// Generate an insight from a context, persist it, and emit
    /// `AiInsightGenerated`. `source_ref` records the trigger (e.g. a snapshot id).
    pub async fn generate(
        &self,
        tenant_id: &TenantId,
        category_hint: Option<String>,
        context: serde_json::Value,
        source_ref: Option<String>,
    ) -> DomainResult<Insight> {
        let generated = self
            .generator
            .generate(&GenerationRequest {
                category_hint,
                context,
            })
            .await?;

        let insight = Insight {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            category: generated.category,
            summary: generated.summary,
            source_ref,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.generated_event(&insight))?;
        self.repo.create(&insight, &event).await?;
        Ok(insight)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Insight>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    fn generated_event(&self, insight: &Insight) -> AiEvent {
        AiEvent::AiInsightGenerated(AiInsightGenerated {
            header: EventHeader::new(
                Uuid::now_v7(),
                self.clock.now_utc(),
                insight.tenant_id.0.clone(),
                "insight",
                insight.id.to_string(),
                "AiInsightGenerated",
                1,
            ),
            insight_id: insight.id.to_string(),
            category: insight.category.clone(),
            summary: insight.summary.clone(),
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
    use crate::domain::insights::ports::GeneratedInsight;

    #[derive(Default)]
    struct FakeRepo {
        items: Mutex<Vec<Insight>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl InsightRepository for FakeRepo {
        async fn create(&self, insight: &Insight, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(insight.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
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

    struct EchoGenerator;
    #[async_trait]
    impl InsightGenerator for EchoGenerator {
        async fn generate(&self, request: &GenerationRequest) -> DomainResult<GeneratedInsight> {
            Ok(GeneratedInsight {
                category: request
                    .category_hint
                    .clone()
                    .unwrap_or_else(|| "general".into()),
                summary: format!("analysed {}", request.context),
            })
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (InsightService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        let svc = InsightService::new(repo.clone(), Arc::new(EchoGenerator), Arc::new(StubClock));
        (svc, repo)
    }

    #[tokio::test]
    async fn generate_persists_and_emits_event() {
        let (svc, repo) = service();
        let insight = svc
            .generate(
                &TenantId("t1".into()),
                Some("sales_anomaly".into()),
                serde_json::json!({ "total_paid": 0 }),
                Some("snap-1".into()),
            )
            .await
            .unwrap();
        assert_eq!(insight.category, "sales_anomaly");
        assert_eq!(insight.source_ref.as_deref(), Some("snap-1"));
        assert_eq!(repo.items.lock().unwrap().len(), 1);
        assert_eq!(*repo.events.lock().unwrap(), vec!["AiInsightGenerated"]);
    }
}
