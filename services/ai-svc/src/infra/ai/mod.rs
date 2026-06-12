//! AI model adapters implementing `domain::insights::ports::InsightGenerator`.
//!
//! `stub_generator` is a deterministic, no-network implementation used until a
//! real LLM/embedding adapter (e.g. the Claude API) is configured — both slot in
//! behind the same domain port.

pub mod stub_assistant;
pub mod stub_generator;
