//! Repository port for loyalty accounts. The redeem mutation also enqueues an
//! event into the outbox in the SAME transaction (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::loyalty::entities::LoyaltyAccount;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait LoyaltyRepository: Send + Sync {
    /// Idempotently accrue points for one order event, in one transaction:
    /// record `event_id` (so a redelivered event never double-credits) and upsert
    /// the account. Returns `true` if applied, `false` if already processed.
    async fn accrue(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        customer_id: &str,
        spend_minor: i64,
        points: i64,
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
    /// Persist the account's new balance and enqueue `event`, in one transaction.
    async fn save_balance(
        &self,
        account: &LoyaltyAccount,
        event: &OutboxMessage,
    ) -> DomainResult<()>;
}
