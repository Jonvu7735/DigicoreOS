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
