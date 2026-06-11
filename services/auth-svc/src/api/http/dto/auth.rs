//! Request/response bodies for `/api/v1/auth/...`, matching the JSON shapes
//! specified in AUTH-FLOW.md §4–6 field-for-field.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    /// Optional when the user belongs to exactly one tenant.
    pub tenant_id: Option<String>,
}

/// Self-serve sign-up (`POST /api/v1/auth/register`): provisions a new tenant
/// and its owner user in one step (API-GATEWAY.md §2.2).
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tenant_name: String,
    /// Subscription plan; defaults to `free` when omitted.
    #[serde(default)]
    pub plan: Option<String>,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String, // "Bearer"
    pub expires_in: i64,
    pub refresh_token: String,
    pub user: UserSummary,
}

#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
}

/// `POST /api/v1/auth/users` – admin creates a user with one default role.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    /// One of OWNER | ADMIN | MANAGER | STAFF | VIEWER.
    pub role: String,
}

/// `PATCH /api/v1/auth/users/{id}` – partial update (any subset of fields).
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Pagination query (`?page=&page_size=`), 1-based pages.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
}

impl ListQuery {
    /// `(limit, offset)` with `page_size` clamped to 1..=100 (default 20).
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        let limit = i64::from(page_size);
        let offset = i64::from(page - 1) * limit;
        (limit, offset)
    }
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>, // present when rotation is enabled
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// Tenant detail response (`GET`/`PATCH /api/v1/auth/tenants/{id}`).
#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub plan: String,
    pub is_active: bool,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// `PATCH /api/v1/auth/tenants/{id}` – partial update.
#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}
