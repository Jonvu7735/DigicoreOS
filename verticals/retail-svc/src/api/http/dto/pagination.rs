//! Shared pagination query (`?page=&page_size=`), 1-based pages.

use serde::{Deserialize, Serialize};

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
        (limit, i64::from(page - 1) * limit)
    }

    /// The effective `(page, page_size)` echoed back in the response envelope.
    pub fn page_meta(&self) -> (u32, u32) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        (page, page_size)
    }
}

/// A page of results — matches the `*Page` schemas (PageMeta + `items`) the
/// OpenAPI contract and typed clients expect for list endpoints.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, page: u32, page_size: u32) -> Self {
        Self {
            items,
            page,
            page_size,
        }
    }
}
