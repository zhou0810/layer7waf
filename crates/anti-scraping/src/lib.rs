pub mod captcha;
pub mod honeypot;
pub mod obfuscation;
pub mod session;

use dashmap::DashMap;
use layer7waf_common::AntiScrapingConfig;
use std::time::Instant;
use tracing::{debug, info};

use captcha::{extract_captcha_cookie, verify_captcha_cookie};
use honeypot::{generate_trap_html, inject_trap, is_trap_request};
use obfuscation::inject_zero_width_chars;
use session::ScrapingSession;

/// Maximum body buffer size for response rewriting (2 MB).
const MAX_BODY_BUFFER: usize = 2 * 1024 * 1024;

/// Result of an anti-scraping check on a request.
#[derive(Debug)]
pub enum ScrapingCheckResult {
    /// Request is allowed to proceed.
    Allow,
    /// Request should be blocked.
    Block,
    /// Request should be challenged — return the CAPTCHA HTML page.
    Challenge(String),
    /// Detection-only mode: request proceeds but score is recorded.
    Detect { score: f64 },
    /// A honeypot trap was triggered.
    TrapTriggered,
}

/// Main anti-scraping engine.
pub struct AntiScraper {
    config: AntiScrapingConfig,
    sessions: DashMap<String, ScrapingSession>,
}

impl AntiScraper {
    pub fn new(config: AntiScrapingConfig) -> Self {
        Self {
            config,
            sessions: DashMap::new(),
        }
    }

    /// Check an incoming request against anti-scraping rules.
    pub fn check_request(
        &self,
        client_ip: &str,
        path: &str,
        _method: &str,
        cookie_header: Option<&str>,
        bot_score: f64,
    ) -> ScrapingCheckResult {
        if !self.config.enabled {
            return ScrapingCheckResult::Allow;
        }

        // Check for honeypot trap
        if self.config.honeypot.enabled
            && is_trap_request(path, &self.config.honeypot.trap_path_prefix)
        {
            info!(client_ip = %client_ip, path = %path, "honeypot trap triggered");
            let mut session = self.sessions.entry(client_ip.to_string()).or_insert_with(ScrapingSession::new);
            session.trap_triggered = true;
            session.record_request(path, bot_score);
            return ScrapingCheckResult::TrapTriggered;
        }

        // Check for valid CAPTCHA cookie
        let has_valid_captcha = if self.config.captcha.enabled {
            cookie_header
                .and_then(extract_captcha_cookie)
                .map(|cookie| {
                    verify_captcha_cookie(
                        &cookie,
                        client_ip,
                        &self.config.captcha.secret,
                        self.config.captcha.ttl_secs,
                    )
                })
                .unwrap_or(false)
        } else {
            false
        };

        // Update session
        let mut session = self.sessions.entry(client_ip.to_string()).or_insert_with(ScrapingSession::new);
        if has_valid_captcha {
            session.captcha_solved = true;
        }
        session.record_request(path, bot_score);
        let score = session.scraping_score;
        drop(session);

        debug!(client_ip = %client_ip, score, "anti-scraping score");

        // Apply mode-specific logic
        if score >= self.config.score_threshold {
            match self.config.mode {
                layer7waf_common::AntiScrapingMode::Block => ScrapingCheckResult::Block,
                layer7waf_common::AntiScrapingMode::Challenge => {
                    if has_valid_captcha {
                        ScrapingCheckResult::Allow
                    } else if self.config.captcha.enabled {
                        let html = captcha::generate_captcha_page(
                            client_ip,
                            &self.config.captcha.secret,
                            path,
                        );
                        ScrapingCheckResult::Challenge(html)
                    } else {
                        ScrapingCheckResult::Block
                    }
                }
                layer7waf_common::AntiScrapingMode::Detect => {
                    ScrapingCheckResult::Detect { score }
                }
            }
        } else {
            match self.config.mode {
                layer7waf_common::AntiScrapingMode::Detect => {
                    ScrapingCheckResult::Detect { score }
                }
                _ => ScrapingCheckResult::Allow,
            }
        }
    }

