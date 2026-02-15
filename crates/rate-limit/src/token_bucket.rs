use dashmap::DashMap;
use std::time::Instant;

/// Internal state for a single token bucket entry.
struct TokenBucketState {
    tokens: f64,
    last_refill: Instant,
    rate: f64,
    burst: f64,
}

/// A concurrent token bucket rate limiter.
///
/// Each key (e.g., client IP) gets its own independent bucket that refills at
/// `rate` tokens per second up to a maximum of `burst` tokens. Every allowed
/// request consumes exactly one token.
pub struct TokenBucketLimiter {
    buckets: DashMap<String, TokenBucketState>,
    rate: f64,
    burst: f64,
}

impl TokenBucketLimiter {
    /// Create a new token bucket limiter.
    ///
    /// * `rps`   - sustained requests per second (refill rate)
    /// * `burst` - maximum burst size (bucket capacity)
    pub fn new(rps: u64, burst: u64) -> Self {
        Self {
            buckets: DashMap::new(),
            rate: rps as f64,
            burst: burst as f64,
        }
    }

    /// Check whether a request identified by `key` is allowed.
    ///
    /// Returns `true` if the request is permitted (a token was available and
    /// consumed), or `false` if the caller should be rate-limited.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();

        let mut entry = self.buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucketState {
                tokens: self.burst,
                last_refill: now,
                rate: self.rate,
                burst: self.burst,
            }
        });

        let state = entry.value_mut();

        // Refill tokens based on elapsed time.
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        state.tokens = (state.tokens + elapsed * state.rate).min(state.burst);
        state.last_refill = now;

        // Try to consume one token.
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Remove entries that have not been accessed in more than 5 minutes.
    ///
    /// This should be called periodically (e.g., every 60 seconds) to prevent
    /// unbounded memory growth from one-off client keys.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let stale_threshold = std::time::Duration::from_secs(5 * 60);

        self.buckets.retain(|_key, state| {
            now.duration_since(state.last_refill) < stale_threshold
        });

        tracing::debug!(
            remaining = self.buckets.len(),
            "token bucket cleanup complete"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn allows_up_to_burst() {
        let limiter = TokenBucketLimiter::new(10, 5);
        let key = "test-client";

        // Should allow up to burst (5) requests immediately.
        for _ in 0..5 {
            assert!(limiter.check(key), "should allow within burst");
        }

        // The 6th should be denied.
        assert!(!limiter.check(key), "should deny beyond burst");
    }

    #[test]
    fn refills_over_time() {
        let limiter = TokenBucketLimiter::new(10, 5);
        let key = "refill-client";

        // Drain all tokens.
        for _ in 0..5 {
            limiter.check(key);
        }
        assert!(!limiter.check(key));

        // Wait enough time for at least 1 token to refill (100ms at 10 rps = 1 token).
        thread::sleep(Duration::from_millis(150));

        assert!(limiter.check(key), "should allow after refill");
    }

    #[test]
    fn independent_keys() {
        let limiter = TokenBucketLimiter::new(10, 2);

        // Drain key A.
        assert!(limiter.check("a"));
        assert!(limiter.check("a"));
        assert!(!limiter.check("a"));

        // Key B should be unaffected.
        assert!(limiter.check("b"));
    }

    #[test]
    fn cleanup_removes_stale_entries() {
        let limiter = TokenBucketLimiter::new(10, 10);
        limiter.check("keep-alive");
        limiter.check("will-be-stale");

        // Manually age one entry by replacing its last_refill.
        {
            let mut entry = limiter.buckets.get_mut("will-be-stale").unwrap();
            entry.last_refill = Instant::now() - Duration::from_secs(6 * 60);
        }

        limiter.cleanup();

        assert!(limiter.buckets.contains_key("keep-alive"));
        assert!(!limiter.buckets.contains_key("will-be-stale"));
    }
}
