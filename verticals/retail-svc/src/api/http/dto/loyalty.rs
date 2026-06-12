//! Loyalty DTOs (`/api/v1/retail/loyalty`).

use serde::{Deserialize, Serialize};

use crate::domain::loyalty::entities::{LoyaltyAccount, LoyaltyRules, PointsLedgerEntry};

#[derive(Debug, Deserialize)]
pub struct RedeemRequest {
    /// Points to redeem (must be > 0 and ≤ the current balance).
    pub points: i64,
}

#[derive(Debug, Serialize)]
pub struct LoyaltyAccountResponse {
    pub tenant_id: String,
    pub customer_id: String,
    pub points_balance: i64,
    /// Lifetime spend in minor currency units.
    pub lifetime_spend: i64,
    /// Tier under the tenant's rules: BRONZE / SILVER / GOLD.
    pub tier: String,
    pub updated_at: String,
}

impl LoyaltyAccountResponse {
    /// Build a response, resolving the tier under the tenant's configured rules.
    pub fn from_account(a: LoyaltyAccount, rules: &LoyaltyRules) -> Self {
        Self {
            tier: rules.tier_for(a.lifetime_spend_minor).as_str().to_string(),
            tenant_id: a.tenant_id.0,
            customer_id: a.customer_id,
            points_balance: a.points_balance,
            lifetime_spend: a.lifetime_spend_minor,
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoyaltyRulesResponse {
    /// Minor currency units of spend per 1 earned point.
    pub minor_per_point: i64,
    /// Lifetime spend (minor units) at which SILVER begins.
    pub silver_min: i64,
    /// Lifetime spend (minor units) at which GOLD begins.
    pub gold_min: i64,
}

impl From<LoyaltyRules> for LoyaltyRulesResponse {
    fn from(r: LoyaltyRules) -> Self {
        Self {
            minor_per_point: r.minor_per_point,
            silver_min: r.silver_min,
            gold_min: r.gold_min,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateLoyaltyRulesRequest {
    pub minor_per_point: i64,
    pub silver_min: i64,
    pub gold_min: i64,
}

impl From<UpdateLoyaltyRulesRequest> for LoyaltyRules {
    fn from(r: UpdateLoyaltyRulesRequest) -> Self {
        Self {
            minor_per_point: r.minor_per_point,
            silver_min: r.silver_min,
            gold_min: r.gold_min,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PointsLedgerEntryResponse {
    pub id: String,
    pub customer_id: String,
    /// EARN or REDEEM.
    pub kind: String,
    /// Positive magnitude of the movement.
    pub points: i64,
    pub balance_after: i64,
    /// Order id for an earn; null for a redeem.
    pub reason: Option<String>,
    pub at: String,
}

impl From<PointsLedgerEntry> for PointsLedgerEntryResponse {
    fn from(e: PointsLedgerEntry) -> Self {
        Self {
            id: e.id.to_string(),
            customer_id: e.customer_id,
            kind: e.kind.as_str().to_string(),
            points: e.points,
            balance_after: e.balance_after,
            reason: e.reason,
            at: e.at.to_rfc3339(),
        }
    }
}
