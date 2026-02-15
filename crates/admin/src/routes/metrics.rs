use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use prometheus::Encoder;

use crate::state::SharedState;

/// GET /api/metrics
///
/// Returns all registered Prometheus metrics in the standard text exposition format.
pub async fn get_metrics(State(state): State<SharedState>) -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = state.metrics.registry.gather();

    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => {
            let body = String::from_utf8(buffer).unwrap_or_default();
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                body,
            )
        }
        Err(e) => {
            tracing::error!("failed to encode prometheus metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain; charset=utf-8")],
                format!("failed to encode metrics: {}", e),
            )
        }
    }
}
