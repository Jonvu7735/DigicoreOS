//! Real `Assistant` backed by the Anthropic Messages API.
//!
//! Slots in behind the same `Assistant` port as `StubAssistant`, so domain and
//! api are untouched. Selected by `bootstrap::wiring` only when both an API key
//! and a model are configured; otherwise the stub is used (like NATS being
//! optional). Rust has no official Anthropic SDK, so this is a thin raw-HTTP
//! client. The request-building and response-parsing are unit-tested; the
//! network round-trip is exercised only in a configured deployment.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::assistant::ports::{AssistKind, AssistRequest, Assistance, Assistant};
use crate::domain::shared::error::{DomainError, DomainResult};

/// Anthropic API version pin (sent as the `anthropic-version` header).
const API_VERSION: &str = "2023-06-01";
/// Upper bound on answer length — a business assistant reply, not an essay.
const MAX_TOKENS: u32 = 1024;
/// Overall deadline for a single LLM call. Without this, a stalled upstream
/// would hold the request handler (and its DB connection) open indefinitely.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Cap on establishing the TCP/TLS connection (DNS + handshake).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Attempts per call: the first try plus retries for transient transport errors
/// (timeout / connect). HTTP error statuses are NOT retried — they are returned.
const MAX_ATTEMPTS: u32 = 3;

/// System prompt establishing the assistant's role and grounding it in the
/// caller-supplied context.
const SYSTEM_PROMPT: &str = "You are the assistant for DigicoreOS, an AI-first \
ERP/CRM/HRM platform for small and medium businesses. Answer the user's question \
clearly and concisely, grounded in the context provided. If the context does not \
contain enough information to answer, say so plainly rather than guessing.";

pub struct ClaudeAssistant {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl ClaudeAssistant {
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        // A bounded-timeout client: a hung LLM call must not pin a request
        // handler open. Falls back to the default client only if the builder
        // somehow fails (it cannot with these options).
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl Assistant for ClaudeAssistant {
    async fn respond(&self, request: &AssistRequest) -> DomainResult<Assistance> {
        let body = MessagesRequest {
            model: &self.model,
            max_tokens: MAX_TOKENS,
            system: SYSTEM_PROMPT,
            messages: vec![Message {
                role: "user",
                content: build_user_message(request),
            }],
        };

        let url = format!("{}/v1/messages", self.base_url);
        let mut attempt = 1;
        let resp = loop {
            let result = self
                .http
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", API_VERSION)
                .json(&body)
                .send()
                .await;
            match result {
                Ok(resp) => break resp,
                // Retry only transient transport failures; surface anything else.
                Err(e) if attempt < MAX_ATTEMPTS && (e.is_timeout() || e.is_connect()) => {
                    let backoff = Duration::from_millis(200 * 2u64.pow(attempt - 1));
                    tracing::warn!(attempt, error = %e, "LLM request failed; retrying");
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
                Err(e) => {
                    return Err(DomainError::Internal(format!("LLM request failed: {e}")));
                }
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(DomainError::Internal(format!("LLM returned HTTP {status}")));
        }

        let parsed: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::Internal(format!("LLM response decode failed: {e}")))?;

        Ok(answer_from(parsed, &self.model))
    }
}

/// Build the user turn from the assist request (question/screen + any context).
fn build_user_message(request: &AssistRequest) -> String {
    let mut msg = match request.kind {
        AssistKind::Query => request.query.clone().unwrap_or_default(),
        AssistKind::Assist => {
            let screen = request.screen.as_deref().unwrap_or("the current");
            let mut m = format!("I need help on the '{screen}' screen.");
            if let Some(q) = request.query.as_deref().filter(|q| !q.is_empty()) {
                m.push_str(&format!(" Specifically: {q}"));
            }
            m
        }
    };
    if !request.context.is_null() {
        if let Ok(ctx) = serde_json::to_string_pretty(&request.context) {
            msg.push_str("\n\nContext:\n");
            msg.push_str(&ctx);
        }
    }
    msg
}

/// Turn the API response into an `Assistance`, concatenating text blocks and
/// surfacing a safe message when the model declined or returned nothing.
fn answer_from(resp: MessagesResponse, fallback_model: &str) -> Assistance {
    let model = resp.model.unwrap_or_else(|| fallback_model.to_string());
    if resp.stop_reason.as_deref() == Some("refusal") {
        return Assistance {
            answer: "The assistant declined to answer this request.".to_string(),
            model,
        };
    }
    let answer: String = resp
        .content
        .into_iter()
        .filter(|b| b.kind == "text")
        .map(|b| b.text)
        .collect::<Vec<_>>()
        .join("");
    let answer = if answer.trim().is_empty() {
        "The assistant returned no answer.".to_string()
    } else {
        answer
    };
    Assistance { answer, model }
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(q: &str, context: serde_json::Value) -> AssistRequest {
        AssistRequest {
            kind: AssistKind::Query,
            query: Some(q.to_string()),
            screen: None,
            context,
        }
    }

    #[test]
    fn user_message_includes_query_and_context() {
        let req = query("What is my top customer?", serde_json::json!({"a": 1}));
        let msg = build_user_message(&req);
        assert!(msg.contains("What is my top customer?"));
        assert!(msg.contains("Context:"));
        assert!(msg.contains("\"a\": 1"));
    }

    #[test]
    fn user_message_for_assist_names_the_screen() {
        let req = AssistRequest {
            kind: AssistKind::Assist,
            query: Some("how do I book?".into()),
            screen: Some("trade-export/shipments".into()),
            context: serde_json::Value::Null,
        };
        let msg = build_user_message(&req);
        assert!(msg.contains("trade-export/shipments"));
        assert!(msg.contains("how do I book?"));
        assert!(!msg.contains("Context:")); // null context omitted
    }

    #[test]
    fn answer_concatenates_text_blocks() {
        let resp = serde_json::from_value::<MessagesResponse>(serde_json::json!({
            "content": [
                {"type": "text", "text": "Hello "},
                {"type": "text", "text": "world"}
            ],
            "stop_reason": "end_turn",
            "model": "served-model"
        }))
        .unwrap();
        let a = answer_from(resp, "configured-model");
        assert_eq!(a.answer, "Hello world");
        assert_eq!(a.model, "served-model"); // prefers the served model id
    }

    #[test]
    fn refusal_yields_safe_message() {
        let resp = serde_json::from_value::<MessagesResponse>(serde_json::json!({
            "content": [],
            "stop_reason": "refusal"
        }))
        .unwrap();
        let a = answer_from(resp, "configured-model");
        assert!(a.answer.contains("declined"));
        assert_eq!(a.model, "configured-model"); // falls back when none served
    }

    #[test]
    fn empty_content_yields_placeholder() {
        let resp = serde_json::from_value::<MessagesResponse>(serde_json::json!({
            "content": [{"type": "text", "text": "   "}],
            "stop_reason": "end_turn"
        }))
        .unwrap();
        let a = answer_from(resp, "m");
        assert!(a.answer.contains("no answer"));
    }
}
