pub mod routes;
pub mod state;

use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use crate::state::SharedState;

pub use state::{AppState, AuditLogEntry, SharedState as SharedStateType, WafMetrics};

/// Build the Axum router with all admin API routes and middleware.
pub fn build_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/api/health", get(routes::health::health_check))
        // Prometheus metrics
        .route("/api/metrics", get(routes::metrics::get_metrics))
        // Configuration management
        .route(
            "/api/config",
            get(routes::config::get_config).put(routes::config::update_config),
        )
        // WAF rules management
        .route(
            "/api/rules",
            get(routes::rules::list_rules).post(routes::rules::add_rule),
        )
        .route("/api/rules/test", post(routes::rules::test_rule))
        .route("/api/rules/{id}", delete(routes::rules::delete_rule))
        // Audit logs
        .route("/api/logs", get(routes::logs::get_logs))
        // Traffic statistics
        .route("/api/stats", get(routes::stats::get_stats))
        // Attach shared state and middleware
        .with_state(state)
        .layer(cors)
}

/// Start the admin API server on the specified address.
///
/// This function will block until the server is shut down.
pub async fn run_admin_server(state: SharedState, listen_addr: &str) -> anyhow::Result<()> {
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    tracing::info!("admin API server listening on {}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Convenience function to create a SharedState from an AppConfig.
pub fn new_shared_state(config: layer7waf_common::AppConfig) -> SharedState {
    Arc::new(AppState::new(config))
}
