//! `ProductRepository` backed by Postgres (`erp_core_svc.products`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::products::entities::Product;
use crate::domain::products::ports::ProductRepository;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{Money, TenantId};
use crate::infra::db::{map_db_err, map_write_err};

type ProductRow = (
    Uuid,
    String,
    String,
    String,
    i64,
    String,
    bool,
    DateTime<Utc>,
);

fn to_product(r: ProductRow) -> Product {
    Product {
        id: r.0,
        tenant_id: TenantId(r.1),
        sku: r.2,
        name: r.3,
        price: Money(r.4),
        currency: r.5,
        is_active: r.6,
        created_at: r.7,
    }
}

const COLS: &str = "id, tenant_id, sku, name, price, currency, is_active, created_at";

pub struct PgProductRepo {
    pool: PgPool,
}

impl PgProductRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductRepository for PgProductRepo {
    async fn insert(&self, product: &Product) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO products (id, tenant_id, sku, name, price, currency, is_active, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(product.id)
        .bind(&product.tenant_id.0)
        .bind(&product.sku)
        .bind(&product.name)
        .bind(product.price.0)
        .bind(&product.currency)
        .bind(product.is_active)
        .bind(product.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Product>> {
        let rows: Vec<ProductRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM products WHERE tenant_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_product).collect())
    }

    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Product>> {
        let row: Option<ProductRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM products WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(&tenant.0)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_product))
    }

    async fn update(&self, product: &Product) -> DomainResult<()> {
        sqlx::query(
            "UPDATE products SET name = $3, price = $4, currency = $5, is_active = $6 \
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(&product.tenant_id.0)
        .bind(product.id)
        .bind(&product.name)
        .bind(product.price.0)
        .bind(&product.currency)
        .bind(product.is_active)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }
}
