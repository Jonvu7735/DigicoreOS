//! Assistant use-cases. Handlers call these; these call the `Assistant` port.

use std::sync::Arc;

use crate::domain::assistant::ports::{AssistKind, AssistRequest, Assistance, Assistant};
use crate::domain::shared::error::{DomainError, DomainResult};

pub struct AssistantService {
    assistant: Arc<dyn Assistant>,
}

impl AssistantService {
    pub fn new(assistant: Arc<dyn Assistant>) -> Self {
        Self { assistant }
    }

    /// Answer a free-form query.
    pub async fn query(
        &self,
        query: String,
        context: serde_json::Value,
    ) -> DomainResult<Assistance> {
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err(DomainError::Validation("query is required".into()));
        }
        self.assistant
            .respond(&AssistRequest {
                kind: AssistKind::Query,
                query: Some(query),
                screen: None,
                context,
            })
            .await
    }

    /// Contextual help for a business screen.
    pub async fn assist(
        &self,
        screen: String,
        query: Option<String>,
        context: serde_json::Value,
    ) -> DomainResult<Assistance> {
        let screen = screen.trim().to_string();
        if screen.is_empty() {
            return Err(DomainError::Validation("screen is required".into()));
        }
        self.assistant
            .respond(&AssistRequest {
                kind: AssistKind::Assist,
                query,
                screen: Some(screen),
                context,
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;

    struct EchoAssistant;
    #[async_trait]
    impl Assistant for EchoAssistant {
        async fn respond(&self, request: &AssistRequest) -> DomainResult<Assistance> {
            Ok(Assistance {
                answer: format!("{:?}", request.kind),
                model: "test".into(),
            })
        }
    }

    fn service() -> AssistantService {
        AssistantService::new(Arc::new(EchoAssistant))
    }

    #[tokio::test]
    async fn query_requires_non_empty() {
        assert!(matches!(
            service().query("  ".into(), serde_json::Value::Null).await,
            Err(DomainError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn assist_requires_screen() {
        assert!(matches!(
            service()
                .assist("".into(), None, serde_json::Value::Null)
                .await,
            Err(DomainError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn query_delegates_to_engine() {
        let a = service()
            .query("hi".into(), serde_json::Value::Null)
            .await
            .unwrap();
        assert_eq!(a.answer, "Query");
        assert_eq!(a.model, "test");
    }
}
