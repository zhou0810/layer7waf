/// Classification result for a User-Agent string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotPattern {
    /// Known good bot (e.g. Googlebot, Bingbot) — should be allowed.
    KnownGoodBot,
    /// Known bad bot signature (curl, wget, python-requests, scrapy).
    KnownBadBot,
    /// Suspicious — unusual UA or patterns suggesting automation.
    Suspicious,
    /// Likely a real browser/human.
    LikelyHuman,
}

/// Known good bot User-Agent substrings.
const KNOWN_GOOD_BOTS: &[&str] = &[
    "googlebot",
    "bingbot",
    "yandexbot",
    "duckduckbot",
    "baiduspider",
    "slurp",         // Yahoo
    "facebookexternalhit",
    "twitterbot",
    "linkedinbot",
    "applebot",
];

/// Known bad bot User-Agent substrings.
const KNOWN_BAD_BOTS: &[&str] = &[
    "curl",
    "wget",
    "python-requests",
    "python-urllib",
    "scrapy",
    "httpclient",
    "go-http-client",
    "java/",
    "libwww-perl",
    "mechanize",
    "phantom",
    "headlesschrome",
    "selenium",
];

/// Suspicious indicators in User-Agent strings.
const SUSPICIOUS_PATTERNS: &[&str] = &[
    "bot",
    "crawler",
    "spider",
    "scraper",
    "fetch",
    "scan",
];

/// Classify a User-Agent string against known bot patterns.
///
/// If the UA matches a name in `allowlist`, it is treated as `KnownGoodBot`.
pub fn classify_user_agent(ua: &str, allowlist: &[String]) -> BotPattern {
    if ua.is_empty() {
        return BotPattern::Suspicious;
    }

    let ua_lower = ua.to_lowercase();

    // Check custom allowlist first
    for allowed in allowlist {
        if ua_lower.contains(&allowed.to_lowercase()) {
            return BotPattern::KnownGoodBot;
        }
    }

    // Check known good bots
    for pattern in KNOWN_GOOD_BOTS {
        if ua_lower.contains(pattern) {
            return BotPattern::KnownGoodBot;
        }
    }

    // Check known bad bots
    for pattern in KNOWN_BAD_BOTS {
        if ua_lower.contains(pattern) {
            return BotPattern::KnownBadBot;
        }
    }

    // Check suspicious patterns (but exclude if it looks like a browser)
    let looks_like_browser = ua_lower.contains("mozilla")
        && (ua_lower.contains("chrome")
            || ua_lower.contains("firefox")
            || ua_lower.contains("safari")
            || ua_lower.contains("edge"));

    if !looks_like_browser {
        for pattern in SUSPICIOUS_PATTERNS {
            if ua_lower.contains(pattern) {
                return BotPattern::Suspicious;
            }
        }
    }

    if looks_like_browser {
        return BotPattern::LikelyHuman;
    }

    // Unknown UA but not matching any pattern
    BotPattern::LikelyHuman
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_good_bots() {
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)", &[]),
            BotPattern::KnownGoodBot
        );
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (compatible; Bingbot/2.0; +http://www.bing.com/bingbot.htm)", &[]),
            BotPattern::KnownGoodBot
        );
    }

    #[test]
    fn test_known_bad_bots() {
        assert_eq!(classify_user_agent("curl/7.88.1", &[]), BotPattern::KnownBadBot);
        assert_eq!(classify_user_agent("python-requests/2.31.0", &[]), BotPattern::KnownBadBot);
        assert_eq!(classify_user_agent("Scrapy/2.9.0", &[]), BotPattern::KnownBadBot);
        assert_eq!(classify_user_agent("Wget/1.21", &[]), BotPattern::KnownBadBot);
    }

    #[test]
    fn test_suspicious() {
        assert_eq!(classify_user_agent("", &[]), BotPattern::Suspicious);
        assert_eq!(classify_user_agent("MyCustomBot/1.0", &[]), BotPattern::Suspicious);
    }

    #[test]
    fn test_likely_human() {
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36", &[]),
            BotPattern::LikelyHuman
        );
    }

    #[test]
    fn test_custom_allowlist() {
        assert_eq!(
            classify_user_agent("MyInternalBot/1.0", &["MyInternalBot".to_string()]),
            BotPattern::KnownGoodBot
        );
    }
}
