//! Reporting events published by `reporting-svc` (`EVENTS.md` §3.5).

use serde::{Deserialize, Serialize};

use crate::EventHeader;

/// NATS subjects for reporting events (`platform.<domain>.<entity>.<action>`).
pub mod subjects {
    pub const SNAPSHOT_CREATED: &str = "platform.reporting.snapshot.created";
}

/// A point-in-time report snapshot was created (`EVENTS.md` §3.5.1). The payload
/// is stored by reporting-svc; consumers fetch it by `snapshot_id` if needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSnapshotCreated {
    pub header: EventHeader,
    pub snapshot_id: String,
    /// e.g. `sales`, `inventory`.
    pub snapshot_type: String,
}

/// In-process wrapper so domain code hands a single type to the event publisher.
/// On the wire, only the inner payload struct is serialized, published on
/// [`ReportingEvent::subject`].
#[derive(Debug, Clone)]
pub enum ReportingEvent {
    ReportSnapshotCreated(ReportSnapshotCreated),
}

impl ReportingEvent {
    pub fn subject(&self) -> &'static str {
        match self {
            ReportingEvent::ReportSnapshotCreated(_) => subjects::SNAPSHOT_CREATED,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            ReportingEvent::ReportSnapshotCreated(_) => "ReportSnapshotCreated",
        }
    }

    pub fn header(&self) -> &EventHeader {
        match self {
            ReportingEvent::ReportSnapshotCreated(e) => &e.header,
        }
    }

    /// Serialize the inner payload (the wire format) to JSON bytes.
    pub fn payload_json(&self) -> serde_json::Result<Vec<u8>> {
        match self {
            ReportingEvent::ReportSnapshotCreated(e) => serde_json::to_vec(e),
        }
    }
}
