//! Customers read model: a per-customer projection of the `CustomerCreated`
//! stream, backing the detailed customer report (`/reporting/customers`).

pub mod entities;
pub mod ports;
