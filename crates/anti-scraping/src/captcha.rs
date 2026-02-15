use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Generate a self-hosted math CAPTCHA HTML page.
///
/// Renders an SVG with a randomized arithmetic problem and an answer form.
/// On correct submission, sets an HMAC-signed cookie.
pub fn generate_captcha_page(client_ip: &str, secret: &str, original_path: &str) -> String {
    let mut rng = rand::thread_rng();
    let a: u32 = rng.gen_range(2..50);
    let b: u32 = rng.gen_range(2..50);
    let answer = a + b;

    // Create noise elements for the SVG
    let mut noise_lines = String::new();
    for _ in 0..5 {
        let x1: u32 = rng.gen_range(0..200);
        let y1: u32 = rng.gen_range(0..60);
        let x2: u32 = rng.gen_range(0..200);
        let y2: u32 = rng.gen_range(0..60);
        let r: u8 = rng.gen_range(100..200);
        let g: u8 = rng.gen_range(100..200);
        let b_color: u8 = rng.gen_range(100..200);
        noise_lines.push_str(&format!(
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="rgb({r},{g},{b_color})" stroke-width="1"/>"#
        ));
    }

    // Randomized digit positions and rotations
    let a_x: u32 = rng.gen_range(15..35);
    let a_rot: i32 = rng.gen_range(-15..15);
    let plus_x: u32 = rng.gen_range(70..90);
    let b_x: u32 = rng.gen_range(115..140);
    let b_rot: i32 = rng.gen_range(-15..15);
    let eq_x: u32 = rng.gen_range(160..180);

    // Sign the answer for verification
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let answer_hash = sha256_hex(format!("{answer}").as_bytes());
    let mac_input = format!("{client_ip}:{timestamp}:{answer_hash}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key");
    mac.update(mac_input.as_bytes());
    let hmac_hex = hex::encode(mac.finalize().into_bytes());

    // Hidden fields encode the challenge
    let challenge_token = format!("{client_ip}:{timestamp}:{answer_hash}:{hmac_hex}");

    // Build SVG text elements
    let fill_color = "#333";
    let svg_texts = format!(
        concat!(
            r#"<text x="{}" y="40" font-size="28" font-family="monospace" fill="{}" transform="rotate({},{},40)">{}</text>"#,
            r#"<text x="{}" y="40" font-size="28" font-family="monospace" fill="{}">+</text>"#,
            r#"<text x="{}" y="40" font-size="28" font-family="monospace" fill="{}" transform="rotate({},{},40)">{}</text>"#,
            r#"<text x="{}" y="40" font-size="28" font-family="monospace" fill="{}">= ?</text>"#,
        ),
        a_x, fill_color, a_rot, a_x, a,
        plus_x, fill_color,
        b_x, fill_color, b_rot, b_x, b,
        eq_x, fill_color,
    );

    // Build HTML using concat of string pieces to avoid format! issues with CSS/JS
    let mut html = String::with_capacity(4096);
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<title>Verification Required</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; display: flex; justify-content: center; align-items: center; min-height: 100vh; margin: 0; background: #0a0a0a; color: #e5e5e5; }\n");
    html.push_str(".container { text-align: center; padding: 2rem; max-width: 400px; background: #1a1a1a; border-radius: 12px; border: 1px solid #333; }\n");
    html.push_str("h1 { font-size: 1.5rem; margin-bottom: 0.5rem; }\n");
    html.push_str("p { color: #999; font-size: 0.875rem; margin-bottom: 1.5rem; }\n");
    html.push_str("svg { display: block; margin: 0 auto 1rem; background: #f5f5f5; border-radius: 8px; }\n");
    html.push_str("input[type=\"text\"] { padding: 0.5rem 1rem; font-size: 1.25rem; width: 120px; text-align: center; border: 1px solid #555; border-radius: 6px; background: #222; color: #fff; }\n");
    html.push_str("button { margin-top: 1rem; padding: 0.5rem 2rem; font-size: 1rem; background: #3b82f6; color: #fff; border: none; border-radius: 6px; cursor: pointer; }\n");
    html.push_str("button:hover { background: #2563eb; }\n");
    html.push_str(".error { color: #ef4444; font-size: 0.875rem; margin-top: 0.5rem; display: none; }\n");
    html.push_str("</style>\n</head>\n<body>\n");
    html.push_str("<div class=\"container\">\n");
    html.push_str("<h1>Verification Required</h1>\n");
    html.push_str("<p>Please solve the math problem below to continue.</p>\n");
    html.push_str("<svg width=\"200\" height=\"60\" viewBox=\"0 0 200 60\" xmlns=\"http://www.w3.org/2000/svg\">\n");
    html.push_str(&noise_lines);
    html.push('\n');
    html.push_str(&svg_texts);
    html.push_str("\n</svg>\n");
    html.push_str(&format!(
        "<form method=\"POST\" action=\"{}\" id=\"captcha-form\">\n",
        original_path
    ));
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"__l7w_captcha_token\" value=\"{}\">\n",
        challenge_token
    ));
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"__l7w_captcha_path\" value=\"{}\">\n",
        original_path
    ));
    html.push_str("<input type=\"text\" name=\"__l7w_captcha_answer\" id=\"answer\" placeholder=\"Answer\" autocomplete=\"off\" autofocus>\n");
    html.push_str("<div class=\"error\" id=\"error-msg\">Incorrect answer. Please try again.</div>\n");
    html.push_str("<br>\n<button type=\"submit\">Verify</button>\n");
    html.push_str("</form>\n");
    html.push_str("<script>\n");
    html.push_str("document.getElementById('captcha-form').addEventListener('submit', function(e) {\n");
    html.push_str("  e.preventDefault();\n");
    html.push_str("  var answer = document.getElementById('answer').value.trim();\n");
    html.push_str("  if (!answer) return;\n");
    html.push_str("  var token = document.querySelector('[name=__l7w_captcha_token]').value;\n");
    html.push_str("  var path = document.querySelector('[name=__l7w_captcha_path]').value;\n");
    html.push_str("  document.cookie = '__l7w_captcha=' + encodeURIComponent(token + ':' + answer) + '; path=/; max-age=1800; SameSite=Strict';\n");
    html.push_str("  window.location.href = path;\n");
    html.push_str("});\n");
    html.push_str("</script>\n");
    html.push_str("</div>\n</body>\n</html>");

    html
}

