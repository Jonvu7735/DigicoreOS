//! Ports for the invoices context.

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::invoices::entities::Invoice;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    /// Insert the invoice and enqueue `event` (InvoiceIssued), in one transaction.
    async fn create(&self, invoice: &Invoice, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Invoice>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Invoice>>;
    /// Persist a status change (e.g. cancellation). No event (EVENTS.md defines
    /// only InvoiceIssued for invoices).
    async fn update_status(&self, invoice: &Invoice) -> DomainResult<()>;
}
