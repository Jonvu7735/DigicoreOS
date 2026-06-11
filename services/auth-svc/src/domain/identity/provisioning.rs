//! Value objects describing an atomic tenant-provisioning operation.
//!
//! The domain computes WHAT to create (tenant, owner, default roles + their
//! permissions, and the owner's role); `ProvisioningRepository` persists it all
//! in ONE transaction (DATA-STRATEGY.md – integrity at the transactional layer).

use crate::domain::identity::entities::{Tenant, User};
use crate::domain::identity::outbox::OutboxMessage;
use crate::domain::shared::types::RoleId;

/// A role to create for a new tenant, with the permission codes it grants.
#[derive(Debug, Clone)]
pub struct NewRole {
    pub id: RoleId,
    pub name: String,
    pub description: Option<String>,
    pub permission_codes: Vec<String>,
}

/// Everything needed to provision a tenant atomically.
#[derive(Debug, Clone)]
pub struct TenantProvisioning {
    pub tenant: Tenant,
    pub owner: User,
    pub roles: Vec<NewRole>,
    /// Name of the role (within `roles`) assigned to the owner user.
    pub owner_role: String,
    /// Events enqueued into the outbox in the same transaction.
    pub events: Vec<OutboxMessage>,
}
