//! Environment-based configuration for auth-svc.
//!
//! Every value has a safe **local dev** default so `cargo run` works against
//! `deploy/docker-compose.dev.yml`. Production values are injected via
//! Kubernetes env/secrets (SECURITY.md: secrets never live in code or git).

use anyhow::Context;

pub const SERVICE_NAME: &str = "auth-svc";

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Stable service identifier used in logs/metrics/traces (OBSERVABILITY.md).
    pub service_name: &'static str,
    /// Deployment environment: `dev` | `staging` | `prod`.
    pub env: String,
    /// HTTP port the service binds to.
    pub http_port: u16,
    /// Postgres connection string (shared instance, DATA-STRATEGY.md §3.1).
    pub database_url: String,
    /// Dedicated schema for this service. auth-svc owns ONLY `auth_svc`;
    /// cross-schema access to other services' data is forbidden.
    pub database_schema: String,
    /// NATS URL for the event bus. `None` disables publishing (dev fallback).
    pub nats_url: Option<String>,
    pub jwt: JwtConfig,
}

/// JWT settings (AUTH-FLOW.md §3: claims `sub`, `tenant_id`, `roles`, `iat`,
/// `exp`, `iss`, `aud`).
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub issuer: String,
    pub audience: String,
    /// Access-token TTL in seconds (short-lived, e.g. 15–30 min).
    pub access_ttl_secs: i64,
    /// Refresh-token TTL in seconds (long-lived, e.g. 7–30 days).
    pub refresh_ttl_secs: i64,
    /// HS256 secret for the skeleton phase.
    /// TODO(Phase 1.3): switch to RS256 key pair per AUTH-FLOW.md §3 so the
    /// gateway and other services verify with a public key only.
    pub hs256_secret: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            service_name: SERVICE_NAME,
            env: env_or("APP_ENV", "dev"),
            http_port: env_or("HTTP_PORT", "8081")
                .parse()
                .context("HTTP_PORT must be a valid u16")?,
            database_url: env_or(
                "DATABASE_URL",
                "postgres://digicore:digicore@localhost:5432/digicore",
            ),
            database_schema: env_or("DATABASE_SCHEMA", "auth_svc"),
            nats_url: std::env::var("NATS_URL").ok().filter(|v| !v.is_empty()),
            jwt: JwtConfig {
                issuer: env_or("JWT_ISSUER", "auth-svc"),
                audience: env_or("JWT_AUDIENCE", "platform-api"),
                access_ttl_secs: env_or("JWT_ACCESS_TTL_SECS", "1800")
                    .parse()
                    .context("JWT_ACCESS_TTL_SECS must be i64")?,
                refresh_ttl_secs: env_or("JWT_REFRESH_TTL_SECS", "1209600") // 14 days
                    .parse()
                    .context("JWT_REFRESH_TTL_SECS must be i64")?,
                hs256_secret: env_or("JWT_HS256_SECRET", "dev-only-insecure-secret"),
            },
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
