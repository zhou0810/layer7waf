pub mod fingerprint;
pub mod js_challenge;
pub mod known_bots;
pub mod score;

use dashmap::DashMap;
use layer7waf_common::BotDetectionConfig;
use std::time::Instant;

use fingerprint::compute_fingerprint;
use js_challenge::{extract_challenge_cookie, verify_challenge_cookie};
use known_bots::classify_user_agent;
use score::compute_bot_score;

/// Result of a bot detection check.
#[derive(Debug)]
pub enum BotCheckResult {
    /// Request is allowed to proceed.
    Allow,
    /// Request should be blocked (bot score exceeded threshold).
    Block,
    /// Request should be challenged — return the HTML page to the client.
    Challenge(String),
    /// Detection-only mode: request proceeds but score is recorded.
    Detect { score: f64 },
}

/// Per-IP session tracking entry.
#[derive(Debug, Clone)]
struct BotSession {
    last_seen: Instant,
    fingerprint_hash: String,
}

/// Bot detection engine wrapping all sub-modules.
pub struct BotDetector {
    config: BotDetectionConfig,
    sessions: DashMap<String, BotSession>,
}

impl BotDetector {
    /// Create a new BotDetector from the given configuration.
    pub fn new(config: BotDetectionConfig) -> Self {
        Self {
            config,
            sessions: DashMap::new(),
        }
    }

