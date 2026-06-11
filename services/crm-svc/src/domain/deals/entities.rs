//! Deal entity + pipeline stage machine (maps to `crm_svc.deals`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::{Money, TenantId};

/// Sales pipeline stage. `Won`/`Lost` are terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DealStage {
    Lead,
    Qualified,
    Proposal,
    Won,
    Lost,
}

impl DealStage {
    pub fn as_str(self) -> &'static str {
        match self {
            DealStage::Lead => "LEAD",
            DealStage::Qualified => "QUALIFIED",
            DealStage::Proposal => "PROPOSAL",
            DealStage::Won => "WON",
            DealStage::Lost => "LOST",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "LEAD" => Some(DealStage::Lead),
            "QUALIFIED" => Some(DealStage::Qualified),
            "PROPOSAL" => Some(DealStage::Proposal),
            "WON" => Some(DealStage::Won),
            "LOST" => Some(DealStage::Lost),
            _ => None,
        }
    }

    /// Allowed pipeline moves: advance LEAD→QUALIFIED→PROPOSAL→WON, and any
    /// non-terminal stage may be marked LOST. `Won`/`Lost` are terminal.
    pub fn can_transition_to(self, next: DealStage) -> bool {
        use DealStage::*;
        matches!(
            (self, next),
            (Lead, Qualified)
                | (Qualified, Proposal)
                | (Proposal, Won)
                | (Lead, Lost)
                | (Qualified, Lost)
                | (Proposal, Lost)
        )
    }
}

#[derive(Debug, Clone)]
pub struct Deal {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub customer_id: Uuid,
    pub title: String,
    /// Estimated value in minor currency units.
    pub amount_estimate: Money,
    pub stage: DealStage,
    pub created_at: DateTime<Utc>,
}
