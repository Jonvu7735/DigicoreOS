//! Ports for the payments context.

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::payments::entities::Payment;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    /// Insert the payment and enqueue `event`, in one transaction.
    async fn create(&self, payment: &Payment, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_for_order(
        &self,
        tenant: &TenantId,
        order_id: &Uuid,
    ) -> DomainResult<Vec<Payment>>;
}