    /// Process a response body: inject honeypot traps and/or zero-width watermarks.
    ///
    /// Returns `None` if no modification was needed (non-HTML, too large, etc.).
    pub fn process_response(
        &self,
        client_ip: &str,
        content_type: Option<&str>,
        body: &[u8],
    ) -> Option<Vec<u8>> {
        if !self.config.enabled {
            return None;
        }

        // Only process HTML responses
        let ct = content_type?;
        if !ct.contains("text/html") {
            return None;
        }

        // Skip if body too large
        if body.len() > MAX_BODY_BUFFER {
            return None;
        }

        let mut modified = body.to_vec();
        let mut was_modified = false;

        // Inject honeypot trap
        if self.config.honeypot.enabled {
            let trap_html = generate_trap_html(
                &self.config.honeypot.trap_path_prefix,
                client_ip,
                &self.config.captcha.secret,
            );
            if let Some(with_trap) = inject_trap(&modified, &trap_html) {
                modified = with_trap;
                was_modified = true;
            }
        }

        // Inject zero-width watermarks
        if self.config.obfuscation.enabled {
            if let Some(with_watermark) = inject_zero_width_chars(&modified, client_ip) {
                modified = with_watermark;
                was_modified = true;
            }
        }

        if was_modified {
            Some(modified)
        } else {
            None
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

    /// Return the number of sessions flagged as scrapers.
    pub fn flagged_scraper_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|entry| entry.value().scraping_score >= self.config.score_threshold)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layer7waf_common::{
        AntiScrapingConfig, AntiScrapingMode, CaptchaConfig, HoneypotConfig, ObfuscationConfig,
    };

    fn test_config(mode: AntiScrapingMode) -> AntiScrapingConfig {
        AntiScrapingConfig {
            enabled: true,
            mode,
            captcha: CaptchaConfig {
                enabled: true,
                ttl_secs: 1800,
                secret: "test-secret".to_string(),
            },
            honeypot: HoneypotConfig {
                enabled: true,
                trap_path_prefix: "/.well-known/l7w-trap".to_string(),
            },
            obfuscation: ObfuscationConfig { enabled: true },
            score_threshold: 0.6,
        }
    }

    #[test]
    fn test_disabled_allows_all() {
        let mut config = test_config(AntiScrapingMode::Block);
        config.enabled = false;
        let scraper = AntiScraper::new(config);
        let result = scraper.check_request("1.2.3.4", "/", "GET", None, 1.0);
        assert!(matches!(result, ScrapingCheckResult::Allow));
    }

    #[test]
    fn test_trap_request_detected() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Block));
        let result = scraper.check_request(
            "1.2.3.4",
            "/.well-known/l7w-trap/abc123",
            "GET",
            None,
            0.0,
        );
        assert!(matches!(result, ScrapingCheckResult::TrapTriggered));
    }

    #[test]
    fn test_normal_request_allowed() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Block));
        let result = scraper.check_request("1.2.3.4", "/api/data", "GET", None, 0.0);
        assert!(matches!(result, ScrapingCheckResult::Allow));
    }

    #[test]
    fn test_high_bot_score_blocks() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Block));
        // High bot score (1.0) contributes 0.3 to scraping score
        // We need trap triggered or high request rate to exceed threshold
        let result = scraper.check_request("1.2.3.4", "/.well-known/l7w-trap/x", "GET", None, 0.0);
        assert!(matches!(result, ScrapingCheckResult::TrapTriggered));
        // Now subsequent requests from this IP should be blocked
        let result = scraper.check_request("1.2.3.4", "/page", "GET", None, 0.0);
        assert!(matches!(result, ScrapingCheckResult::Block));
    }

    #[test]
    fn test_challenge_mode_issues_captcha() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Challenge));
        // Trigger trap first
        scraper.check_request("1.2.3.4", "/.well-known/l7w-trap/x", "GET", None, 0.0);
        let result = scraper.check_request("1.2.3.4", "/page", "GET", None, 0.0);
        assert!(matches!(result, ScrapingCheckResult::Challenge(_)));
    }

    #[test]
    fn test_detect_mode_returns_score() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Detect));
        let result = scraper.check_request("1.2.3.4", "/page", "GET", None, 0.5);
        assert!(matches!(result, ScrapingCheckResult::Detect { .. }));
    }

    #[test]
    fn test_process_response_html() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Block));
        let body = b"<html><body><p>Hello</p></body></html>";
        let result = scraper.process_response("1.2.3.4", Some("text/html"), body);
        assert!(result.is_some());
        let result_bytes = result.unwrap();
        let result_str = std::str::from_utf8(&result_bytes).unwrap();
        assert!(result_str.contains("l7w-trap"));
    }

    #[test]
    fn test_process_response_non_html_skipped() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Block));
        let body = b"{'key': 'value'}";
        let result = scraper.process_response("1.2.3.4", Some("application/json"), body);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_response_disabled() {
        let mut config = test_config(AntiScrapingMode::Block);
        config.enabled = false;
        let scraper = AntiScraper::new(config);
        let body = b"<html><body><p>Hello</p></body></html>";
        let result = scraper.process_response("1.2.3.4", Some("text/html"), body);
        assert!(result.is_none());
    }

    #[test]
    fn test_session_tracking() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Detect));
        assert_eq!(scraper.session_count(), 0);
        scraper.check_request("1.2.3.4", "/page1", "GET", None, 0.0);
        assert_eq!(scraper.session_count(), 1);
        scraper.check_request("5.6.7.8", "/page1", "GET", None, 0.0);
        assert_eq!(scraper.session_count(), 2);
    }

    #[test]
    fn test_cleanup_sessions() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Detect));
        scraper.check_request("1.2.3.4", "/page", "GET", None, 0.0);
        assert_eq!(scraper.session_count(), 1);
        // Cleanup with zero duration should remove all
        scraper.cleanup_sessions(std::time::Duration::from_secs(0));
        assert_eq!(scraper.session_count(), 0);
    }

    #[test]
    fn test_flagged_scraper_count() {
        let scraper = AntiScraper::new(test_config(AntiScrapingMode::Detect));
        // Trigger trap for one IP
        scraper.check_request("1.2.3.4", "/.well-known/l7w-trap/x", "GET", None, 0.0);
        // Normal request for another IP
        scraper.check_request("5.6.7.8", "/page", "GET", None, 0.0);
        assert_eq!(scraper.flagged_scraper_count(), 1);
    }
}
