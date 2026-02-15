use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::SharedState;

#[derive(Serialize)]
pub struct ScrapingStatsResponse {
    pub scrapers_blocked: u64,
    pub traps_triggered: u64,
    pub captchas_issued: u64,
    pub captchas_solved: u64,
    pub responses_obfuscated: u64,
    pub captcha_pass_rate: f64,
}

pub async fn get_scraping_stats(State(state): State<SharedState>) -> Json<ScrapingStatsResponse> {
    let scrapers_blocked = state.metrics.scrapers_blocked.get();
    let traps_triggered = state.metrics.traps_triggered.get();
    let captchas_issued = state.metrics.captchas_issued.get();
    let captchas_solved = state.metrics.captchas_solved.get();
    let responses_obfuscated = state.metrics.responses_obfuscated.get();

    let captcha_pass_rate = if captchas_issued > 0 {
        captchas_solved as f64 / captchas_issued as f64
    } else {
        0.0
    };

    Json(ScrapingStatsResponse {
        scrapers_blocked,
        traps_triggered,
        captchas_issued,
        captchas_solved,
        responses_obfuscated,
        captcha_pass_rate,
    })
}
