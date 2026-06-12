//! Model-management DTOs (`/api/v1/ai/models`).

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AiModelResponse {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}
