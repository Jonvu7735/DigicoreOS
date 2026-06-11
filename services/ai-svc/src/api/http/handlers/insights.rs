//! Insight handlers (ARCHITECTURE.md §3.6). RBAC-guarded, tenant-scoped.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::insights::{GenerateInsightRequest, InsightResponse};
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/ai/insight` (`ai_assistant_use`) — generate on demand.
pub async fn generate(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<GenerateInsightRequest>,
) -> Result<(StatusCode, Json<InsightResponse>), ApiError> {
    auth.0.require_permission("ai_assistant_use")?;
    let tenant = TenantId(auth.0.tenant_id);
    let insight = state
        .insights
        .generate(&tenant, body.category_hint, body.context, None)
        .await?;
    Ok((StatusCode::CREATED, Json(insight.into())))
}

/// `GET /api/v1/ai/insights` (`ai_assistant_use`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<InsightResponse>>, ApiError> {
    auth.0.require_permission("ai_assistant_use")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let insights = state
        .insights
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(InsightResponse::from)
        .collect();
    Ok(Json(insights))
}
