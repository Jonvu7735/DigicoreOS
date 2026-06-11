//! Environment-based configuration for crm-svc.
//!
//! Every value has a safe local-dev default so `cargo run` works against
//! `deploy/docker-compose.dev.yml`. Production values come from env/secrets.

use anyhow::Context;

pub const SERVICE_NAME: &str = "crm-svc";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub service_name: &'static str,
    /// Deployment environment: `dev` | `staging` | `prod`.
    pub env: String,
    pub http_port: u16,
    /// Shared Postgres instance (DATA-STRATEGY.md §3.1).
    pub database_url: String,
    /// Dedicated schema; this service owns ONLY `crm_svc`.
    pub database_schema: String,
    /// NATS URL for the event bus; `None` disables the relay (dev fallback).
    pub nats_url: Option<String>,
    pub jwt: JwtConfig,
}

/// JWT verification settings. CRM only VERIFIES the tokens auth-svc issues, so
/// it holds the public key (AUTH-FLOW.md §7).
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub public_key_pem: String,
    pub issuer: String,
    pub audience: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let env = env_or("APP_ENV", "dev");
        Ok(Self {
            service_name: SERVICE_NAME,
            env: env.clone(),
            http_port: env_or("HTTP_PORT", "8083")
                .parse()
                .context("HTTP_PORT must be a valid u16")?,
            database_url: env_or(
                "DATABASE_URL",
                "postgres://digicore:digicore@localhost:5432/digicore",
            ),
            database_schema: env_or("DATABASE_SCHEMA", "crm_svc"),
            nats_url: std::env::var("NATS_URL").ok().filter(|v| !v.is_empty()),
            jwt: JwtConfig {
                public_key_pem: load_public_key(&env)?,
                issuer: env_or("JWT_ISSUER", "auth-svc"),
                audience: env_or("JWT_AUDIENCE", "platform-api"),
            },
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Resolve the RS256 *public* key (PEM): `JWT_PUBLIC_KEY_PEM`, then
/// `JWT_PUBLIC_KEY_PATH`, then (dev only) `./.dev/jwt_public.pem` produced by
/// `scripts/gen-dev-jwt-keys.sh`.
fn load_public_key(env: &str) -> anyhow::Result<String> {
    if let Some(pem) = std::env::var("JWT_PUBLIC_KEY_PEM")
        .ok()
        .filter(|v| !v.is_empty())
    {
        return Ok(pem);
    }
    let path = std::env::var("JWT_PUBLIC_KEY_PATH")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| (env == "dev").then(|| ".dev/jwt_public.pem".to_string()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "JWT public key not configured: set JWT_PUBLIC_KEY_PEM or JWT_PUBLIC_KEY_PATH"
            )
        })?;
    std::fs::read_to_string(&path)
        .with_context(|| format!("reading {path} (dev: run scripts/gen-dev-jwt-keys.sh)"))
}
