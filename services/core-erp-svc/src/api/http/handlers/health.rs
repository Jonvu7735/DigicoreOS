//! Liveness & readiness endpoints (Kubernetes probes).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::bootstrap::wiring::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub env: String,
}

/// `GET /api/v1/erp/health` – liveness.
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: state.config.service_name,
        env: state.config.env.clone(),
    })
}

/// `GET /api/v1/erp/ready` – readiness (DB reachable).
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(HealthResponse {
                status: "ready",
                service: state.config.service_name,
                env: state.config.env.clone(),
            }),
        ),
        Err(error) => {
            tracing::warn!(%error, "readiness check failed: database unreachable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    status: "degraded",
                    service: state.config.service_name,
                    env: state.config.env.clone(),
                }),
            )
        }
    }
}
