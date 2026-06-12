//! Loyalty use-cases. The API and the inbound consumer call these; these call
//! ports. No HTTP/SQL here.

use std::sync::Arc;

use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::loyalty::entities::{
    LoyaltyAccount, LoyaltyRules, PointsEntryKind, PointsLedgerEntry,
};
use crate::domain::loyalty::events::{redeemed_outbox, PointsRedeemed};
use crate::domain::loyalty::ports::LoyaltyRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};

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
    /// The `order_id` is recorded as the ledger entry's reason.
    pub async fn accrue_for_order(
        &self,
        event_id: Uuid,
        tenant_id: &TenantId,
        customer_id: &str,
        order_id: &str,
        total_amount_minor: i64,
    ) -> DomainResult<bool> {
        let customer_id = customer_id.trim();
        if customer_id.is_empty() {
            return Err(DomainError::Validation("customer_id is required".into()));
        }
        let spend = total_amount_minor.max(0);
        let points = self.repo.get_rules(tenant_id).await?.points_for(spend);
        let order_id = order_id.trim();
        let reason = (!order_id.is_empty()).then_some(order_id);
        self.repo
            .accrue(
                event_id,
                Uuid::now_v7(),
                tenant_id,
                customer_id,
                spend,
                points,
                reason,
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

    /// The tenant's loyalty rules (default when unconfigured).
    pub async fn rules(&self, tenant_id: &TenantId) -> DomainResult<LoyaltyRules> {
        self.repo.get_rules(tenant_id).await
    }

    /// Update the tenant's loyalty rules (validated before persisting).
    pub async fn set_rules(
        &self,
        tenant_id: &TenantId,
        rules: LoyaltyRules,
    ) -> DomainResult<LoyaltyRules> {
        rules.validate().map_err(DomainError::Validation)?;
        self.repo
            .set_rules(tenant_id, &rules, self.clock.now_utc())
            .await?;
        Ok(rules)
    }

    /// A customer's points history (after confirming the account exists).
    pub async fn list_ledger(
        &self,
        tenant_id: &TenantId,
        customer_id: &str,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<PointsLedgerEntry>> {
        self.get(tenant_id, customer_id).await?;
        self.repo
            .list_ledger(tenant_id, customer_id, limit, offset)
            .await
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
        let entry = PointsLedgerEntry {
            id: Uuid::now_v7(),
            tenant_id: account.tenant_id.clone(),
            customer_id: account.customer_id.clone(),
            kind: PointsEntryKind::Redeem,
            points,
            balance_after: account.points_balance,
            reason: None,
            at: account.updated_at,
        };
        let event = redeemed_outbox(&self.redeemed_event(&account, points))?;
        self.repo.save_balance(&account, &entry, &event).await?;
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
        ledger: Mutex<Vec<PointsLedgerEntry>>,
        rules: Mutex<Option<LoyaltyRules>>,
    }
    #[async_trait]
    impl LoyaltyRepository for FakeRepo {
        async fn accrue(
            &self,
            event_id: Uuid,
            entry_id: Uuid,
            tenant: &TenantId,
            customer_id: &str,
            spend_minor: i64,
            points: i64,
            reason: Option<&str>,
            now: DateTime<Utc>,
        ) -> DomainResult<bool> {
            let mut processed = self.processed.lock().unwrap();
            if processed.contains(&event_id) {
                return Ok(false);
            }
            processed.push(event_id);
            let mut accounts = self.accounts.lock().unwrap();
            let balance_after = if let Some(a) = accounts
                .iter_mut()
                .find(|a| a.tenant_id == *tenant && a.customer_id == customer_id)
            {
                a.points_balance += points;
                a.lifetime_spend_minor += spend_minor;
                a.updated_at = now;
                a.points_balance
            } else {
                accounts.push(LoyaltyAccount {
                    tenant_id: tenant.clone(),
                    customer_id: customer_id.to_string(),
                    points_balance: points,
                    lifetime_spend_minor: spend_minor,
                    updated_at: now,
                });
                points
            };
            if points > 0 {
                self.ledger.lock().unwrap().push(PointsLedgerEntry {
                    id: entry_id,
                    tenant_id: tenant.clone(),
                    customer_id: customer_id.to_string(),
                    kind: PointsEntryKind::Earn,
                    points,
                    balance_after,
                    reason: reason.map(|s| s.to_string()),
                    at: now,
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
            entry: &PointsLedgerEntry,
            event: &OutboxMessage,
        ) -> DomainResult<()> {
            let mut accounts = self.accounts.lock().unwrap();
            if let Some(slot) = accounts
                .iter_mut()
                .find(|a| a.tenant_id == account.tenant_id && a.customer_id == account.customer_id)
            {
                *slot = account.clone();
            }
            self.ledger.lock().unwrap().push(entry.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_ledger(
            &self,
            tenant: &TenantId,
            customer_id: &str,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<PointsLedgerEntry>> {
            Ok(self
                .ledger
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.tenant_id == *tenant && e.customer_id == customer_id)
                .cloned()
                .collect())
        }
        async fn get_rules(&self, _tenant: &TenantId) -> DomainResult<LoyaltyRules> {
            Ok(self.rules.lock().unwrap().unwrap_or_default())
        }
        async fn set_rules(
            &self,
            _tenant: &TenantId,
            rules: &LoyaltyRules,
            _now: DateTime<Utc>,
        ) -> DomainResult<()> {
            *self.rules.lock().unwrap() = Some(*rules);
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
            .accrue_for_order(event_id, &tenant, "cust1", "o1", 5000)
            .await
            .unwrap());
        // Redelivery of the SAME event must not double-credit.
        assert!(!svc
            .accrue_for_order(event_id, &tenant, "cust1", "o1", 5000)
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
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "o1", 600_000)
            .await
            .unwrap();
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "o2", 600_000)
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
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "o1", 10_000)
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
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "o1", 10_000)
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

    #[tokio::test]
    async fn ledger_records_earns_and_redeems() {
        let (svc, _) = service();
        let tenant = TenantId("t1".into());
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "order-7", 10_000)
            .await
            .unwrap(); // +100, balance 100
        svc.redeem(&tenant, "c", 30).await.unwrap(); // -30, balance 70

        let ledger = svc.list_ledger(&tenant, "c", 50, 0).await.unwrap();
        let rows: Vec<(&str, i64, i64, Option<&str>)> = ledger
            .iter()
            .map(|e| {
                (
                    e.kind.as_str(),
                    e.points,
                    e.balance_after,
                    e.reason.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("EARN", 100, 100, Some("order-7")),
                ("REDEEM", 30, 70, None),
            ]
        );
    }

    #[tokio::test]
    async fn accrue_uses_tenant_rules() {
        let (svc, _) = service();
        let tenant = TenantId("t1".into());
        // Generous program: 1 point per 10 minor units (default is 1 per 100).
        svc.set_rules(
            &tenant,
            LoyaltyRules {
                minor_per_point: 10,
                silver_min: 100_000,
                gold_min: 1_000_000,
            },
        )
        .await
        .unwrap();
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "o1", 5000)
            .await
            .unwrap();
        let account = svc.get(&tenant, "c").await.unwrap();
        assert_eq!(account.points_balance, 500); // 5000 / 10, not / 100
    }

    #[tokio::test]
    async fn set_rules_rejects_invalid() {
        let (svc, _) = service();
        let tenant = TenantId("t1".into());
        assert!(matches!(
            svc.set_rules(
                &tenant,
                LoyaltyRules {
                    minor_per_point: 0, // must be > 0
                    silver_min: 100_000,
                    gold_min: 1_000_000,
                },
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
        // gold below silver is also rejected.
        assert!(matches!(
            svc.set_rules(
                &tenant,
                LoyaltyRules {
                    minor_per_point: 100,
                    silver_min: 500_000,
                    gold_min: 100_000,
                },
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn ledger_skips_zero_point_earns() {
        let (svc, _) = service();
        let tenant = TenantId("t1".into());
        // 50 minor units -> 0 points (below MINOR_PER_POINT): account still gets
        // the spend, but a points ledger shouldn't show a no-op movement.
        svc.accrue_for_order(Uuid::now_v7(), &tenant, "c", "tiny", 50)
            .await
            .unwrap();
        assert!(svc
            .list_ledger(&tenant, "c", 50, 0)
            .await
            .unwrap()
            .is_empty());
    }
}
