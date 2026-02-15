/// Generate a hidden trap link HTML snippet.
///
/// The link is invisible to regular users (off-screen, aria-hidden, no tab focus)
/// but scrapers following all links will hit the trap path.
pub fn generate_trap_html(trap_path_prefix: &str, client_ip: &str, secret: &str) -> String {
    // Create a unique trap path per IP using HMAC
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC key");
    mac.update(client_ip.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());
    let short_hash = &hash[..12];

    format!(
        r#"<a href="{trap_path_prefix}/{short_hash}" style="position:absolute;left:-10000px;top:-10000px;width:1px;height:1px;overflow:hidden" aria-hidden="true" tabindex="-1"></a>"#
    )
}

/// Check if a request path matches the trap path prefix.
pub fn is_trap_request(path: &str, trap_path_prefix: &str) -> bool {
    path.starts_with(trap_path_prefix)
}

/// Inject trap HTML before the closing `</body>` tag.
///
/// Returns `None` if the body doesn't contain `</body>`.
pub fn inject_trap(body: &[u8], trap_html: &str) -> Option<Vec<u8>> {
    // Search for </body> case-insensitively
    let body_str = std::str::from_utf8(body).ok()?;
    let lower = body_str.to_lowercase();
    let pos = lower.find("</body>")?;

    let mut result = Vec::with_capacity(body.len() + trap_html.len());
    result.extend_from_slice(&body[..pos]);
    result.extend_from_slice(trap_html.as_bytes());
    result.extend_from_slice(&body[pos..]);
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_trap_html() {
        let html = generate_trap_html("/.well-known/l7w-trap", "1.2.3.4", "secret");
        assert!(html.contains("/.well-known/l7w-trap/"));
        assert!(html.contains("aria-hidden=\"true\""));
        assert!(html.contains("tabindex=\"-1\""));
        assert!(html.contains("position:absolute"));
    }

    #[test]
    fn test_is_trap_request_matches() {
        assert!(is_trap_request(
            "/.well-known/l7w-trap/abc123",
            "/.well-known/l7w-trap"
        ));
    }

    #[test]
    fn test_is_trap_request_no_match() {
        assert!(!is_trap_request("/api/users", "/.well-known/l7w-trap"));
    }

    #[test]
    fn test_inject_trap_before_body() {
        let body = b"<html><body><p>Hello</p></body></html>";
        let trap = r#"<a href="/trap" style="display:none"></a>"#;
        let result = inject_trap(body, trap).unwrap();
        let result_str = std::str::from_utf8(&result).unwrap();
        assert!(result_str.contains(r#"<a href="/trap" style="display:none"></a></body>"#));
    }

    #[test]
    fn test_inject_trap_no_body_tag() {
        let body = b"<html><p>No body tag</p></html>";
        let result = inject_trap(body, "<trap>");
        assert!(result.is_none());
    }

    #[test]
    fn test_inject_trap_case_insensitive() {
        let body = b"<html><body><p>Hello</p></BODY></html>";
        let trap = "<trap>";
        let result = inject_trap(body, trap).unwrap();
        let result_str = std::str::from_utf8(&result).unwrap();
        assert!(result_str.contains("<trap></BODY>"));
    }
}