/// Verify a CAPTCHA cookie value.
///
/// Cookie format: `ip:timestamp:answer_hash:hmac:user_answer`
pub fn verify_captcha_cookie(cookie_value: &str, client_ip: &str, secret: &str, ttl_secs: u64) -> bool {
    let parts: Vec<&str> = cookie_value.split(':').collect();
    if parts.len() != 5 {
        return false;
    }

    let (ip, ts_str, answer_hash, hmac_hex, user_answer) =
        (parts[0], parts[1], parts[2], parts[3], parts[4]);

    // Verify IP matches
    if ip != client_ip {
        return false;
    }

    // Verify timestamp not expired
    let ts: u64 = match ts_str.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now.saturating_sub(ts) > ttl_secs {
        return false;
    }

    // Verify HMAC
    let mac_input = format!("{ip}:{ts_str}:{answer_hash}");
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(mac_input.as_bytes());
    let expected_hmac = hex::encode(mac.finalize().into_bytes());
    if hmac_hex != expected_hmac {
        return false;
    }

    // Verify user answer matches the hash
    let user_answer_hash = sha256_hex(user_answer.as_bytes());
    answer_hash == user_answer_hash
}

/// Extract the `__l7w_captcha` cookie from a Cookie header value.
pub fn extract_captcha_cookie(cookie_header: &str) -> Option<String> {
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix("__l7w_captcha=") {
            let decoded = urldecode(value);
            return Some(decoded);
        }
    }
    None
}

fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex_str: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex_str);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_captcha_page_contains_svg() {
        let html = generate_captcha_page("1.2.3.4", "test-secret", "/test");
        assert!(html.contains("<svg"));
        assert!(html.contains("__l7w_captcha_token"));
        assert!(html.contains("Verification Required"));
    }

    #[test]
    fn test_extract_captcha_cookie() {
        let cookie = "session=abc; __l7w_captcha=some%3Avalue; other=123";
        let result = extract_captcha_cookie(cookie);
        assert_eq!(result, Some("some:value".to_string()));
    }

    #[test]
    fn test_extract_captcha_cookie_missing() {
        let cookie = "session=abc; other=123";
        assert!(extract_captcha_cookie(cookie).is_none());
    }

    #[test]
    fn test_verify_captcha_invalid_parts() {
        assert!(!verify_captcha_cookie("a:b:c", "1.2.3.4", "secret", 3600));
    }

    #[test]
    fn test_verify_captcha_wrong_ip() {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let answer_hash = sha256_hex(b"42");
        let mac_input = format!("1.2.3.4:{ts}:{answer_hash}");
        let mut mac = HmacSha256::new_from_slice(b"secret").unwrap();
        mac.update(mac_input.as_bytes());
        let hmac_hex = hex::encode(mac.finalize().into_bytes());
        let cookie = format!("1.2.3.4:{ts}:{answer_hash}:{hmac_hex}:42");
        assert!(!verify_captcha_cookie(&cookie, "5.6.7.8", "secret", 3600));
    }

    #[test]
    fn test_verify_captcha_valid() {
        let ip = "10.0.0.1";
        let secret = "test-secret";
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let answer = "42";
        let answer_hash = sha256_hex(answer.as_bytes());
        let mac_input = format!("{ip}:{ts}:{answer_hash}");
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(mac_input.as_bytes());
        let hmac_hex = hex::encode(mac.finalize().into_bytes());
        let cookie = format!("{ip}:{ts}:{answer_hash}:{hmac_hex}:{answer}");
        assert!(verify_captcha_cookie(&cookie, ip, secret, 3600));
    }
}
