//! Inventory read-model entities (map to `reporting_svc.stock_facts`).

/// A signed stock movement to apply (from a `StockAdjusted`).
#[derive(Debug, Clone)]
pub struct StockAdjustment {
    pub product_id: String,
    pub warehouse_id: String,
    /// Signed quantity delta (positive = increase).
    pub delta: i64,
}

/// Current stock-on-hand for one (product, warehouse).
#[derive(Debug, Clone)]
pub struct StockLevel {
    pub product_id: String,
    pub warehouse_id: String,
    pub quantity: i64,
}
