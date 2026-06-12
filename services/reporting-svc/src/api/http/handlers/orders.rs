//! Order report handler (`/api/v1/reporting/orders`). RBAC-guarded, tenant-scoped.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::http::dto::date_range;
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `?page=&page_size=&from_date=&to_date=` for the orders report.
#[derive(Debug, Deserialize)]
pub struct OrdersQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub from_date: Option<String>,
    #[serde(default)]
    pub to_date: Option<String>,
}

impl OrdersQuery {
    /// `(limit, offset)` with `page_size` clamped to 1..=100 (default 20).
    fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        let limit = i64::from(page_size);
        (limit, i64::from(page - 1) * limit)
    }
}

/// `GET /api/v1/reporting/orders` (`reporting_dashboard_view`) — detailed orders
/// from the `order_facts` projection (paginated, newest first, with an optional
/// `from_date`/`to_date` window on `created_at`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<OrdersQuery>,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (from, to) =
        date_range::parse_bounds(query.from_date.as_deref(), query.to_date.as_deref())?;
    let (limit, offset) = query.limit_offset();
    let items: Vec<serde_json::Value> = state
        .orders
        .list(&tenant, from, to, limit, offset)
        .await?
        .into_iter()
        .map(|o| {
            serde_json::json!({
                "order_id": o.order_id,
                "customer_id": o.customer_id,
                "total_amount": o.total_amount.0,
                "currency": o.currency,
                "status": o.status,
                "created_at": o.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(ReportEnvelope::new(
        "orders",
        serde_json::json!({ "items": items }),
    )))
}
