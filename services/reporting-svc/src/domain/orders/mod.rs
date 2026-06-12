//! Orders read model: a per-order projection of the `OrderCreated` stream,
//! backing the detailed order report and the overview's order rollup.

pub mod entities;
pub mod ports;
