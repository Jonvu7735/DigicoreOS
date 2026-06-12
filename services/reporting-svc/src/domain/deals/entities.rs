//! Deals read-model entities (map to `reporting_svc.deal_facts`).

use chrono::{DateTime, Utc};

use crate::domain::shared::types::TenantId;

/// The fields needed to project a new deal fact (from a `DealCreated`).
#[derive(Debug, Clone)]
pub struct NewDealFact {
    pub deal_id: String,
    pub tenant_id: TenantId,
    pub customer_id: String,
    pub amount_estimate: i64,
    pub stage: String,
    pub occurred_at: DateTime<Utc>,
}

/// A deal moved to a new stage (from a `DealStageChanged`). Applied
/// monotonically by `occurred_at`.
#[derive(Debug, Clone)]
pub struct DealStageChange {
    pub deal_id: String,
    pub tenant_id: TenantId,
    pub new_stage: String,
    pub occurred_at: DateTime<Utc>,
}

/// One row of the funnel: how many deals currently sit in a given stage.
#[derive(Debug, Clone)]
pub struct StageCount {
    pub stage: String,
    pub deal_count: i64,
}
