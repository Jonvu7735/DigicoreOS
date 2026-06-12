//! Customer handlers (ARCHITECTURE.md §3.3). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::api::http::dto::customers::{
    CreateCustomerRequest, CustomerResponse, TimelineEntry, UpdateCustomerRequest,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::activities::entities::Activity;
use crate::domain::deals::entities::Deal;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/crm/customers` (`crm_customer_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<CustomerResponse>), ApiError> {
    auth.0.require_permission("crm_customer_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let customer = state
        .customers
        .create(&tenant, body.name, body.email, body.phone, body.segment)
        .await?;
    Ok((StatusCode::CREATED, Json(customer.into())))
}

/// `GET /api/v1/crm/customers` (`crm_customer_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<CustomerResponse>>, ApiError> {
    auth.0.require_permission("crm_customer_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let customers = state
        .customers
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(CustomerResponse::from)
        .collect();
    Ok(Json(customers))
}

/// `GET /api/v1/crm/customers/{customer_id}` (`crm_customer_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<CustomerResponse>, ApiError> {
    auth.0.require_permission("crm_customer_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let customer = state.customers.get(&tenant, &customer_id).await?;
    Ok(Json(customer.into()))
}

/// `PATCH /api/v1/crm/customers/{customer_id}` (`crm_customer_update`).
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<Uuid>,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<Json<CustomerResponse>, ApiError> {
    auth.0.require_permission("crm_customer_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let customer = state
        .customers
        .update(
            &tenant,
            &customer_id,
            body.name,
            body.email,
            body.phone,
            body.segment,
        )
        .await?;
    Ok(Json(customer.into()))
}

/// `GET /api/v1/crm/customers/{customer_id}/timeline` (`crm_customer_read`) —
/// the customer's history (deals + logged activities), newest first. Orders live
/// in erp-svc and are not visible across schemas, so the timeline is CRM-owned
/// events only.
pub async fn timeline(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<TimelineEntry>>, ApiError> {
    auth.0.require_permission("crm_customer_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    // 404 if the customer doesn't exist in this tenant.
    state.customers.get(&tenant, &customer_id).await?;
    let (limit, offset) = query.limit_offset();
    // Pull enough from each source that the merged page is complete.
    let window = limit + offset;
    let deals = state
        .deals
        .list_for_customer(&tenant, &customer_id, window, 0)
        .await?;
    let activities = state
        .activities
        .list_for_customer(&tenant, &customer_id, window, 0)
        .await?;
    Ok(Json(build_timeline(
        deals,
        activities,
        limit as usize,
        offset as usize,
    )))
}

/// Merge deals + activities into a newest-first timeline, then take the requested
/// page. Pure (no I/O), so it is unit-tested directly.
fn build_timeline(
    deals: Vec<Deal>,
    activities: Vec<Activity>,
    limit: usize,
    offset: usize,
) -> Vec<TimelineEntry> {
    let mut rows: Vec<(DateTime<Utc>, TimelineEntry)> =
        Vec::with_capacity(deals.len() + activities.len());
    for d in deals {
        rows.push((
            d.created_at,
            TimelineEntry {
                kind: "deal".into(),
                ref_id: d.id.to_string(),
                summary: format!("{} ({})", d.title, d.stage.as_str()),
                occurred_at: d.created_at.to_rfc3339(),
            },
        ));
    }
    for a in activities {
        rows.push((
            a.occurred_at,
            TimelineEntry {
                kind: "activity".into(),
                ref_id: a.id.to_string(),
                summary: format!("{}: {}", a.kind.as_str(), a.subject),
                occurred_at: a.occurred_at.to_rfc3339(),
            },
        ));
    }
    rows.sort_by_key(|(when, _)| std::cmp::Reverse(*when));
    rows.into_iter()
        .skip(offset)
        .take(limit)
        .map(|(_, e)| e)
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::build_timeline;
    use crate::domain::activities::entities::{Activity, ActivityKind};
    use crate::domain::deals::entities::{Deal, DealStage};
    use crate::domain::shared::types::{Money, TenantId};

    fn at(hour: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 12, hour, 0, 0).unwrap()
    }

    fn deal(title: &str, when: chrono::DateTime<Utc>) -> Deal {
        Deal {
            id: Uuid::now_v7(),
            tenant_id: TenantId("t1".into()),
            customer_id: Uuid::now_v7(),
            title: title.into(),
            amount_estimate: Money(0),
            stage: DealStage::Lead,
            created_at: when,
        }
    }

    fn activity(subject: &str, when: chrono::DateTime<Utc>) -> Activity {
        Activity {
            id: Uuid::now_v7(),
            tenant_id: TenantId("t1".into()),
            customer_id: Uuid::now_v7(),
            kind: ActivityKind::Call,
            subject: subject.into(),
            notes: None,
            occurred_at: when,
            created_at: when,
        }
    }

    #[test]
    fn merges_newest_first_and_paginates() {
        let deals = vec![deal("A", at(9)), deal("C", at(13))];
        let acts = vec![activity("B", at(11))];

        // Page 1 (size 2): newest two -> C(13, deal), B(11, activity).
        let page = build_timeline(deals.clone(), acts.clone(), 2, 0);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].kind, "deal");
        assert_eq!(page[0].summary, "C (LEAD)");
        assert_eq!(page[1].kind, "activity");
        assert_eq!(page[1].summary, "CALL: B");

        // Page 2 (size 2): the remaining oldest -> A(9).
        let page2 = build_timeline(deals, acts, 2, 2);
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].summary, "A (LEAD)");
    }
}
