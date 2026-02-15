use dashmap::DashMap;
use std::time::{Duration, Instant};

/// Internal state for a single sliding window counter entry.
struct SlidingWindowState {
    current_count: u64,
    previous_count: u64,
    window_start: Instant,
    window_secs: u64,
    limit: u64,
}

/// A concurrent sliding window counter rate limiter.
///
/// This algorithm approximates a true sliding window by interpolating between
/// the previous and current fixed windows. It provides smoother rate limiting
/// than a simple fixed-window counter while using very little memory per key.
pub struct SlidingWindowLimiter {
    windows: DashMap<String, SlidingWindowState>,
    window_secs: u64,
    limit: u64,
}

impl SlidingWindowLimiter {
    /// Create a new sliding window limiter.
    ///
    /// * `rps`         - maximum requests allowed per second
    /// * `window_secs` - window duration in seconds
    ///
    /// The effective per-window limit is `rps * window_secs`.
    pub fn new(rps: u64, window_secs: u64) -> Self {
        Self {
            windows: DashMap::new(),
            window_secs,
            limit: rps * window_secs,
        }
    }

    /// Check whether a request identified by `key` is allowed.
    ///
    /// Returns `true` if the request is permitted, or `false` if the caller
    /// has exceeded the rate limit.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_secs);

        let mut entry = self.windows.entry(key.to_string()).or_insert_with(|| {
            SlidingWindowState {
                current_count: 0,
                previous_count: 0,
                window_start: now,
                window_secs: self.window_secs,
                limit: self.limit,
            }
        });

        let state = entry.value_mut();

        // Rotate windows if the current window has elapsed.
        // We loop in case more than one full window has passed since the last
        // request (e.g., the client was idle for a long time).
        while now.duration_since(state.window_start) >= window_duration {
            state.previous_count = state.current_count;
            state.current_count = 0;
            state.window_start += window_duration;
        }

        // If the window_start is somehow in the future after rotation (shouldn't
        // happen, but guard defensively), reset.
        let elapsed_in_window = now
            .duration_since(state.window_start)
            .as_secs_f64();
        let window_secs_f64 = state.window_secs as f64;

        // Fraction of the current window that has elapsed (0.0 .. 1.0).
        let elapsed_fraction = (elapsed_in_window / window_secs_f64).min(1.0);

        // Weighted count: blend previous window's contribution with the current
        // window's count.
        let weighted_count =
            (state.previous_count as f64) * (1.0 - elapsed_fraction) + (state.current_count as f64);

        if weighted_count < state.limit as f64 {
            state.current_count += 1;
            true
        } else {
            false
        }
    }

    /// Remove entries whose window started more than `2 * window_secs` ago.
    ///
    /// This should be called periodically (e.g., every 60 seconds) to prevent
    /// unbounded memory growth from one-off client keys.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let stale_threshold = Duration::from_secs(self.window_secs * 2);

        self.windows.retain(|_key, state| {
            now.duration_since(state.window_start) < stale_threshold
        });

        tracing::debug!(
            remaining = self.windows.len(),
            "sliding window cleanup complete"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn allows_up_to_limit() {
        // 10 rps with a 1-second window => limit of 10 per window.
        let limiter = SlidingWindowLimiter::new(10, 1);
        let key = "test-client";

        for i in 0..10 {
            assert!(limiter.check(key), "request {} should be allowed", i);
        }

        assert!(!limiter.check(key), "should deny beyond limit");
    }

    #[test]
    fn window_rotation_resets_count() {
        // 5 rps, 1-second window => limit of 5.
        let limiter = SlidingWindowLimiter::new(5, 1);
        let key = "rotate-client";

        // Exhaust the limit.
        for _ in 0..5 {
            limiter.check(key);
        }
        assert!(!limiter.check(key));

        // Wait for the window to rotate (plus some margin for the weighted
        // blending of the previous window to decay).
        thread::sleep(Duration::from_millis(1100));

        // After rotation the previous window's count still contributes via
        // weighting, but with elapsed_fraction close to 1.0 the weighted
        // previous contribution is near zero, so new requests should pass.
        assert!(limiter.check(key), "should allow after window rotation");
    }

    #[test]
    fn independent_keys() {
        let limiter = SlidingWindowLimiter::new(2, 1);

        assert!(limiter.check("a"));
        assert!(limiter.check("a"));
        assert!(!limiter.check("a"));

        // Key B is independent.
        assert!(limiter.check("b"));
    }

    #[test]
    fn cleanup_removes_stale_entries() {
        let limiter = SlidingWindowLimiter::new(10, 1);
        limiter.check("keep-alive");
        limiter.check("will-be-stale");

        // Manually age one entry.
        {
            let mut entry = limiter.windows.get_mut("will-be-stale").unwrap();
            entry.window_start = Instant::now() - Duration::from_secs(10);
        }

        limiter.cleanup();

        assert!(limiter.windows.contains_key("keep-alive"));
        assert!(!limiter.windows.contains_key("will-be-stale"));
    }
}
