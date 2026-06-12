//! # platform-ratelimit
//!
//! Per-tenant application-layer rate limiting (`SECURITY.md §5.2`).
//!
//! The edge (ingress) rate-limits per client IP, but the **tenant** is only known
//! after verifying the JWT — so per-tenant limiting belongs in the app. This is a
//! token-bucket limiter keyed by `tenant_id`, applied as an Axum middleware that
//! reuses the service's existing [`JwtVerifier`].
//!
//! Apply it (outermost, so limited requests are rejected before any work):
//!
//! ```ignore
//! let limiter = TenantRateLimiter::from_env(state.verifier.clone(), service_name);
//! Router::new()
//!     /* ...routes... */
//!     .layer(axum::middleware::from_fn_with_state(limiter, tenant_rate_limit))
//! ```
//!
//! Scope & limits:
//! - Only **authenticated** requests are limited (those carrying a valid bearer
//!   token); public/unauthenticated requests pass through (the edge throttles
//!   `/auth/login` per IP, and the auth layer rejects bad tokens).
//! - Buckets are **in-memory per pod** (best-effort): with N replicas the effective
//!   limit is ~N× the configured rate. For a strict global limit, back this with a
//!   shared store (Redis); the middleware boundary stays the same.
//! - The verifier runs here and again in the per-request auth extractor (one extra
//!   RS256 verification per request — cheap, and acceptable for v1).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, RETRY_AFTER};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use platform_auth::JwtVerifier;

/// Token-bucket parameters, applied per tenant.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Sustained refill rate (tokens per second). `<= 0` disables limiting.
    pub rps: f64,
    /// Bucket capacity (max burst).
    pub burst: f64,
}

impl RateLimitConfig {
    /// From env: `RATE_LIMIT_RPS` (default 50), `RATE_LIMIT_BURST` (default 2×rps).
    /// Set `RATE_LIMIT_RPS=0` to disable.
    pub fn from_env() -> Self {
        let rps = env_f64("RATE_LIMIT_RPS", 50.0);
        let burst = env_f64("RATE_LIMIT_BURST", (rps * 2.0).max(1.0));
        Self { rps, burst }
    }
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

/// The per-tenant token buckets (the testable core — no JWT/HTTP here).
struct Buckets {
    config: RateLimitConfig,
    map: Mutex<HashMap<String, Bucket>>,
}

impl Buckets {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            map: Mutex::new(HashMap::new()),
        }
    }

    /// Try to consume one token for `tenant`. `Ok(())` if allowed, `Err(secs)` with
    /// a suggested `Retry-After` if limited.
    fn check(&self, tenant: &str, now: Instant) -> Result<(), u64> {
        if self.config.rps <= 0.0 {
            return Ok(()); // disabled
        }
        let mut map = self.map.lock().unwrap();
        let bucket = map.entry(tenant.to_string()).or_insert(Bucket {
            tokens: self.config.burst,
            last: now,
        });
        let elapsed = now.saturating_duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.config.rps).min(self.config.burst);
        bucket.last = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            let deficit = 1.0 - bucket.tokens;
            Err(((deficit / self.config.rps).ceil() as u64).max(1))
        }
    }
}

/// Per-tenant token-bucket limiter. Construct once per service and share via the
/// layer state.
pub struct TenantRateLimiter {
    verifier: Arc<JwtVerifier>,
    service: &'static str,
    buckets: Buckets,
}

impl TenantRateLimiter {
    pub fn new(
        verifier: Arc<JwtVerifier>,
        config: RateLimitConfig,
        service: &'static str,
    ) -> Arc<Self> {
        Arc::new(Self {
            verifier,
            service,
            buckets: Buckets::new(config),
        })
    }

    /// Convenience: build from `RateLimitConfig::from_env()`.
    pub fn from_env(verifier: Arc<JwtVerifier>, service: &'static str) -> Arc<Self> {
        Self::new(verifier, RateLimitConfig::from_env(), service)
    }

    /// The tenant of a request's bearer token, if it carries a valid one.
    fn tenant_of(&self, req: &Request) -> Option<String> {
        let header = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
        let token = header.strip_prefix("Bearer ")?.trim();
        if token.is_empty() {
            return None;
        }
        self.verifier.verify(token).ok().map(|c| c.tenant_id)
    }
}

/// Axum middleware: rate-limit authenticated requests per tenant.
pub async fn tenant_rate_limit(
    State(limiter): State<Arc<TenantRateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(tenant) = limiter.tenant_of(&req) {
        if let Err(retry) = limiter.buckets.check(&tenant, Instant::now()) {
            metrics::counter!("rate_limited_total", "service" => limiter.service).increment(1);
            let body = axum::Json(serde_json::json!({
                "error_code": "RATE_LIMITED",
                "message": "per-tenant rate limit exceeded",
                "details": null,
            }));
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(RETRY_AFTER, retry.to_string())],
                body,
            )
                .into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn buckets(rps: f64, burst: f64) -> Buckets {
        Buckets::new(RateLimitConfig { rps, burst })
    }

    #[test]
    fn allows_within_burst_then_blocks() {
        let b = buckets(1.0, 3.0);
        let t0 = Instant::now();
        assert!(b.check("t1", t0).is_ok());
        assert!(b.check("t1", t0).is_ok());
        assert!(b.check("t1", t0).is_ok());
        assert!(b.check("t1", t0).is_err()); // 4th in the same instant
    }

    #[test]
    fn refills_over_time() {
        let b = buckets(10.0, 2.0);
        let t0 = Instant::now();
        assert!(b.check("t1", t0).is_ok());
        assert!(b.check("t1", t0).is_ok());
        assert!(b.check("t1", t0).is_err());
        // After 0.5s at 10 rps, ~5 tokens have refilled.
        assert!(b.check("t1", t0 + Duration::from_millis(500)).is_ok());
    }

    #[test]
    fn tenants_are_independent() {
        let b = buckets(1.0, 1.0);
        let t0 = Instant::now();
        assert!(b.check("a", t0).is_ok());
        assert!(b.check("a", t0).is_err());
        assert!(b.check("b", t0).is_ok()); // b has its own full bucket
    }

    #[test]
    fn rps_zero_disables() {
        let b = buckets(0.0, 0.0);
        let t0 = Instant::now();
        for _ in 0..1000 {
            assert!(b.check("t1", t0).is_ok());
        }
    }
}
