//! Generic report envelope, matching the OpenAPI `ReportData` schema
//! (`{ report, generated_at, data }`). Report handlers build the `data` payload.

use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReportEnvelope {
    pub report: String,
    pub generated_at: String,
    pub data: serde_json::Value,
}

impl ReportEnvelope {
    pub fn new(report: &str, data: serde_json::Value) -> Self {
        Self {
            report: report.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            data,
        }
    }
}
