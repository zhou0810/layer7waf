use sha2::{Digest, Sha256};

/// HTTP fingerprint computed from request headers.
#[derive(Debug, Clone)]
pub struct HttpFingerprint {
    /// SHA-256 hash of ordered lowercase header names.
    pub header_order_hash: String,
    /// Extracted User-Agent family (e.g. "Chrome", "Firefox", "curl").
    pub ua_family: String,
    /// Hash of the Accept header combination.
    pub accept_hash: String,
}

/// Compute an HTTP fingerprint from the given headers and method.
///
/// `headers` is a slice of (name, value) pairs in the order they appeared in the request.
pub fn compute_fingerprint(headers: &[(String, String)], _method: &str) -> HttpFingerprint {
    // Header order hash: SHA-256 of lowercase header names joined by commas
    let header_names: Vec<String> = headers.iter().map(|(k, _)| k.to_lowercase()).collect();
    let header_order_input = header_names.join(",");
    let header_order_hash = sha256_hex(header_order_input.as_bytes());

    // User-Agent family extraction
    let ua = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("user-agent"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    let ua_family = extract_ua_family(ua);

    // Accept header hash
    let accept = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("accept"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    let accept_encoding = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("accept-encoding"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    let accept_language = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("accept-language"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    let accept_input = format!("{}|{}|{}", accept, accept_encoding, accept_language);
    let accept_hash = sha256_hex(accept_input.as_bytes());

    HttpFingerprint {
        header_order_hash,
        ua_family,
        accept_hash,
    }
}

/// Extract a UA family string from a User-Agent header value.
fn extract_ua_family(ua: &str) -> String {
    let ua_lower = ua.to_lowercase();

    if ua_lower.contains("chrome") && !ua_lower.contains("chromium") && !ua_lower.contains("edg") {
        "Chrome".to_string()
    } else if ua_lower.contains("firefox") {
        "Firefox".to_string()
    } else if ua_lower.contains("safari") && !ua_lower.contains("chrome") {
        "Safari".to_string()
    } else if ua_lower.contains("edg") {
        "Edge".to_string()
    } else if ua_lower.contains("curl") {
        "curl".to_string()
    } else if ua_lower.contains("wget") {
        "wget".to_string()
    } else if ua_lower.contains("python-requests") || ua_lower.contains("python-urllib") {
        "python".to_string()
    } else if ua_lower.contains("scrapy") {
        "scrapy".to_string()
    } else if ua_lower.contains("googlebot") {
        "Googlebot".to_string()
    } else if ua_lower.contains("bingbot") {
        "Bingbot".to_string()
    } else if ua_lower.contains("bot") || ua_lower.contains("crawler") || ua_lower.contains("spider") {
        "bot-generic".to_string()
    } else if ua.is_empty() {
        "empty".to_string()
    } else {
        "other".to_string()
    }
}

/// Compute SHA-256 and return as hex string.
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Check whether the request has a standard Accept header (i.e. not missing or unusual).
pub fn has_standard_accept(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("accept") && !v.is_empty() && v != "*/*")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fingerprint_basic() {
        let headers = vec![
            ("Host".into(), "example.com".into()),
            ("User-Agent".into(), "Mozilla/5.0 Chrome/120".into()),
            ("Accept".into(), "text/html".into()),
            ("Accept-Encoding".into(), "gzip, deflate".into()),
            ("Accept-Language".into(), "en-US".into()),
        ];
        let fp = compute_fingerprint(&headers, "GET");
        assert_eq!(fp.ua_family, "Chrome");
        assert!(!fp.header_order_hash.is_empty());
        assert!(!fp.accept_hash.is_empty());
    }

    #[test]
    fn test_ua_family_extraction() {
        assert_eq!(extract_ua_family("curl/7.88.1"), "curl");
        assert_eq!(extract_ua_family("python-requests/2.31.0"), "python");
        assert_eq!(extract_ua_family("Scrapy/2.9.0"), "scrapy");
        assert_eq!(extract_ua_family("Googlebot/2.1"), "Googlebot");
        assert_eq!(extract_ua_family(""), "empty");
        assert_eq!(extract_ua_family("Mozilla/5.0 (compatible; Bingbot/2.0)"), "Bingbot");
    }

    #[test]
    fn test_has_standard_accept() {
        let with_accept = vec![("Accept".into(), "text/html".into())];
        assert!(has_standard_accept(&with_accept));

        let wildcard = vec![("Accept".into(), "*/*".into())];
        assert!(!has_standard_accept(&wildcard));

        let empty: Vec<(String, String)> = vec![];
        assert!(!has_standard_accept(&empty));
    }

    #[test]
    fn test_different_header_orders_produce_different_hashes() {
        let h1 = vec![
            ("Host".into(), "a.com".into()),
            ("Accept".into(), "text/html".into()),
        ];
        let h2 = vec![
            ("Accept".into(), "text/html".into()),
            ("Host".into(), "a.com".into()),
        ];
        let fp1 = compute_fingerprint(&h1, "GET");
        let fp2 = compute_fingerprint(&h2, "GET");
        assert_ne!(fp1.header_order_hash, fp2.header_order_hash);
    }
}
