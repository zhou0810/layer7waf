use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::time::Instant;

/// Per-IP session tracking for scraping detection.
#[derive(Debug, Clone)]
pub struct ScrapingSession {
    pub first_seen: Instant,
    pub last_seen: Instant,
    pub request_count: u64,
    pub unique_path_count: u64,
    path_hashes: HashSet<u64>,
    pub trap_triggered: bool,
    pub captcha_solved: bool,
    pub scraping_score: f64,
}

impl ScrapingSession {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            first_seen: now,
            last_seen: now,
            request_count: 0,
            unique_path_count: 0,
            path_hashes: HashSet::new(),
            trap_triggered: false,
            captcha_solved: false,
            scraping_score: 0.0,
        }
    }

    /// Record a new request and recalculate the scraping score.
    pub fn record_request(&mut self, path: &str, bot_score: f64) {
        self.request_count += 1;
        self.last_seen = Instant::now();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        let path_hash = hasher.finish();
        if self.path_hashes.insert(path_hash) {
            self.unique_path_count += 1;
        }

        self.scraping_score = self.compute_score(bot_score);
    }

    fn compute_score(&self, bot_score: f64) -> f64 {
        let mut score = 0.0;

        // Trap triggered is a strong signal
        if self.trap_triggered {
            score += 1.0;
        }

        // High request rate (more than 60 requests per minute)
        let elapsed = self.last_seen.duration_since(self.first_seen).as_secs_f64();
        if elapsed > 0.0 {
            let rps = self.request_count as f64 / elapsed;
            if rps > 1.0 {
                score += 0.3;
            }
        }

        // High unique path count (crawling many pages)
        if self.unique_path_count > 20 {
            score += 0.2;
        }

        // Factor in bot detection score
        score += bot_score * 0.3;

        // CAPTCHA solved reduces score
        if self.captcha_solved {
            score -= 0.5;
        }

        score.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session_score_zero() {
        let session = ScrapingSession::new();
        assert_eq!(session.scraping_score, 0.0);
        assert_eq!(session.request_count, 0);
    }

    #[test]
    fn test_record_request_increments_count() {
        let mut session = ScrapingSession::new();
        session.record_request("/page1", 0.0);
        assert_eq!(session.request_count, 1);
        assert_eq!(session.unique_path_count, 1);
    }

    #[test]
    fn test_duplicate_paths_not_counted() {
        let mut session = ScrapingSession::new();
        session.record_request("/page1", 0.0);
        session.record_request("/page1", 0.0);
        assert_eq!(session.request_count, 2);
        assert_eq!(session.unique_path_count, 1);
    }

    #[test]
    fn test_trap_triggered_raises_score() {
        let mut session = ScrapingSession::new();
        session.trap_triggered = true;
        session.record_request("/trap", 0.0);
        assert!(session.scraping_score >= 1.0);
    }

    #[test]
    fn test_captcha_solved_reduces_score() {
        let mut session = ScrapingSession::new();
        session.captcha_solved = true;
        session.record_request("/page", 0.5);
        // bot_score * 0.3 = 0.15, captcha -0.5 → clamped to 0.0
        assert!(session.scraping_score < 0.2);
    }

    #[test]
    fn test_bot_score_contributes() {
        let mut session = ScrapingSession::new();
        session.record_request("/page", 1.0);
        // bot_score * 0.3 = 0.3
        assert!(session.scraping_score >= 0.3);
    }
}
