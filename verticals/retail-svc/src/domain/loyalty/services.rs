//! Loyalty use-cases. The API and the inbound consumer call these; these call
//! ports. No HTTP/SQL here.

use std::sync::Arc;

use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::loyalty::entities::LoyaltyAccount;
use crate::domain::loyalty::events::{redeemed_outbox, PointsRedeemed};
use crate::domain::loyalty::ports::LoyaltyRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};

/// 1 loyalty point per whole currency unit (amounts are minor units).
const MINOR_PER_POINT: i64 = 100;

pub struct LoyaltyService {
    repo: Arc<dyn LoyaltyRepository>,
    clock: Arc<dyn Clock>,
}

impl LoyaltyService {
    pub fn new(repo: Arc<dyn LoyaltyRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Accrue points for an order (called by the inbound consumer). Idempotent by
    /// `event_id`; returns whether it was applied (`false` = already processed).
    pub async fn accrue_for_order(
        &self,
        event_id: Uuid,
        tenant_id: &TenantId,
        customer_id: &str,
        total_amount_minor: i64,
    ) -> DomainResult<bool> {
        let customer_id = customer_id.trim();
        if customer_id.is_empty() {
            return Err(DomainError::Validation("customer_id is required".into()));
        }
        let spend = total_amount_minor.max(0);
        let points = spend / MINOR_PER_POINT;
        self.repo
            .accrue(
                event_id,
                tenant_id,
                customer_id,
                spend,
                points,
                self.clock.now_utc(),
            )
            .await
    }

    pub async fn get(
        &self,
        tenant_id: &TenantId,
        customer_id: &str,
    ) -> DomainResult<LoyaltyAccount> {
        self.repo
            .find(tenant_id, customer_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("loyalty account for {customer_id}")))
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<LoyaltyAccount>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    /// Redeem points from a customer's balance; emits `PointsRedeemed`.
    pub async fn redeem(
        &self,
        tenant_id: &TenantId,
        customer_id: &str,
        points: i64,
    ) -> DomainResult<LoyaltyAccount> {
        if points <= 0 {
            return Err(DomainError::Validation("points must be > 0".into()));
        }
        let mut account = self.get(tenant_id, customer_id).await?;
        if points > account.points_balance {
            return Err(DomainError::Validation(format!(
                "insufficient points: balance {} < {}",
                account.points_balance, points
            )));
        }
        account.points_balance -= points;
        account.updated_at = self.clock.now_utc();
        let event = redeemed_outbox(&self.redeemed_event(&account, points))?;
        self.repo.save_balance(&account, &event).await?;
        Ok(account)
    }

    fn redeemed_event(&self, account: &LoyaltyAccount, points: i64) -> PointsRedeemed {
        PointsRedeemed {
            header: EventHeader::new(
                Uuid::now_v7(),
                self.clock.now_utc(),
                account.tenant_id.0.clone(),
                "loyalty_account",
                account.customer_id.clone(),
                "PointsRedeemed",
                1,
            ),
            customer_id: account.customer_id.clone(),
            points_redeemed: points,
            balance_after: account.points_balance,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use platform_outbox::OutboxMessage;

    use super::*;

    #[derive(Default)]
    struct FakeRepo {
        accounts: Mutex<Vec<LoyaltyAccount>>,
        processed: Mutex<Vec<Uuid>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl LoyaltyRepository for FakeRepo {
        async fn accrue(
            &self,
            event_id: Uuid,
            tenant: &TenantId,
            customer_id: &str,
            spend_minor: i64,
            points: i64,
            now: DateTime<Utc>,
        ) -> DomainResult<bool> {
            let mut processed = self.processed.lock().unwrap();
            if processed.contains(&event_id) {
                return Ok(false);
            }
            processed.push(event_id);
            let mut accounts = self.accounts.lock().unwrap();
            if let Some(a) = accounts
                .iter_mut()
                .find(|a| a.tenant_id == *tenant && a.customer_id == customer_id)
            {
                a.points_balance += points;
                a.lifetime_spend_minor += spend_minor;
                a.updated_at = now;
            } else {
                accounts.push(LoyaltyAccount {
                    tenant_id: tenant.clone(),
                    customer_id: customer_id.to_string(),
                    points_balance: points,
                    lifetime_spend_minor: spend_minor,
                    updated_at: now,
                });
            }
            Ok(true)
        }
        async fn find(
            &self,
            tenant: &TenantId,
            customer_id: &str,
        ) -> DomainResult<Option<LoyaltyAccount>> {
            Ok(self
                .accounts
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.tenant_id == *tenant && a.customer_id == customer_id)
                .cloned())
        }
        async fn list_in_tenant(
            &self,
            tenant: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<LoyaltyAccount>> {
            Ok(self
                .accounts
                .lock()
                .unwrap()
                .iter()
                .filter(|a| a.tenant_id == *tenant)
                .cloned()
                .collect())
        }
        async fn save_balance(
            &self,
            account: &LoyaltyAccount,
            event: &OutboxMessage,
        ) -> DomainResult<()> {
            let mut accounts = self.accounts.lock().unwrap();
            if let Some(slot) = accounts
                .iter_mut()
                .find(|a| a.tenant_id == account.tenant_id && a.customer_id == account.customer_id)
            {
                *slot = account.clone();
            }
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (LoyaltyService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (LoyaltyService::new(repo.clone(), Arc::new(StubClock)), repo)
    }

    #[tokio::test]
    async fn accrue_awards_points_and_is_idempotent() {
        let (svc, repo) = service();
        let tenant = TenantId("t1".into());
        let event_id = Uuid::now_v7();

        // 5000 minor units -> 50 points.
        assert!(svc
            .accrue_for_order(event_id, &tenant, "cust1", 5000)
            .await
            .unwrap());
        // Redelivery of the SAME event must not double-credit.
        assert!(!svc
            .accrue_for_order(event_id, &tenant, "cust1", 5000)
            .await
            .unwrap());

        let account = svc.get(&tenant, "cust1").await.unwrap();
        assert_eq!(account.points_balance, 50);
        assert_eq!(account.lifetime_spend_minor, 5000);
        assert_eq!(repo.accounts.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn accrue_accumulates_across_orders_and_sets_tier() {
        let (svc, _) = service();
        let tenant = TenantId("t1".into());
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", 600_000)
            .await
            .unwrap();
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", 600_000)
            .await
            .unwrap();
        let account = svc.get(&tenant, "c").await.unwrap();
        assert_eq!(account.lifetime_spend_minor, 1_200_000);
        assert_eq!(account.points_balance, 12_000);
        assert_eq!(account.tier().as_str(), "GOLD");
    }

    #[tokio::test]
    async fn redeem_decrements_and_emits_event() {
        let (svc, repo) = service();
        let tenant = TenantId("t1".into());
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", 10_000)
            .await
            .unwrap(); // 100 points
        let account = svc.redeem(&tenant, "c", 30).await.unwrap();
        assert_eq!(account.points_balance, 70);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["PointsRedeemed".to_string()]
        );
    }

    #[tokio::test]
    async fn redeem_rejects_insufficient_and_nonpositive() {
        let (svc, _) = service();
        let tenant = TenantId("t1".into());
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", 10_000)
            .await
            .unwrap(); // 100 points
        assert!(matches!(
            svc.redeem(&tenant, "c", 1000).await.unwrap_err(),
            DomainError::Validation(_)
        ));
        assert!(matches!(
            svc.redeem(&tenant, "c", 0).await.unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn get_unknown_is_not_found() {
        let (svc, _) = service();
        assert!(matches!(
            svc.get(&TenantId("t1".into()), "nobody").await.unwrap_err(),
            DomainError::NotFound(_)
        ));
    }
}
