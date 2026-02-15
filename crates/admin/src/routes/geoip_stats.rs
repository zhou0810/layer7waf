use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::SharedState;

#[derive(Serialize)]
pub struct GeoIpStatsResponse {
    pub geoip_blocked: u64,
    pub geoip_lookups: u64,
    pub enabled: bool,
    pub blocked_countries: Vec<String>,
    pub allowed_countries: Vec<String>,
}

pub async fn get_geoip_stats(State(state): State<SharedState>) -> Json<GeoIpStatsResponse> {
    let geoip_blocked = state.metrics.geoip_blocked.get();
    let geoip_lookups = state.metrics.geoip_lookups.get();

    let config = state.config.read().expect("config lock poisoned");
    let enabled = config.geoip.enabled;
    let blocked_countries = config.geoip.blocked_countries.clone();
    let allowed_countries = config.geoip.allowed_countries.clone();

    Json(GeoIpStatsResponse {
        geoip_blocked,
        geoip_lookups,
        enabled,
        blocked_countries,
        allowed_countries,
    })
}
