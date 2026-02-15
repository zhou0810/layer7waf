use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use layer7waf_common::AppConfig;
use serde_json::json;

use crate::state::SharedState;

/// GET /api/config
///
/// Returns the current WAF configuration as JSON.
pub async fn get_config(State(state): State<SharedState>) -> impl IntoResponse {
    let config = state.config.read().expect("config lock poisoned");
    Json(serde_json::to_value(&*config).unwrap_or(json!({"error": "serialization failed"})))
}

/// PUT /api/config
///
/// Accepts a full configuration as JSON, validates it, and replaces
/// the current running configuration.
pub async fn update_config(
    State(state): State<SharedState>,
    Json(new_config): Json<AppConfig>,
) -> impl IntoResponse {
    // Validate the incoming configuration before applying it.
    if let Err(e) = new_config.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": format!("validation failed: {}", e)
            })),
        );
    }

    let mut config = state.config.write().expect("config lock poisoned");
    *config = new_config;

    tracing::info!("configuration updated via admin API");

    (
        StatusCode::OK,
        Json(json!({
            "status": "updated"
        })),
    )
}
