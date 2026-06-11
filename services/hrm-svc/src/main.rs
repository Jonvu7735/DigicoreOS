//! hrm-svc entrypoint.
//!
//! Responsibilities (ARCHITECTURE.md §3.4): employees, attendance, leave, org
//! structure. Boot order: load config -> init observability -> wire
//! dependencies -> build router -> serve with graceful shutdown.

// Skeleton phase: scaffold (config, wiring, auth, outbox relay) is in place;
// domain slices (employees, attendance, ...) fill in without restructuring.
#![allow(dead_code)]

mod api;
mod bootstrap;
mod domain;
mod infra;
mod utils;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = bootstrap::config::AppConfig::from_env().context("failed to load AppConfig")?;

    // Hold the guard until `main` returns so OTLP spans flush on exit.
    let _tracing_guard = utils::logging::init(&config);

    let state = bootstrap::wiring::build_app_state(config)
        .await
        .context("failed to build AppState")?;

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], state.config.http_port));
    let router = bootstrap::wiring::build_router(state);

    tracing::info!(%addr, "hrm-svc listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    tracing::info!("hrm-svc shut down cleanly");
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
