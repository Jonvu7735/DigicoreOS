//! Inventory use-cases. Adjusting stock emits `StockAdjusted`.

use std::sync::Arc;

use event_models::erp::{ErpEvent, StockAdjusted};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::inventory::entities::{StockAdjustment, StockLevel};
use crate::domain::inventory::ports::InventoryRepository;
use crate::domain::products::ports::ProductRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, TenantId};

pub struct InventoryService {
    repo: Arc<dyn InventoryRepository>,
    products: Arc<dyn ProductRepository>,
    clock: Arc<dyn Clock>,
}

impl InventoryService {
    pub fn new(
        repo: Arc<dyn InventoryRepository>,
        products: Arc<dyn ProductRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            products,
            clock,
        }
    }

    /// Adjust stock and return the recorded movement + resulting on-hand quantity.
    pub async fn adjust_stock(
        &self,
        tenant_id: &TenantId,
        product_id: Uuid,
        warehouse_id: String,
        delta: i64,
        reason: String,
    ) -> DomainResult<(StockAdjustment, i64)> {
        let warehouse_id = warehouse_id.trim().to_string();
        let reason = reason.trim().to_string();
        if warehouse_id.is_empty() {
            return Err(DomainError::Validation("warehouse_id is required".into()));
        }
        if reason.is_empty() {
            return Err(DomainError::Validation("reason is required".into()));
        }
        if delta == 0 {
            return Err(DomainError::Validation("delta must be non-zero".into()));
        }
        self.products
            .find_in_tenant(tenant_id, &product_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("product {product_id}")))?;

        let adjustment = StockAdjustment {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            product_id,
            warehouse_id,
            delta,
            reason,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.stock_adjusted_event(&adjustment))?;
        let new_quantity = self.repo.adjust(&adjustment, &event).await?;
        Ok((adjustment, new_quantity))
    }

    pub async fn list_stock(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<StockLevel>> {
        self.repo.list_stock(tenant_id, limit, offset).await
    }

    pub async fn list_adjustments(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<StockAdjustment>> {
        self.repo.list_adjustments(tenant_id, limit, offset).await
    }

    fn stock_adjusted_event(&self, adjustment: &StockAdjustment) -> ErpEvent {
        let header = EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            adjustment.tenant_id.0.clone(),
            "inventory",
            adjustment.product_id.to_string(),
            "StockAdjusted",
            1,
        );
        ErpEvent::StockAdjusted(StockAdjusted {
            header,
            product_id: adjustment.product_id.to_string(),
            warehouse_id: adjustment.warehouse_id.clone(),
            delta: adjustment.delta,
            reason: adjustment.reason.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use platform_outbox::OutboxMessage;

    use super::*;
    use crate::domain::products::entities::Product;
    use crate::domain::shared::types::Money;

    #[derive(Default)]
    struct FakeInventory {
        adjustments: Mutex<Vec<StockAdjustment>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl InventoryRepository for FakeInventory {
        async fn adjust(
            &self,
            adjustment: &StockAdjustment,
            event: &OutboxMessage,
        ) -> DomainResult<i64> {
            self.adjustments.lock().unwrap().push(adjustment.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(adjustment.delta.max(0))
        }
        async fn list_stock(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<StockLevel>> {
            Ok(vec![])
        }
        async fn list_adjustments(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<StockAdjustment>> {
            Ok(self.adjustments.lock().unwrap().clone())
        }
    }

    struct FakeProducts {
        exists: bool,
    }
    #[async_trait]
    impl ProductRepository for FakeProducts {
        async fn insert(&self, _p: &Product) -> DomainResult<()> {
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Product>> {
            Ok(vec![])
        }
        async fn find_in_tenant(&self, t: &TenantId, id: &Uuid) -> DomainResult<Option<Product>> {
            Ok(self.exists.then(|| Product {
                id: *id,
                tenant_id: t.clone(),
                sku: "S".into(),
                name: "P".into(),
                price: Money(1),
                currency: "USD".into(),
                is_active: true,
                created_at: Utc::now(),
            }))
        }
        async fn update(&self, _p: &Product) -> DomainResult<()> {
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service(product_exists: bool) -> (InventoryService, Arc<FakeInventory>) {
        let repo = Arc::new(FakeInventory::default());
        let svc = InventoryService::new(
            repo.clone(),
            Arc::new(FakeProducts {
                exists: product_exists,
            }),
            Arc::new(StubClock),
        );
        (svc, repo)
    }

    #[tokio::test]
    async fn adjust_stock_emits_stock_adjusted() {
        let (svc, repo) = service(true);
        let (adj, qty) = svc
            .adjust_stock(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "WH1".into(),
                10,
                "order".into(),
            )
            .await
            .unwrap();
        assert_eq!(adj.delta, 10);
        assert_eq!(qty, 10);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["StockAdjusted".to_string()]
        );
    }

    #[tokio::test]
    async fn adjust_stock_unknown_product_is_not_found() {
        let (svc, _) = service(false);
        assert!(matches!(
            svc.adjust_stock(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "WH1".into(),
                1,
                "x".into()
            )
            .await
            .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn adjust_stock_rejects_zero_delta() {
        let (svc, _) = service(true);
        assert!(matches!(
            svc.adjust_stock(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "WH1".into(),
                0,
                "x".into()
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
