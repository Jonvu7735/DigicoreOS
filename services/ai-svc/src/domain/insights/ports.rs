//! Ports for the insights context.
//!
//! `InsightGenerator` is the AI boundary: a real adapter calls an LLM/embedding
//! model; the bundled stub is deterministic (this environment has no model
//! credentials). `InsightRepository` persists insights + the outbox event.

use async_trait::async_trait;
use platform_outbox::OutboxMessage;

use crate::domain::insights::entities::Insight;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// What to analyse. `context` is free-form JSON (a snapshot payload, a screen
/// state, …); `category_hint` nudges the classification.
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub category_hint: Option<String>,
    pub context: serde_json::Value,
}

/// The model's output: a classified, summarised insight.
#[derive(Debug, Clone)]
pub struct GeneratedInsight {
    pub category: String,
    pub summary: String,
}

/// The AI model boundary. Implementations live in `infra/ai`.
#[async_trait]
pub trait InsightGenerator: Send + Sync {
    async fn generate(&self, request: &GenerationRequest) -> DomainResult<GeneratedInsight>;
}

#[async_trait]
pub trait InsightRepository: Send + Sync {
    /// Insert the insight and enqueue `event` (AiInsightGenerated), in one tx.
    async fn create(&self, insight: &Insight, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Insight>>;
}
