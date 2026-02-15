use sha2::Digest;

// Zero-width characters used for watermarking
const ZWC_ZERO: char = '\u{200B}'; // ZERO WIDTH SPACE  → bit 0
const ZWC_ONE: char = '\u{200C}';  // ZERO WIDTH NON-JOINER → bit 1

/// Inject zero-width character watermarks into HTML text content.
///
/// Inserts invisible Unicode characters between `>` and `<` text nodes,
/// seeded by client IP for forensic identification of scraping source.
///
/// Returns `None` if the body is not valid UTF-8 or has no suitable text nodes.
pub fn inject_zero_width_chars(body: &[u8], client_ip: &str) -> Option<Vec<u8>> {
    let body_str = std::str::from_utf8(body).ok()?;

    // Generate watermark bits from IP hash
    let watermark = generate_watermark(client_ip);

    let mut result = String::with_capacity(body_str.len() + watermark.len() * 10);
    let mut injected = false;
    let mut injection_count = 0;
    let max_injections = 5;

    let mut chars = body_str.char_indices().peekable();
    let mut last_idx = 0;

    while let Some((idx, ch)) = chars.next() {
        if ch == '>' && injection_count < max_injections {
            // Check if there's text content after this '>' (not another '<')
            if let Some(&(next_idx, next_ch)) = chars.peek() {
                if next_ch != '<' && next_ch != '\n' && !next_ch.is_whitespace() {
                    // Found a text node, inject watermark after '>'
                    result.push_str(&body_str[last_idx..=idx]);
                    result.push_str(&watermark);
                    last_idx = next_idx;
                    injected = true;
                    injection_count += 1;
                    continue;
                }
            }
        }
        let _ = idx; // used via last_idx tracking
    }

    if !injected {
        return None;
    }

    result.push_str(&body_str[last_idx..]);
    Some(result.into_bytes())
}

/// Generate a watermark string from a client IP.
///
/// The watermark encodes a hash of the IP as a sequence of zero-width characters.
fn generate_watermark(client_ip: &str) -> String {
    let hash = sha2::Sha256::digest(client_ip.as_bytes());
    // Use first 4 bytes (32 bits) for the watermark
    let mut watermark = String::new();
    for &byte in &hash[..4] {
        for bit in (0..8).rev() {
            if (byte >> bit) & 1 == 1 {
                watermark.push(ZWC_ONE);
            } else {
                watermark.push(ZWC_ZERO);
            }
        }
    }
    watermark
}

/// Extract a watermark from text content.
///
/// Reads sequences of zero-width characters and returns the hex-encoded hash prefix.
pub fn extract_watermark(text: &str) -> Option<String> {
    let mut bits = Vec::new();

    for ch in text.chars() {
        match ch {
            c if c == ZWC_ZERO => bits.push(false),
            c if c == ZWC_ONE => bits.push(true),
            _ => {
                if bits.len() >= 32 {
                    break;
                }
            }
        }
    }

    if bits.len() < 32 {
        return None;
    }

    // Convert bits to bytes
    let mut bytes = Vec::new();
    for chunk in bits.chunks(8) {
        if chunk.len() == 8 {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }
    }

    Some(hex::encode(&bytes[..4.min(bytes.len())]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_watermark_consistent() {
        let wm1 = generate_watermark("1.2.3.4");
        let wm2 = generate_watermark("1.2.3.4");
        assert_eq!(wm1, wm2);
        assert_eq!(wm1.chars().count(), 32); // 32 zero-width chars
    }

    #[test]
    fn test_generate_watermark_different_ips() {
        let wm1 = generate_watermark("1.2.3.4");
        let wm2 = generate_watermark("5.6.7.8");
        assert_ne!(wm1, wm2);
    }

    #[test]
    fn test_extract_watermark_roundtrip() {
        let wm = generate_watermark("10.0.0.1");
        let extracted = extract_watermark(&wm).unwrap();
        // Verify it matches the first 4 bytes of the SHA256 hash
        let hash = sha2::Sha256::digest(b"10.0.0.1");
        let expected = hex::encode(&hash[..4]);
        assert_eq!(extracted, expected);
    }

    #[test]
    fn test_inject_zero_width_chars() {
        let body = b"<html><body><p>Hello world</p></body></html>";
        let result = inject_zero_width_chars(body, "1.2.3.4");
        assert!(result.is_some());
        let result_bytes = result.unwrap();
        let result_str = std::str::from_utf8(&result_bytes).unwrap();
        // The visible text should still be the same when zero-width chars are stripped
        let visible: String = result_str
            .chars()
            .filter(|&c| c != ZWC_ZERO && c != ZWC_ONE)
            .collect();
        assert_eq!(visible, "<html><body><p>Hello world</p></body></html>");
    }

    #[test]
    fn test_inject_no_text_nodes() {
        let body = b"<html><body><br><br></body></html>";
        let result = inject_zero_width_chars(body, "1.2.3.4");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_watermark_too_short() {
        let text = "\u{200B}\u{200C}";
        assert!(extract_watermark(text).is_none());
    }
}
