use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::state::SharedState;

/// GET /api/health
///
/// Returns the current health status of the WAF, including uptime and version.
pub async fn health_check(State(state): State<SharedState>) -> Json<Value> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(json!({
        "status": "healthy",
        "uptime_secs": uptime,
        "version": "0.1.0"
    }))
}
