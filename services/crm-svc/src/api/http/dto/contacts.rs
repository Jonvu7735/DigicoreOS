//! Contact DTOs (`/api/v1/crm/contacts`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::contacts::entities::Contact;

#[derive(Debug, Deserialize)]
pub struct CreateContactRequest {
    pub customer_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContactResponse {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
}

impl From<Contact> for ContactResponse {
    fn from(c: Contact) -> Self {
        Self {
            id: c.id.to_string(),
            tenant_id: c.tenant_id.0,
            customer_id: c.customer_id.to_string(),
            name: c.name,
            email: c.email,
            phone: c.phone,
            title: c.title,
            created_at: c.created_at.to_rfc3339(),
        }
    }
}
