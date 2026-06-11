//! Product DTOs (`/api/v1/erp/products`).

use serde::{Deserialize, Serialize};

use crate::domain::products::entities::Product;

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub sku: String,
    pub name: String,
    /// Minor currency units.
    pub price: i64,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub price: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub tenant_id: String,
    pub sku: String,
    pub name: String,
    pub price: i64,
    pub currency: String,
    pub is_active: bool,
    pub created_at: String,
}

impl From<Product> for ProductResponse {
    fn from(p: Product) -> Self {
        Self {
            id: p.id.to_string(),
            tenant_id: p.tenant_id.0,
            sku: p.sku,
            name: p.name,
            price: p.price.0,
            currency: p.currency,
            is_active: p.is_active,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

/// Pagination query (`?page=&page_size=`), 1-based pages.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
}

impl ListQuery {
    /// `(limit, offset)` with `page_size` clamped to 1..=100 (default 20).
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        let limit = i64::from(page_size);
        (limit, i64::from(page - 1) * limit)
    }
}
