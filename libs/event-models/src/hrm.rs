//! HRM events published by `hrm-svc` (`EVENTS.md` §3.4).

use serde::{Deserialize, Serialize};

use crate::EventHeader;

/// NATS subjects for HRM events (`platform.<domain>.<entity>.<action>`).
pub mod subjects {
    pub const EMPLOYEE_HIRED: &str = "platform.hrm.employee.hired";
    pub const EMPLOYEE_TERMINATED: &str = "platform.hrm.employee.terminated";
}

/// A new employee was hired (`EVENTS.md` §3.4.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeHired {
    pub header: EventHeader,
    pub employee_id: String,
    pub full_name: String,
    pub position: String,
}

/// An employee was terminated (`EVENTS.md` §3.4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeTerminated {
    pub header: EventHeader,
    pub employee_id: String,
    pub reason: Option<String>,
}

/// In-process wrapper so domain code hands a single type to the event publisher.
/// On the wire, only the inner payload struct is serialized, published on
/// [`HrmEvent::subject`].
#[derive(Debug, Clone)]
pub enum HrmEvent {
    EmployeeHired(EmployeeHired),
    EmployeeTerminated(EmployeeTerminated),
}

impl HrmEvent {
    pub fn subject(&self) -> &'static str {
        match self {
            HrmEvent::EmployeeHired(_) => subjects::EMPLOYEE_HIRED,
            HrmEvent::EmployeeTerminated(_) => subjects::EMPLOYEE_TERMINATED,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            HrmEvent::EmployeeHired(_) => "EmployeeHired",
            HrmEvent::EmployeeTerminated(_) => "EmployeeTerminated",
        }
    }

    pub fn header(&self) -> &EventHeader {
        match self {
            HrmEvent::EmployeeHired(e) => &e.header,
            HrmEvent::EmployeeTerminated(e) => &e.header,
        }
    }

    /// Serialize the inner payload (the wire format) to JSON bytes.
    pub fn payload_json(&self) -> serde_json::Result<Vec<u8>> {
        match self {
            HrmEvent::EmployeeHired(e) => serde_json::to_vec(e),
            HrmEvent::EmployeeTerminated(e) => serde_json::to_vec(e),
        }
    }
}
