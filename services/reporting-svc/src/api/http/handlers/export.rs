//! Report export handler (`/api/v1/reporting/export`). RBAC-guarded
//! (`reporting_report_export`), tenant-scoped. Serializes an existing read model
//! to a downloadable file in the requested `format`.
//!
//! `csv` and `xlsx` are implemented; `pdf` (in the OpenAPI enum) returns a 400
//! until a renderer is added — a real PDF needs a heavyweight layout/font
//! dependency for marginal value over xlsx/csv. Date-range filtering (`from`/
//! `to`) is accepted but not yet applied — the read models are not range-indexed.

use axum::extract::{Query, State};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::api::http::dto::error::ApiError;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::error::DomainError;
use crate::domain::shared::types::TenantId;

/// Upper bound on rows pulled into a single export (keeps memory bounded).
const EXPORT_LIMIT: i64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// Which report to export (e.g. `sales-summary`, `orders`).
    pub report: String,
    /// `csv` | `xlsx` | `pdf` (`csv` and `xlsx` implemented; `pdf` -> 400).
    pub format: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

/// `GET /api/v1/reporting/export` (`reporting_report_export`).
pub async fn export(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    auth.0.require_permission("reporting_report_export")?;
    let tenant = TenantId(auth.0.tenant_id);

    let rows: Vec<Vec<String>> = match query.report.as_str() {
        "sales-summary" => {
            let s = state.sales.get_summary(&tenant).await?;
            vec![
                vec!["total_paid".into(), "payment_count".into()],
                vec![s.total_paid.0.to_string(), s.payment_count.to_string()],
            ]
        }
        "orders" => {
            let mut out = vec![vec![
                "order_id".into(),
                "customer_id".into(),
                "total_amount".into(),
                "currency".into(),
                "status".into(),
                "created_at".into(),
            ]];
            for o in state.orders.list(&tenant, EXPORT_LIMIT, 0).await? {
                out.push(vec![
                    o.order_id,
                    o.customer_id,
                    o.total_amount.0.to_string(),
                    o.currency,
                    o.status,
                    o.created_at.to_rfc3339(),
                ]);
            }
            out
        }
        "customers" => {
            let mut out = vec![vec![
                "customer_id".into(),
                "name".into(),
                "email".into(),
                "segment".into(),
                "created_at".into(),
            ]];
            for c in state.customers.list(&tenant, EXPORT_LIMIT, 0).await? {
                out.push(vec![
                    c.customer_id,
                    c.name,
                    c.email.unwrap_or_default(),
                    c.segment.unwrap_or_default(),
                    c.created_at.to_rfc3339(),
                ]);
            }
            out
        }
        "employees" => {
            let mut out = vec![vec![
                "employee_id".into(),
                "full_name".into(),
                "position".into(),
                "created_at".into(),
            ]];
            for e in state.employees.list(&tenant, EXPORT_LIMIT, 0).await? {
                out.push(vec![
                    e.employee_id,
                    e.full_name,
                    e.position,
                    e.created_at.to_rfc3339(),
                ]);
            }
            out
        }
        "crm-funnel" => {
            let mut out = vec![vec!["stage".into(), "deal_count".into()]];
            for s in state.deals.funnel(&tenant).await? {
                out.push(vec![s.stage, s.deal_count.to_string()]);
            }
            out
        }
        "inventory-summary" => {
            let mut out = vec![vec![
                "product_id".into(),
                "warehouse_id".into(),
                "quantity".into(),
            ]];
            for l in state.inventory.summary(&tenant).await? {
                out.push(vec![l.product_id, l.warehouse_id, l.quantity.to_string()]);
            }
            out
        }
        other => {
            return Err(DomainError::Validation(format!(
                "unknown report '{other}'; supported: sales-summary, orders, customers, \
                 employees, crm-funnel, inventory-summary"
            ))
            .into());
        }
    };

    let (bytes, content_type, ext): (Vec<u8>, &str, &str) = match query.format.as_str() {
        "csv" => (to_csv(&rows).into_bytes(), "text/csv; charset=utf-8", "csv"),
        "xlsx" => (
            to_xlsx(&rows)
                .map_err(|e| DomainError::Internal(format!("xlsx render failed: {e}")))?,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "xlsx",
        ),
        other => {
            return Err(DomainError::Validation(format!(
                "export format '{other}' is not supported yet; use 'csv' or 'xlsx'"
            ))
            .into());
        }
    };
    let filename = format!("{}.{ext}", query.report);

    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

/// Serialize rows to a single-sheet `.xlsx` workbook (one row per record).
fn to_xlsx(rows: &[Vec<String>]) -> Result<Vec<u8>, rust_xlsxwriter::XlsxError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let sheet = workbook.add_worksheet();
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            sheet.write_string(r as u32, c as u16, cell.as_str())?;
        }
    }
    workbook.save_to_buffer()
}

/// Serialize rows to RFC 4180-ish CSV (CRLF-free; fields quoted as needed).
fn to_csv(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for row in rows {
        let line = row
            .iter()
            .map(|f| csv_field(f))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Quote a field if it contains a delimiter, quote, or newline (doubling quotes).
fn csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_only_when_needed() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("line\nbreak"), "\"line\nbreak\"");
    }

    #[test]
    fn to_csv_joins_rows_with_newlines() {
        let rows = vec![
            vec!["h1".to_string(), "h2".to_string()],
            vec!["v1".to_string(), "v,2".to_string()],
        ];
        assert_eq!(to_csv(&rows), "h1,h2\nv1,\"v,2\"\n");
    }

    #[test]
    fn xlsx_is_a_nonempty_zip_workbook() {
        let rows = vec![
            vec!["h1".to_string(), "h2".to_string()],
            vec!["v1".to_string(), "v2".to_string()],
        ];
        let bytes = to_xlsx(&rows).unwrap();
        // .xlsx is an Office Open XML ZIP — verify the PK magic + real content.
        assert!(bytes.starts_with(b"PK\x03\x04"));
        assert!(bytes.len() > 200);
    }
}
