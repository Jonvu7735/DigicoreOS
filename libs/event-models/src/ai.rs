//! AI events published by `ai-svc` (`EVENTS.md` §3.6).

use serde::{Deserialize, Serialize};

use crate::EventHeader;

/// NATS subjects for AI events (`platform.<domain>.<entity>.<action>`).
pub mod subjects {
    pub const INSIGHT_GENERATED: &str = "platform.ai.insight.generated";
}

/// An AI insight was generated (`EVENTS.md` §3.6.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInsightGenerated {
    pub header: EventHeader,
    pub insight_id: String,
    /// e.g. `sales_anomaly`, `churn_risk`.
    pub category: String,
    pub summary: String,
}

/// In-process wrapper so domain code hands a single type to the event publisher.
/// On the wire, only the inner payload struct is serialized, published on
/// [`AiEvent::subject`].
#[derive(Debug, Clone)]
pub enum AiEvent {
    AiInsightGenerated(AiInsightGenerated),
}

impl AiEvent {
    pub fn subject(&self) -> &'static str {
        match self {
            AiEvent::AiInsightGenerated(_) => subjects::INSIGHT_GENERATED,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            AiEvent::AiInsightGenerated(_) => "AiInsightGenerated",
        }
    }

    pub fn header(&self) -> &EventHeader {
        match self {
            AiEvent::AiInsightGenerated(e) => &e.header,
        }
    }

    /// Serialize the inner payload (the wire format) to JSON bytes.
    pub fn payload_json(&self) -> serde_json::Result<Vec<u8>> {
        match self {
            AiEvent::AiInsightGenerated(e) => serde_json::to_vec(e),
        }
    }
}
