//! Loyalty DTOs (`/api/v1/retail/loyalty`).

use serde::{Deserialize, Serialize};

use crate::domain::loyalty::entities::LoyaltyAccount;

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
