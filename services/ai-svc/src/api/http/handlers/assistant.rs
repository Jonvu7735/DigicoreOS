//! Assistant handlers (`/api/v1/ai/query`, `/api/v1/ai/assist`). RBAC-guarded.

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::assistant::{AiAssistRequest, AiQueryRequest, AiResponse};
use crate::api::http::dto::error::ApiError;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;

/// `POST /api/v1/ai/query` (`ai_assistant_use`).
pub async fn query(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<AiQueryRequest>,
) -> Result<Json<AiResponse>, ApiError> {
    auth.0.require_permission("ai_assistant_use")?;
    let answer = state.assistant.query(body.query, body.context).await?;
    Ok(Json(answer.into()))
}

/// `POST /api/v1/ai/assist` (`ai_assistant_use`).
pub async fn assist(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<AiAssistRequest>,
) -> Result<Json<AiResponse>, ApiError> {
    auth.0.require_permission("ai_assistant_use")?;
    let answer = state
        .assistant
        .assist(body.screen, body.query, body.context)
        .await?;
    Ok(Json(answer.into()))
}
