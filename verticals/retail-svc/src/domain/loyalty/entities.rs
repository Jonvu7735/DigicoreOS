//! Loyalty account entity + tier (maps to `retail_svc.loyalty_accounts`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

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

    /// Tier under the platform-default rules (kept for convenience / tests).
    pub fn from_lifetime_spend(minor: i64) -> Self {
        LoyaltyRules::default().tier_for(minor)
    }
}

/// Per-tenant loyalty program policy: how spend converts to points and where the
/// tier boundaries sit. Defaults match the platform's original constants, so a
/// tenant that never configures anything behaves exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyRules {
    /// Minor currency units of spend per 1 earned point.
    pub minor_per_point: i64,
    /// Lifetime spend (minor units) at which SILVER begins.
    pub silver_min: i64,
    /// Lifetime spend (minor units) at which GOLD begins.
    pub gold_min: i64,
}

impl Default for LoyaltyRules {
    fn default() -> Self {
        Self {
            minor_per_point: 100,
            silver_min: 100_000,
            gold_min: 1_000_000,
        }
    }
}

impl LoyaltyRules {
    /// Points earned for a spend amount (minor units), floor-divided by the rate.
    pub fn points_for(&self, spend_minor: i64) -> i64 {
        if self.minor_per_point <= 0 {
            return 0;
        }
        spend_minor.max(0) / self.minor_per_point
    }

    /// Tier for a lifetime-spend amount under these rules.
    pub fn tier_for(&self, lifetime_spend_minor: i64) -> Tier {
        if lifetime_spend_minor >= self.gold_min {
            Tier::Gold
        } else if lifetime_spend_minor >= self.silver_min {
            Tier::Silver
        } else {
            Tier::Bronze
        }
    }

    /// Reject nonsensical policy (non-positive rate, negative or out-of-order
    /// thresholds) before it is persisted.
    pub fn validate(&self) -> Result<(), String> {
        if self.minor_per_point <= 0 {
            return Err("minor_per_point must be > 0".into());
        }
        if self.silver_min < 0 || self.gold_min < 0 {
            return Err("tier thresholds must be >= 0".into());
        }
        if self.gold_min < self.silver_min {
            return Err("gold_min must be >= silver_min".into());
        }
        Ok(())
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

/// Direction of a points-ledger movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointsEntryKind {
    Earn,
    Redeem,
}

impl PointsEntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PointsEntryKind::Earn => "EARN",
            PointsEntryKind::Redeem => "REDEEM",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "EARN" => Some(PointsEntryKind::Earn),
            "REDEEM" => Some(PointsEntryKind::Redeem),
            _ => None,
        }
    }
}

/// One movement in a customer's points history (earn or redeem) with the
/// resulting balance. Written in the same transaction as the balance change, so
/// the ledger can never drift from the account.
#[derive(Debug, Clone)]
pub struct PointsLedgerEntry {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub customer_id: String,
    pub kind: PointsEntryKind,
    /// Positive magnitude of the movement.
    pub points: i64,
    pub balance_after: i64,
    /// Source reference — the order id for an earn; `None` for a redeem.
    pub reason: Option<String>,
    pub at: DateTime<Utc>,
}
