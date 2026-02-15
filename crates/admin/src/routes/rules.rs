use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::state::SharedState;

/// GET /api/rules
///
/// Returns the list of configured WAF rule files from the config
/// plus any custom rules added at runtime.
pub async fn list_rules(State(state): State<SharedState>) -> Json<Value> {
    let config = state.config.read().expect("config lock poisoned");
    let custom_rules = state.custom_rules.read().expect("custom_rules lock poisoned");

    Json(json!({
        "rule_files": config.waf.rules,
        "custom_rules": custom_rules.iter().enumerate().map(|(i, r)| {
            json!({ "id": i, "rule": r })
        }).collect::<Vec<Value>>()
    }))
}

/// Request body for adding a new custom rule.
#[derive(Debug, Deserialize)]
pub struct AddRuleRequest {
    pub rule: String,
}

/// POST /api/rules
///
/// Adds a custom WAF rule string (e.g. "SecRule ...") to the in-memory list.
pub async fn add_rule(
    State(state): State<SharedState>,
    Json(body): Json<AddRuleRequest>,
) -> impl IntoResponse {
    if body.rule.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": "rule must not be empty"
            })),
        );
    }

    let mut custom_rules = state.custom_rules.write().expect("custom_rules lock poisoned");
    let id = custom_rules.len();
    custom_rules.push(body.rule.clone());

    tracing::info!("custom rule added at index {}: {}", id, body.rule);

    (
        StatusCode::CREATED,
        Json(json!({
            "status": "created",
            "id": id,
            "rule": body.rule
        })),
    )
}

/// DELETE /api/rules/:id
///
/// Removes a custom rule by its index. Returns 404 if the index is out of range.
pub async fn delete_rule(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    let mut custom_rules = state.custom_rules.write().expect("custom_rules lock poisoned");

    if id >= custom_rules.len() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "error",
                "message": format!("rule with id {} not found", id)
            })),
        );
    }

    let removed = custom_rules.remove(id);
    tracing::info!("custom rule removed at index {}: {}", id, removed);

    (
        StatusCode::OK,
        Json(json!({
            "status": "deleted",
            "id": id,
            "rule": removed
        })),
    )
}

/// Request body for testing a rule against a synthetic request.
#[derive(Debug, Deserialize)]
pub struct TestRuleRequest {
    pub rule: String,
    pub request: TestRequestData,
}

/// Synthetic request data used when testing a rule.
#[derive(Debug, Deserialize)]
pub struct TestRequestData {
    pub method: String,
    pub uri: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

/// POST /api/rules/test
///
/// Tests a WAF rule against a synthetic request. This is a stub implementation
/// that returns a placeholder response indicating whether the rule would match.
pub async fn test_rule(
    Json(body): Json<TestRuleRequest>,
) -> Json<Value> {
    tracing::info!(
        "testing rule against {} {}",
        body.request.method,
        body.request.uri
    );

    // Stub implementation: in a real system this would invoke the Coraza engine
    // to evaluate the rule against the synthetic request.
    Json(json!({
        "matched": false,
        "rule": body.rule,
        "request": {
            "method": body.request.method,
            "uri": body.request.uri
        },
        "message": "stub: rule evaluation not yet implemented"
    }))
}