    /// Perform a bot detection check on the incoming request.
    ///
    /// # Arguments
    /// - `client_ip`: The client's IP address as a string.
    /// - `headers`: Request headers as (name, value) pairs in order.
    /// - `method`: HTTP method (GET, POST, etc.).
    /// - `cookie_header`: The raw `Cookie` header value, if present.
    pub fn check(
        &self,
        client_ip: &str,
        headers: &[(String, String)],
        method: &str,
        cookie_header: Option<&str>,
    ) -> BotCheckResult {
        if !self.config.enabled {
            return BotCheckResult::Allow;
        }

        // 1. Compute HTTP fingerprint
        let fp = compute_fingerprint(headers, method);

        // 2. Classify User-Agent
        let ua = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("user-agent"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        let bot_pattern = classify_user_agent(ua, &self.config.known_bots_allowlist);

        // 3. Check JS challenge cookie
        let has_valid_challenge = cookie_header
            .and_then(extract_challenge_cookie)
            .map(|cookie| {
                verify_challenge_cookie(
                    &cookie,
                    client_ip,
                    &self.config.js_challenge.secret,
                    self.config.js_challenge.ttl_secs,
                )
            })
            .unwrap_or(false);

        // 4. Compute composite score
        let bot_score = compute_bot_score(&fp, bot_pattern, has_valid_challenge, headers);

        // 5. Track session
        self.sessions.insert(
            client_ip.to_string(),
            BotSession {
                last_seen: Instant::now(),
                fingerprint_hash: fp.header_order_hash.clone(),
            },
        );

        // 6. Known good bots always pass
        if bot_pattern == known_bots::BotPattern::KnownGoodBot {
            return BotCheckResult::Allow;
        }

        // 7. Apply mode-specific logic
        if bot_score >= self.config.score_threshold {
            match self.config.mode {
                layer7waf_common::BotDetectionMode::Block => BotCheckResult::Block,
                layer7waf_common::BotDetectionMode::Challenge => {
                    if has_valid_challenge {
                        // Already passed challenge, allow through
                        BotCheckResult::Allow
                    } else if self.config.js_challenge.enabled {
                        let html = js_challenge::generate_challenge(
                            client_ip,
                            self.config.js_challenge.difficulty,
                            &self.config.js_challenge.secret,
                        );
                        BotCheckResult::Challenge(html)
                    } else {
                        BotCheckResult::Block
                    }
                }
                layer7waf_common::BotDetectionMode::Detect => {
                    BotCheckResult::Detect { score: bot_score }
                }
            }
        } else {
            match self.config.mode {
                layer7waf_common::BotDetectionMode::Detect => {
                    BotCheckResult::Detect { score: bot_score }
                }
                _ => BotCheckResult::Allow,
            }
        }
    }

    /// Remove stale session entries older than the given duration.
    pub fn cleanup_sessions(&self, max_age: std::time::Duration) {
        let now = Instant::now();
        self.sessions
            .retain(|_, session| now.duration_since(session.last_seen) < max_age);
    }

    /// Return the number of tracked sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layer7waf_common::{BotDetectionConfig, BotDetectionMode, JsChallengeConfig};

    fn test_config(mode: BotDetectionMode) -> BotDetectionConfig {
        BotDetectionConfig {
            enabled: true,
            mode,
            js_challenge: JsChallengeConfig {
                enabled: true,
                difficulty: 16,
                ttl_secs: 3600,
                secret: "test-secret".to_string(),
            },
            score_threshold: 0.7,
            known_bots_allowlist: vec![],
        }
    }

    fn browser_headers() -> Vec<(String, String)> {
        vec![
            ("Host".into(), "example.com".into()),
            (
                "User-Agent".into(),
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0".into(),
            ),
            ("Accept".into(), "text/html,application/xhtml+xml".into()),
            ("Accept-Encoding".into(), "gzip, deflate, br".into()),
            ("Accept-Language".into(), "en-US,en;q=0.9".into()),
        ]
    }

    fn curl_headers() -> Vec<(String, String)> {
        vec![
            ("Host".into(), "example.com".into()),
            ("User-Agent".into(), "curl/7.88.1".into()),
            ("Accept".into(), "*/*".into()),
        ]
    }

    #[test]
    fn test_disabled_detector_allows_all() {
        let mut config = test_config(BotDetectionMode::Block);
        config.enabled = false;
        let detector = BotDetector::new(config);
        let result = detector.check("1.2.3.4", &curl_headers(), "GET", None);
        assert!(matches!(result, BotCheckResult::Allow));
    }

    #[test]
    fn test_browser_request_allowed() {
        let detector = BotDetector::new(test_config(BotDetectionMode::Block));
        let result = detector.check("1.2.3.4", &browser_headers(), "GET", None);
        assert!(matches!(result, BotCheckResult::Allow));
    }

    #[test]
    fn test_curl_blocked_in_block_mode() {
        let detector = BotDetector::new(test_config(BotDetectionMode::Block));
        let result = detector.check("1.2.3.4", &curl_headers(), "GET", None);
        assert!(matches!(result, BotCheckResult::Block));
    }

    #[test]
    fn test_curl_challenged_in_challenge_mode() {
        let detector = BotDetector::new(test_config(BotDetectionMode::Challenge));
        let result = detector.check("1.2.3.4", &curl_headers(), "GET", None);
        assert!(matches!(result, BotCheckResult::Challenge(_)));
    }

    #[test]
    fn test_curl_detected_in_detect_mode() {
        let detector = BotDetector::new(test_config(BotDetectionMode::Detect));
        let result = detector.check("1.2.3.4", &curl_headers(), "GET", None);
        match result {
            BotCheckResult::Detect { score } => assert!(score >= 0.7),
            other => panic!("expected Detect, got {:?}", other),
        }
    }

    #[test]
    fn test_googlebot_always_allowed() {
        let detector = BotDetector::new(test_config(BotDetectionMode::Block));
        let headers = vec![
            ("Host".into(), "example.com".into()),
            (
                "User-Agent".into(),
                "Mozilla/5.0 (compatible; Googlebot/2.1)".into(),
            ),
        ];
        let result = detector.check("66.249.66.1", &headers, "GET", None);
        assert!(matches!(result, BotCheckResult::Allow));
    }

    #[test]
    fn test_session_tracking() {
        let detector = BotDetector::new(test_config(BotDetectionMode::Detect));
        assert_eq!(detector.session_count(), 0);
        detector.check("1.2.3.4", &browser_headers(), "GET", None);
        assert_eq!(detector.session_count(), 1);
        detector.check("5.6.7.8", &browser_headers(), "GET", None);
        assert_eq!(detector.session_count(), 2);
    }
}
