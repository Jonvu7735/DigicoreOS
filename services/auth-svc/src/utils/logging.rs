//! Logging bootstrap – thin wrapper over `platform-observability` so every
//! service initializes identically (OBSERVABILITY.md).

use crate::bootstrap::config::AppConfig;

pub fn init(config: &AppConfig) {
    platform_observability::init_tracing(config.service_name, &config.env);
}
