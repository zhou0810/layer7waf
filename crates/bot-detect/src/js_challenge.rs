use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Generate a self-contained HTML page with an embedded JS proof-of-work challenge.
///
/// The page computes SHA-256 hashes until it finds one with the required number of
/// leading zero bits, then sets a cookie and redirects to the original URL.
pub fn generate_challenge(client_ip: &str, difficulty: u32, secret: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // The challenge string the client must find a nonce for
    let challenge_data = format!("{}:{}", client_ip, timestamp);

    // Pre-compute HMAC of the challenge data for server-side verification
    let hmac_value = compute_hmac(secret, &format!("{}:verified", challenge_data));

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<title>Checking your browser...</title>
<style>
body {{ font-family: -apple-system, sans-serif; display: flex; justify-content: center;
  align-items: center; min-height: 100vh; margin: 0; background: #0a0a0a; color: #e0e0e0; }}
.container {{ text-align: center; max-width: 400px; }}
.spinner {{ width: 40px; height: 40px; border: 3px solid #333; border-top: 3px solid #3b82f6;
  border-radius: 50%; animation: spin 1s linear infinite; margin: 20px auto; }}
@keyframes spin {{ to {{ transform: rotate(360deg); }} }}
p {{ color: #888; font-size: 14px; }}
</style>
</head>
<body>
<div class="container">
  <h2>Verifying you are human</h2>
  <div class="spinner"></div>
  <p id="status">Running browser check...</p>
</div>
<script>
(async function() {{
  const challenge = "{challenge_data}";
  const difficulty = {difficulty};
  const hmac = "{hmac_value}";
  const ip = "{client_ip}";
  const ts = "{timestamp}";

  // SHA-256 helper using Web Crypto API
  async function sha256(msg) {{
    const data = new TextEncoder().encode(msg);
    const buf = await crypto.subtle.digest('SHA-256', data);
    return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('');
  }}

  // Check if hash has required leading zero bits
  function hasLeadingZeros(hash, bits) {{
    const fullBytes = Math.floor(bits / 4);
    const prefix = hash.substring(0, fullBytes);
    for (let i = 0; i < prefix.length; i++) {{
      if (prefix[i] !== '0') return false;
    }}
    if (bits % 4 !== 0) {{
      const nextChar = parseInt(hash[fullBytes], 16);
      const remaining = bits % 4;
      if (nextChar >= (1 << (4 - remaining))) return false;
    }}
    return true;
  }}

  // Proof-of-work: find nonce where SHA-256(challenge + ":" + nonce) has leading zeros
  let nonce = 0;
  let hash = '';
  const statusEl = document.getElementById('status');
  const startTime = Date.now();

  while (true) {{
    hash = await sha256(challenge + ':' + nonce);
    if (hasLeadingZeros(hash, difficulty)) break;
    nonce++;
    if (nonce % 1000 === 0) {{
      statusEl.textContent = 'Computing... (' + nonce + ' hashes)';
      await new Promise(r => setTimeout(r, 0)); // yield to UI
    }}
  }}

  const elapsed = Date.now() - startTime;
  statusEl.textContent = 'Verified in ' + elapsed + 'ms. Redirecting...';

  // Set verification cookie: ip:timestamp:hash:hmac
  const cookieValue = ip + ':' + ts + ':' + hash + ':' + hmac;
  document.cookie = '__l7w_bc=' + encodeURIComponent(cookieValue) + ';path=/;max-age=3600;SameSite=Lax';

  // Redirect to the same page
  setTimeout(function() {{ window.location.reload(); }}, 500);
}})();
</script>
</body>
</html>"#,
        challenge_data = challenge_data,
        difficulty = difficulty,
        hmac_value = hmac_value,
        client_ip = client_ip,
        timestamp = timestamp,
    )
}

/// Verify a challenge cookie value.
///
/// Cookie format: `ip:timestamp:hash:hmac`
///
/// Returns `true` if the cookie is valid (correct HMAC, within TTL, matching IP).
pub fn verify_challenge_cookie(
    cookie_value: &str,
    client_ip: &str,
    secret: &str,
    ttl_secs: u64,
) -> bool {
    let parts: Vec<&str> = cookie_value.splitn(4, ':').collect();
    if parts.len() != 4 {
        return false;
    }

    let cookie_ip = parts[0];
    let cookie_ts = parts[1];
    let _cookie_hash = parts[2];
    let cookie_hmac = parts[3];

    // Verify IP matches
    if cookie_ip != client_ip {
        return false;
    }

    // Verify timestamp is within TTL
    let ts: u64 = match cookie_ts.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now.saturating_sub(ts) > ttl_secs {
        return false;
    }

    // Verify HMAC
    let challenge_data = format!("{}:{}:verified", cookie_ip, cookie_ts);
    let expected_hmac = compute_hmac(secret, &challenge_data);

    cookie_hmac == expected_hmac
}

/// Compute HMAC-SHA256 and return as hex string.
fn compute_hmac(secret: &str, data: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Extract the `__l7w_bc` cookie value from a Cookie header string.
pub fn extract_challenge_cookie(cookie_header: &str) -> Option<String> {
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("__l7w_bc=") {
            // URL-decode the value
            let decoded = urldecode(value);
            return Some(decoded);
        }
    }
    None
}

/// Simple URL decode (handles %XX encoding).
fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push('%');
                result.push_str(&hex);
            }
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
    fn test_generate_challenge_contains_html() {
        let html = generate_challenge("192.168.1.1", 16, "test-secret");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("__l7w_bc"));
        assert!(html.contains("crypto.subtle.digest"));
    }

    #[test]
    fn test_verify_challenge_cookie_valid() {
        let secret = "test-secret-key";
        let ip = "10.0.0.1";
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Build a valid cookie
        let challenge_data = format!("{}:{}:verified", ip, now);
        let hmac = compute_hmac(secret, &challenge_data);
        let cookie = format!("{}:{}:somehash:{}", ip, now, hmac);

        assert!(verify_challenge_cookie(&cookie, ip, secret, 3600));
    }

    #[test]
    fn test_verify_challenge_cookie_wrong_ip() {
        let secret = "test-secret-key";
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let challenge_data = format!("10.0.0.1:{}:verified", now);
        let hmac = compute_hmac(secret, &challenge_data);
        let cookie = format!("10.0.0.1:{}:somehash:{}", now, hmac);

        // Different IP should fail
        assert!(!verify_challenge_cookie(&cookie, "10.0.0.2", secret, 3600));
    }

    #[test]
    fn test_verify_challenge_cookie_expired() {
        let secret = "test-secret-key";
        let ip = "10.0.0.1";
        // Timestamp from 2 hours ago
        let old_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 7200;

        let challenge_data = format!("{}:{}:verified", ip, old_ts);
        let hmac = compute_hmac(secret, &challenge_data);
        let cookie = format!("{}:{}:somehash:{}", ip, old_ts, hmac);

        // TTL of 3600 should reject a 7200-second-old cookie
        assert!(!verify_challenge_cookie(&cookie, ip, secret, 3600));
    }

    #[test]
    fn test_extract_challenge_cookie() {
        assert_eq!(
            extract_challenge_cookie("session=abc; __l7w_bc=10.0.0.1%3A123%3Ahash%3Ahmac; other=x"),
            Some("10.0.0.1:123:hash:hmac".to_string())
        );
        assert_eq!(
            extract_challenge_cookie("session=abc"),
            None
        );
    }
}
