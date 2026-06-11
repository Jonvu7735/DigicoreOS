//! auth-svc entrypoint.
//!
//! Responsibilities (SERVICE-auth-svc.md): multi-tenant authentication,
//! user/tenant/role/permission management (RBAC), JWT issuance & validation.
//!
//! Boot order: load config -> init observability -> wire dependencies (DI) ->
//! build router -> serve with graceful shutdown.

// Skeleton phase: several types/ports are declared ahead of their first use so
// AI agents can fill in Phase 1.2+ without restructuring. Remove this once the
// login/refresh/logout flows are implemented.
#![allow(dead_code)]

mod api;
mod bootstrap;
mod domain;
mod infra;
mod utils;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Config from environment (bootstrap/config.rs).
    let config = bootstrap::config::AppConfig::from_env().context("failed to load AppConfig")?;

    // 2. Observability first, so wiring failures are visible (OBSERVABILITY.md).
    utils::logging::init(&config);

    // 3. Dependency wiring (bootstrap/wiring.rs).
    let state = bootstrap::wiring::build_app_state(config)
        .await
        .context("failed to build AppState")?;

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], state.config.http_port));
    let router = bootstrap::wiring::build_router(state);

    tracing::info!(%addr, "auth-svc listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    tracing::info!("auth-svc shut down cleanly");
    Ok(())
}

/// Resolve on SIGINT/SIGTERM so Kubernetes rollouts drain connections cleanly.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
