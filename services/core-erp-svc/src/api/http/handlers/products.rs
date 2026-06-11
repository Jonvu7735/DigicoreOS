//! Product handlers (API-GATEWAY.md §4.4). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::dto::products::{
    CreateProductRequest, ProductResponse, UpdateProductRequest,
};
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/erp/products` (`erp_product_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), ApiError> {
    auth.0.require_permission("erp_product_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let product = state
        .products
        .create(&tenant, body.sku, body.name, body.price, body.currency)
        .await?;
    Ok((StatusCode::CREATED, Json(product.into())))
}

/// `GET /api/v1/erp/products` (`erp_product_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ProductResponse>>, ApiError> {
    auth.0.require_permission("erp_product_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let products = state
        .products
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(ProductResponse::from)
        .collect();
    Ok(Json(products))
}

/// `GET /api/v1/erp/products/{product_id}` (`erp_product_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(product_id): Path<Uuid>,
) -> Result<Json<ProductResponse>, ApiError> {
    auth.0.require_permission("erp_product_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let product = state.products.get(&tenant, &product_id).await?;
    Ok(Json(product.into()))
}

/// `PATCH /api/v1/erp/products/{product_id}` (`erp_product_update`).
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(product_id): Path<Uuid>,
    Json(body): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, ApiError> {
    auth.0.require_permission("erp_product_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let product = state
        .products
        .update(
            &tenant,
            &product_id,
            body.name,
            body.price,
            body.currency,
            body.is_active,
        )
        .await?;
    Ok(Json(product.into()))
}
