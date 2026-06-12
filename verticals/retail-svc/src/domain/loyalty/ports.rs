//! Repository port for loyalty accounts. The redeem mutation also enqueues an
//! event into the outbox in the SAME transaction (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::loyalty::entities::{LoyaltyAccount, LoyaltyRules, PointsLedgerEntry};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait LoyaltyRepository: Send + Sync {
    /// Idempotently accrue points for one order event, in one transaction:
    /// record `event_id` (so a redelivered event never double-credits), upsert
    /// the account, and — when `points > 0` — append an EARN ledger entry
    /// (`entry_id`, `reason`) carrying the resulting balance. Returns `true` if
    /// applied, `false` if already processed.
    #[allow(clippy::too_many_arguments)]
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
    ) -> DomainResult<bool>;
    async fn find(
        &self,
        tenant: &TenantId,
        customer_id: &str,
    ) -> DomainResult<Option<LoyaltyAccount>>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<LoyaltyAccount>>;
    /// Persist the account's new balance, append the ledger `entry`, and enqueue
    /// `event` — all in one transaction.
    async fn save_balance(
        &self,
        account: &LoyaltyAccount,
        entry: &PointsLedgerEntry,
        event: &OutboxMessage,
    ) -> DomainResult<()>;
    /// A customer's points history, newest first.
    async fn list_ledger(
        &self,
        tenant: &TenantId,
        customer_id: &str,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<PointsLedgerEntry>>;
    /// The tenant's loyalty rules, or the platform default when unconfigured.
    async fn get_rules(&self, tenant: &TenantId) -> DomainResult<LoyaltyRules>;
    /// Upsert the tenant's loyalty rules.
    async fn set_rules(
        &self,
        tenant: &TenantId,
        rules: &LoyaltyRules,
        now: DateTime<Utc>,
    ) -> DomainResult<()>;
}
