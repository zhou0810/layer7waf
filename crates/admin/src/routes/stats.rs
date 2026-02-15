use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::state::SharedState;

/// GET /api/stats
///
/// Returns aggregated traffic statistics derived from Prometheus counters
/// and the server's uptime.
pub async fn get_stats(State(state): State<SharedState>) -> Json<Value> {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let total_requests = state.metrics.requests_total.get() as u64;
    let blocked_requests = state.metrics.requests_blocked.get() as u64;
    let rate_limited_requests = state.metrics.rate_limited_total.get() as u64;

    let requests_per_second = if uptime_secs > 0 {
        total_requests as f64 / uptime_secs as f64
    } else {
        0.0
    };

    Json(json!({
        "total_requests": total_requests,
        "blocked_requests": blocked_requests,
        "rate_limited_requests": rate_limited_requests,
        "uptime_secs": uptime_secs,
        "requests_per_second": requests_per_second
    }))
}
