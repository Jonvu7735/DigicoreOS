//! Loyalty account entity + tier (maps to `retail_svc.loyalty_accounts`).

use chrono::{DateTime, Utc};

use crate::domain::shared::types::TenantId;

/// Loyalty tier, derived from lifetime spend (minor currency units).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Bronze,
    Silver,
    Gold,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Bronze => "BRONZE",
            Tier::Silver => "SILVER",
            Tier::Gold => "GOLD",
        }
    }

    /// `GOLD` from 10,000 currency units of lifetime spend, `SILVER` from 1,000
    /// (amounts are minor units, so 1_000_000 / 100_000).
    pub fn from_lifetime_spend(minor: i64) -> Self {
        if minor >= 1_000_000 {
            Tier::Gold
        } else if minor >= 100_000 {
            Tier::Silver
        } else {
            Tier::Bronze
        }
    }
}

/// A customer's loyalty account: the vertical's aggregate, keyed by
/// `(tenant_id, customer_id)`.
#[derive(Debug, Clone)]
pub struct LoyaltyAccount {
    pub tenant_id: TenantId,
    pub customer_id: String,
    pub points_balance: i64,
    pub lifetime_spend_minor: i64,
    pub updated_at: DateTime<Utc>,
}

impl LoyaltyAccount {
    pub fn tier(&self) -> Tier {
        Tier::from_lifetime_spend(self.lifetime_spend_minor)
    }
}
