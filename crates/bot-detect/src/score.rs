use crate::fingerprint::{self, HttpFingerprint};
use crate::known_bots::BotPattern;

/// Compute a composite bot score from multiple signals.
///
/// Returns a value in [0.0, 1.0] where higher values indicate higher likelihood of being a bot.
///
/// Scoring weights:
/// - Known bad bot UA: 0.9
/// - Suspicious UA: 0.5
/// - Missing standard Accept header: +0.2
/// - Valid JS challenge cookie: -0.8 (strong human signal)
/// - Known good bot: 0.0 (trusted)
/// - Likely human with good Accept: 0.1 (baseline)
pub fn compute_bot_score(
    _fingerprint: &HttpFingerprint,
    bot_pattern: BotPattern,
    has_valid_challenge: bool,
    headers: &[(String, String)],
) -> f64 {
    let mut score: f64 = match bot_pattern {
        BotPattern::KnownGoodBot => 0.0,
        BotPattern::KnownBadBot => 0.9,
        BotPattern::Suspicious => 0.5,
        BotPattern::LikelyHuman => 0.1,
    };

    // Penalize missing/unusual Accept header
    if !fingerprint::has_standard_accept(headers) && bot_pattern != BotPattern::KnownGoodBot {
        score += 0.2;
    }

    // Strong human signal: passed JS challenge
    if has_valid_challenge {
        score -= 0.8;
    }

    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_fingerprint() -> HttpFingerprint {
        HttpFingerprint {
            header_order_hash: "abc".into(),
            ua_family: "Chrome".into(),
            accept_hash: "def".into(),
        }
    }

    fn html_headers() -> Vec<(String, String)> {
        vec![("Accept".into(), "text/html".into())]
    }

    fn empty_headers() -> Vec<(String, String)> {
        vec![]
    }

    #[test]
    fn test_known_bad_bot_high_score() {
        let score = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::KnownBadBot,
            false,
            &empty_headers(),
        );
        assert!(score >= 0.9, "known bad bot without accept: {}", score);
    }

    #[test]
    fn test_likely_human_low_score() {
        let score = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::LikelyHuman,
            false,
            &html_headers(),
        );
        assert!(score <= 0.2, "likely human with accept: {}", score);
    }

    #[test]
    fn test_challenge_reduces_score() {
        let without = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::Suspicious,
            false,
            &html_headers(),
        );
        let with = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::Suspicious,
            true,
            &html_headers(),
        );
        assert!(with < without, "challenge should reduce score: {} vs {}", with, without);
    }

    #[test]
    fn test_known_good_bot_zero_score() {
        let score = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::KnownGoodBot,
            false,
            &empty_headers(),
        );
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_clamped() {
        // Even with maximum penalties, should not exceed 1.0
        let score = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::KnownBadBot,
            false,
            &empty_headers(),
        );
        assert!(score <= 1.0);

        // Even with maximum bonus, should not go below 0.0
        let score = compute_bot_score(
            &dummy_fingerprint(),
            BotPattern::KnownGoodBot,
            true,
            &html_headers(),
        );
        assert!(score >= 0.0);
    }
}
