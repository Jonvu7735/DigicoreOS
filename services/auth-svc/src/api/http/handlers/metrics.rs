//! Prometheus scrape endpoint (`GET /metrics`, OBSERVABILITY.md §4.2).
//! Internal only – must NOT be exposed through the public API gateway.

use axum::extract::State;

use crate::bootstrap::wiring::AppState;

pub async fn render(State(state): State<AppState>) -> String {
    state.metrics.render()
}
