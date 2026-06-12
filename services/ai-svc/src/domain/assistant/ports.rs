//! Ports for the assistant context.
//!
//! `Assistant` is the AI boundary: a real adapter calls an LLM; the bundled stub
//! (`infra/ai/stub_assistant.rs`) is deterministic, since this environment has no
//! model credentials. Same arrangement as the insights `InsightGenerator`.

use async_trait::async_trait;

use crate::domain::shared::error::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistKind {
    /// `POST /ai/query` — a free-form question over platform data.
    Query,
    /// `POST /ai/assist` — contextual help for a business screen.
    Assist,
}

/// What the caller asked. `context` is free-form JSON (screen state, a record, …).
#[derive(Debug, Clone)]
pub struct AssistRequest {
    pub kind: AssistKind,
    pub query: Option<String>,
    pub screen: Option<String>,
    pub context: serde_json::Value,
}

/// The model's answer.
#[derive(Debug, Clone)]
pub struct Assistance {
    pub answer: String,
    /// Which engine produced it (e.g. `stub-assistant-v1`).
    pub model: String,
}

/// The AI model boundary. Implementations live in `infra/ai`.
#[async_trait]
pub trait Assistant: Send + Sync {
    async fn respond(&self, request: &AssistRequest) -> DomainResult<Assistance>;
}
