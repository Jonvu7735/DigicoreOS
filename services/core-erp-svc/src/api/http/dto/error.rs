//! Standard JSON error body + `DomainError` -> HTTP mapping.
//! Body shape: `{ "error_code": "...", "message": "...", "details": null }`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::domain::shared::error::DomainError;

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub body: ApiErrorBody,
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        let (status, error_code) = match &err {
            DomainError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            DomainError::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            DomainError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            DomainError::PermissionDenied(_) => (StatusCode::FORBIDDEN, "PERMISSION_DENIED"),
            DomainError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            DomainError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let message = match &err {
            DomainError::Internal(detail) => {
                tracing::error!(%detail, "internal domain error");
                "internal error".to_string()
            }
            other => other.to_string(),
        };

        Self {
            status,
            body: ApiErrorBody {
                error_code: error_code.into(),
                message,
                details: None,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
