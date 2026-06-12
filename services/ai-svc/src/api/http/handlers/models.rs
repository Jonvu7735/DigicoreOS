//! Model-management handlers (`/api/v1/ai/models`). Admin-only (`ai_config_manage`).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::models::AiModelResponse;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;

/// `GET /api/v1/ai/models` — the AI engines this service has configured.
pub async fn list(
    State(_state): State<AppState>,
    auth: Auth,
) -> Result<Json<Vec<AiModelResponse>>, ApiError> {
    auth.0.require_permission("ai_config_manage")?;
    // Deterministic engines today (no external model credentials); a real adapter
    // would surface its configured models here.
    Ok(Json(vec![
        AiModelResponse {
            id: "stub-assistant-v1".into(),
            name: "Deterministic assistant (no external model)".into(),
            enabled: true,
        },
        AiModelResponse {
            id: "stub-insight-v1".into(),
            name: "Deterministic insight generator".into(),
            enabled: true,
        },
    ]))
}

/// `POST /api/v1/ai/models/reload` — reload model/prompt configuration.
pub async fn reload(State(_state): State<AppState>, auth: Auth) -> Result<StatusCode, ApiError> {
    auth.0.require_permission("ai_config_manage")?;
    // No external model config to reload yet; acknowledge the request.
    Ok(StatusCode::ACCEPTED)
}
