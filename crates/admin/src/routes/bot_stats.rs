use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::SharedState;

#[derive(Serialize)]
pub struct BotStatsResponse {
    pub bots_detected: u64,
    pub challenges_issued: u64,
    pub challenges_solved: u64,
    pub challenge_pass_rate: f64,
}

pub async fn get_bot_stats(State(state): State<SharedState>) -> Json<BotStatsResponse> {
    let bots_detected = state.metrics.bots_detected.get();
    let challenges_issued = state.metrics.challenges_issued.get();
    let challenges_solved = state.metrics.challenges_solved.get();

    let challenge_pass_rate = if challenges_issued > 0 {
        challenges_solved as f64 / challenges_issued as f64
    } else {
        0.0
    };

    Json(BotStatsResponse {
        bots_detected,
        challenges_issued,
        challenges_solved,
        challenge_pass_rate,
    })
}
