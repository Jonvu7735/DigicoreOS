//! Assistant DTOs (`/api/v1/ai/query`, `/api/v1/ai/assist`).

use serde::{Deserialize, Serialize};

use crate::domain::assistant::ports::Assistance;

#[derive(Debug, Deserialize)]
pub struct AiQueryRequest {
    pub query: String,
    #[serde(default)]
    pub context: serde_json::Value,
    /// Optional model override (ignored by the stub engine).
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiAssistRequest {
    /// Business screen/context key (e.g. `erp/orders`).
    pub screen: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AiResponse {
    pub answer: String,
    pub model: String,
}

impl From<Assistance> for AiResponse {
    fn from(a: Assistance) -> Self {
        Self {
            answer: a.answer,
            model: a.model,
        }
    }
}
