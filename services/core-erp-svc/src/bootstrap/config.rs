//! Environment-based configuration for core-erp-svc.
//!
//! Every value has a safe local-dev default so `cargo run` works against
//! `deploy/docker-compose.dev.yml`. Production values come from env/secrets.

use anyhow::Context;

pub const SERVICE_NAME: &str = "core-erp-svc";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub service_name: &'static str,
    /// Deployment environment: `dev` | `staging` | `prod`.
    pub env: String,
    pub http_port: u16,
    /// Shared Postgres instance (DATA-STRATEGY.md §3.1).
    pub database_url: String,
    /// Dedicated schema; this service owns ONLY `erp_core_svc`.
    pub database_schema: String,
    /// NATS URL for the event bus; `None` disables the relay (dev fallback).
    pub nats_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            service_name: SERVICE_NAME,
            env: env_or("APP_ENV", "dev"),
            http_port: env_or("HTTP_PORT", "8082")
                .parse()
                .context("HTTP_PORT must be a valid u16")?,
            database_url: env_or(
                "DATABASE_URL",
                "postgres://digicore:digicore@localhost:5432/digicore",
            ),
            database_schema: env_or("DATABASE_SCHEMA", "erp_core_svc"),
            nats_url: std::env::var("NATS_URL").ok().filter(|v| !v.is_empty()),
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
