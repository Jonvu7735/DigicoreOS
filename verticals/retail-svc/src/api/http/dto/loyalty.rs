//! Loyalty DTOs (`/api/v1/retail/loyalty`).

use serde::{Deserialize, Serialize};

use crate::domain::loyalty::entities::{LoyaltyAccount, PointsLedgerEntry};

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
    /// Derived from lifetime spend: BRONZE / SILVER / GOLD.
    pub tier: String,
    pub updated_at: String,
}

impl From<LoyaltyAccount> for LoyaltyAccountResponse {
    fn from(a: LoyaltyAccount) -> Self {
        let tier = a.tier().as_str().to_string();
        Self {
            tenant_id: a.tenant_id.0,
            customer_id: a.customer_id,
            points_balance: a.points_balance,
            lifetime_spend: a.lifetime_spend_minor,
            tier,
            updated_at: a.updated_at.to_rfc3339(),
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
