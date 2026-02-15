use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::SharedState;

/// Query parameters for the audit log endpoint.
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    /// Maximum number of entries to return (default: 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of entries to skip (default: 0).
    #[serde(default)]
    pub offset: usize,
    /// Optional filter by client IP address.
    pub ip: Option<String>,
    /// Optional filter by WAF rule ID.
    pub rule_id: Option<String>,
}

fn default_limit() -> usize {
    100
}

/// GET /api/logs
///
/// Returns a paginated, optionally filtered list of audit log entries
/// from the in-memory ring buffer.
pub async fn get_logs(
    State(state): State<SharedState>,
    Query(params): Query<LogQuery>,
) -> Json<Value> {
    let logs = state.audit_log.read().expect("audit_log lock poisoned");

    // Apply filters.
    let filtered: Vec<_> = logs
        .iter()
        .filter(|entry| {
            if let Some(ref ip) = params.ip {
                if &entry.client_ip != ip {
                    return false;
                }
            }
            if let Some(ref rule_id) = params.rule_id {
                match &entry.rule_id {
                    Some(rid) if rid == rule_id => {}
                    _ => return false,
                }
            }
            true
        })
        .collect();

    let total = filtered.len();

    // Apply pagination.
    let page: Vec<_> = filtered
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .cloned()
        .collect();

    Json(json!({
        "total": total,
        "offset": params.offset,
        "limit": params.limit,
        "entries": page
    }))
}
