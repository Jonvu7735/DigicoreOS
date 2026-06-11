//! Logging bootstrap – thin wrapper over `platform-observability` so every
//! service initializes identically (OBSERVABILITY.md).

use crate::bootstrap::config::AppConfig;

/// Initialize observability; hold the returned guard for the program's lifetime
/// so OTLP spans are flushed on shutdown.
pub fn init(config: &AppConfig) -> platform_observability::TracingGuard {
    platform_observability::init_tracing(config.service_name, &config.env)
}
