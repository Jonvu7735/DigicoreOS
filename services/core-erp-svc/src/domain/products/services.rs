//! Product use-cases. Handlers call these; these call ports. No HTTP/SQL here.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::products::entities::Product;
use crate::domain::products::ports::ProductRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, Money, TenantId};

pub struct ProductService {
    repo: Arc<dyn ProductRepository>,
    clock: Arc<dyn Clock>,
}

impl ProductService {
    pub fn new(repo: Arc<dyn ProductRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    pub async fn create(
        &self,
        tenant_id: &TenantId,
        sku: String,
        name: String,
        price: i64,
        currency: String,
    ) -> DomainResult<Product> {
        let sku = sku.trim().to_string();
        let name = name.trim().to_string();
        let currency = currency.trim().to_uppercase();
        validate(&sku, &name, price, &currency)?;

        let product = Product {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            sku,
            name,
            price: Money(price),
            currency,
            is_active: true,
            created_at: self.clock.now_utc(),
        };
        self.repo.insert(&product).await?;
        Ok(product)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Product>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Product> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("product {id}")))
    }

    pub async fn update(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        name: Option<String>,
        price: Option<i64>,
        currency: Option<String>,
        is_active: Option<bool>,
    ) -> DomainResult<Product> {
        let mut product = self.get(tenant_id, id).await?;
        if let Some(name) = name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(DomainError::Validation("name cannot be empty".into()));
            }
            product.name = name;
        }
        if let Some(price) = price {
            if price < 0 {
                return Err(DomainError::Validation("price must be >= 0".into()));
            }
            product.price = Money(price);
        }
        if let Some(currency) = currency {
            let currency = currency.trim().to_uppercase();
            if currency.len() != 3 {
                return Err(DomainError::Validation(
                    "currency must be a 3-letter code".into(),
                ));
            }
            product.currency = currency;
        }
        if let Some(active) = is_active {
            product.is_active = active;
        }
        self.repo.update(&product).await?;
        Ok(product)
    }
}

fn validate(sku: &str, name: &str, price: i64, currency: &str) -> DomainResult<()> {
    if sku.is_empty() {
        return Err(DomainError::Validation("sku is required".into()));
    }
    if name.is_empty() {
        return Err(DomainError::Validation("name is required".into()));
    }
    if price < 0 {
        return Err(DomainError::Validation("price must be >= 0".into()));
    }
    if currency.len() != 3 {
        return Err(DomainError::Validation(
            "currency must be a 3-letter code".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

    use super::*;

    #[derive(Default)]
    struct FakeRepo {
        items: Mutex<Vec<Product>>,
    }
    #[async_trait]
    impl ProductRepository for FakeRepo {
        async fn insert(&self, product: &Product) -> DomainResult<()> {
            self.items.lock().unwrap().push(product.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _tenant: &TenantId,
            _limit: i64,
            _offset: i64,
        ) -> DomainResult<Vec<Product>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(
            &self,
            _tenant: &TenantId,
            id: &Uuid,
        ) -> DomainResult<Option<Product>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.id == *id)
                .cloned())
        }
        async fn update(&self, product: &Product) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|p| p.id == product.id) {
                *slot = product.clone();
            }
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (ProductService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (ProductService::new(repo.clone(), Arc::new(StubClock)), repo)
    }

    #[tokio::test]
    async fn create_validates_and_inserts() {
        let (svc, repo) = service();
        let product = svc
            .create(
                &TenantId("t1".into()),
                "SKU1".into(),
                "Widget".into(),
                1999,
                "usd".into(),
            )
            .await
            .unwrap();
        assert_eq!(product.sku, "SKU1");
        assert_eq!(product.price.0, 1999);
        assert_eq!(product.currency, "USD"); // upper-cased
        assert_eq!(repo.items.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_rejects_empty_sku_and_bad_currency() {
        let (svc, _) = service();
        assert!(matches!(
            svc.create(
                &TenantId("t1".into()),
                "  ".into(),
                "X".into(),
                1,
                "USD".into()
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
        assert!(matches!(
            svc.create(
                &TenantId("t1".into()),
                "S".into(),
                "X".into(),
                1,
                "US".into()
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn get_unknown_is_not_found() {
        let (svc, _) = service();
        assert!(matches!(
            svc.get(&TenantId("t1".into()), &Uuid::now_v7())
                .await
                .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let (svc, _) = service();
        let p = svc
            .create(
                &TenantId("t1".into()),
                "S".into(),
                "Old".into(),
                100,
                "USD".into(),
            )
            .await
            .unwrap();
        let updated = svc
            .update(
                &TenantId("t1".into()),
                &p.id,
                Some("New".into()),
                Some(250),
                None,
                Some(false),
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.price.0, 250);
        assert!(!updated.is_active);
    }
}
