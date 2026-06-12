//! Deterministic, no-network `Assistant` (mirrors `StubInsightGenerator`).
//!
//! Stands in for a real LLM while this environment has no model credentials, so
//! the HTTP pipeline is exercisable end to end. A real adapter (e.g. Claude API)
//! replaces this behind the same port without touching domain/api.

use async_trait::async_trait;

use crate::domain::assistant::ports::{AssistKind, AssistRequest, Assistance, Assistant};
use crate::domain::shared::error::DomainResult;

/// Engine id reported in responses and `GET /ai/models`.
pub const STUB_MODEL: &str = "stub-assistant-v1";

pub struct StubAssistant;

#[async_trait]
impl Assistant for StubAssistant {
    async fn respond(&self, request: &AssistRequest) -> DomainResult<Assistance> {
        let answer = match request.kind {
            AssistKind::Query => {
                let q = request.query.as_deref().unwrap_or("");
                format!(
                    "You asked: \"{q}\". No external model is configured, so this is a \
                     deterministic placeholder — a real LLM adapter slots in behind this port."
                )
            }
            AssistKind::Assist => {
                let screen = request.screen.as_deref().unwrap_or("unknown");
                format!(
                    "Contextual help for the '{screen}' screen is not yet backed by a model; \
                     configure an LLM adapter to enable it."
                )
            }
        };
        Ok(Assistance {
            answer,
            model: STUB_MODEL.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn query_echoes_and_tags_model() {
        let a = StubAssistant
            .respond(&AssistRequest {
                kind: AssistKind::Query,
                query: Some("revenue this month?".into()),
                screen: None,
                context: serde_json::Value::Null,
            })
            .await
            .unwrap();
        assert!(a.answer.contains("revenue this month?"));
        assert_eq!(a.model, STUB_MODEL);
    }

    #[tokio::test]
    async fn assist_names_the_screen() {
        let a = StubAssistant
            .respond(&AssistRequest {
                kind: AssistKind::Assist,
                query: None,
                screen: Some("erp/orders".into()),
                context: serde_json::Value::Null,
            })
            .await
            .unwrap();
        assert!(a.answer.contains("erp/orders"));
    }
}
