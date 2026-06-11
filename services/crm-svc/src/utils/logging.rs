//! Logging bootstrap – thin wrapper over `platform-observability`.

use crate::bootstrap::config::AppConfig;

/// Initialize observability; hold the guard for the program's lifetime so OTLP
/// spans flush on shutdown.
pub fn init(config: &AppConfig) -> platform_observability::TracingGuard {
    platform_observability::init_tracing(config.service_name, &config.env)
}
