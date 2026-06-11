//! Shared pagination query (`?page=&page_size=`), 1-based pages.

use serde::Deserialize;

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
}
