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
    /// PEM-encoded RSA private key used to SIGN access tokens (RS256).
    pub private_key_pem: String,
    /// PEM-encoded RSA public key used to VERIFY access tokens. Distributed to
    /// the gateway/other services so they verify without the private key.
    pub public_key_pem: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let env = env_or("APP_ENV", "dev");
        let (private_key_pem, public_key_pem) = load_rsa_keys(&env)?;
        Ok(Self {
            service_name: SERVICE_NAME,
            env: env.clone(),
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
                private_key_pem,
                public_key_pem,
            },
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Resolve the RS256 key pair (PEM). Precedence:
/// 1. Inline PEM: `JWT_PRIVATE_KEY_PEM` + `JWT_PUBLIC_KEY_PEM` (prod secrets).
/// 2. File paths: `JWT_PRIVATE_KEY_PATH` + `JWT_PUBLIC_KEY_PATH`.
/// 3. Dev only (`APP_ENV=dev`): default paths `./.dev/jwt_private.pem` and
///    `./.dev/jwt_public.pem` (run `scripts/gen-dev-jwt-keys.sh` to create them).
fn load_rsa_keys(env: &str) -> anyhow::Result<(String, String)> {
    let inline_priv = std::env::var("JWT_PRIVATE_KEY_PEM")
        .ok()
        .filter(|v| !v.is_empty());
    let inline_pub = std::env::var("JWT_PUBLIC_KEY_PEM")
        .ok()
        .filter(|v| !v.is_empty());
    if let (Some(priv_pem), Some(pub_pem)) = (inline_priv, inline_pub) {
        return Ok((priv_pem, pub_pem));
    }

    let priv_path = std::env::var("JWT_PRIVATE_KEY_PATH")
        .ok()
        .filter(|v| !v.is_empty());
    let pub_path = std::env::var("JWT_PUBLIC_KEY_PATH")
        .ok()
        .filter(|v| !v.is_empty());
    let (priv_path, pub_path) = match (priv_path, pub_path) {
        (Some(a), Some(b)) => (a, b),
        _ if env == "dev" => (
            ".dev/jwt_private.pem".to_string(),
            ".dev/jwt_public.pem".to_string(),
        ),
        _ => anyhow::bail!(
            "RS256 keys not configured: set JWT_PRIVATE_KEY_PEM/JWT_PUBLIC_KEY_PEM \
             or JWT_PRIVATE_KEY_PATH/JWT_PUBLIC_KEY_PATH"
        ),
    };

    let private_key_pem = std::fs::read_to_string(&priv_path)
        .with_context(|| format!("reading {priv_path} (dev: run scripts/gen-dev-jwt-keys.sh)"))?;
    let public_key_pem =
        std::fs::read_to_string(&pub_path).with_context(|| format!("reading {pub_path}"))?;
    Ok((private_key_pem, public_key_pem))
}
