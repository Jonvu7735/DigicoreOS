//! Inventory read model: stock-on-hand per (product, warehouse), summed from
//! the `StockAdjusted` stream; backs the inventory summary report
//! (`/reporting/inventory-summary`).

pub mod entities;
pub mod ports;
