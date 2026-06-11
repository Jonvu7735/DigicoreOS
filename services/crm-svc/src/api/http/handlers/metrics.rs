//! Prometheus scrape endpoint (`GET /metrics`). Internal only.

use axum::extract::State;

use crate::bootstrap::wiring::AppState;

pub async fn render(State(state): State<AppState>) -> String {
    state.metrics.render()
}
