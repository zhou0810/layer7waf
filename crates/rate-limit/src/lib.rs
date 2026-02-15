//! Rate limiting for the Layer 7 WAF.
//!
//! This crate provides two interchangeable rate-limiting algorithms behind a
//! unified [`RateLimiter`] facade:
//!
//! - **Token bucket** -- smooth, burst-tolerant limiting well suited for API
//!   gateways. Each key gets a bucket that refills at a steady rate and can
//!   accumulate up to a configurable burst capacity.
//!
//! - **Sliding window counter** -- a lightweight approximation of a true
//!   sliding window that blends the previous and current fixed-window counts.
//!   Good when you want hard per-window caps with minimal memory overhead.
//!
//! Both implementations use [`DashMap`](dashmap::DashMap) for lock-free
//! concurrent access and include periodic cleanup to evict stale entries.

pub mod sliding_window;
pub mod token_bucket;

use std::sync::Arc;

pub use sliding_window::SlidingWindowLimiter;
pub use token_bucket::TokenBucketLimiter;

/// A unified rate limiter that delegates to one of the supported algorithms.
///
/// This is the primary public interface of the crate. Construct it with one of
/// the `new_*` constructors and then call [`check`](RateLimiter::check) on
/// every incoming request.
///
/// The limiter is cheaply cloneable (backed by `Arc`) and safe to share across
/// tasks and threads.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

enum RateLimiterInner {
    TokenBucket(TokenBucketLimiter),
    SlidingWindow(SlidingWindowLimiter),
}

impl RateLimiter {
    /// Create a rate limiter backed by the token bucket algorithm.
    ///
    /// * `rps`   - sustained requests per second (token refill rate)
    /// * `burst` - maximum burst size (bucket capacity)
    pub fn new_token_bucket(rps: u64, burst: u64) -> Self {
        tracing::info!(rps, burst, "creating token bucket rate limiter");
        Self {
            inner: Arc::new(RateLimiterInner::TokenBucket(
                TokenBucketLimiter::new(rps, burst),
            )),
        }
    }

    /// Create a rate limiter backed by the sliding window counter algorithm.
    ///
    /// * `rps`         - maximum requests allowed per second
    /// * `window_secs` - window duration in seconds
    pub fn new_sliding_window(rps: u64, window_secs: u64) -> Self {
        tracing::info!(rps, window_secs, "creating sliding window rate limiter");
        Self {
            inner: Arc::new(RateLimiterInner::SlidingWindow(
                SlidingWindowLimiter::new(rps, window_secs),
            )),
        }
    }

    /// Check whether a request identified by `key` is allowed.
    ///
    /// Returns `true` if the request is permitted, `false` if the caller has
    /// exceeded the rate limit and should receive a 429 response.
    pub fn check(&self, key: &str) -> bool {
        match self.inner.as_ref() {
            RateLimiterInner::TokenBucket(limiter) => limiter.check(key),
            RateLimiterInner::SlidingWindow(limiter) => limiter.check(key),
        }
    }

    /// Spawn a background Tokio task that periodically evicts stale entries.
    ///
    /// The cleanup task runs every 60 seconds and will continue until the
    /// runtime shuts down. It holds an `Arc` reference to the inner limiter,
    /// so the limiter will stay alive as long as the task is running.
    pub fn start_cleanup_task(&self) {
        let inner = Arc::clone(&self.inner);

        std::thread::Builder::new()
            .name("rate-limit-cleanup".into())
            .spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(60));

                match inner.as_ref() {
                    RateLimiterInner::TokenBucket(limiter) => limiter.cleanup(),
                    RateLimiterInner::SlidingWindow(limiter) => limiter.cleanup(),
                }

                tracing::trace!("rate limiter cleanup tick completed");
            })
            .expect("failed to spawn rate-limit cleanup thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_through_facade() {
        let limiter = RateLimiter::new_token_bucket(5, 3);

        // Should allow burst of 3.
        assert!(limiter.check("client-a"));
        assert!(limiter.check("client-a"));
        assert!(limiter.check("client-a"));

        // 4th request exceeds burst.
        assert!(!limiter.check("client-a"));

        // Different key is independent.
        assert!(limiter.check("client-b"));
    }

    #[test]
    fn sliding_window_through_facade() {
        let limiter = RateLimiter::new_sliding_window(5, 1);

        // Limit = 5 * 1 = 5 per window.
        for i in 0..5 {
            assert!(limiter.check("client-x"), "request {} should pass", i);
        }

        assert!(!limiter.check("client-x"), "should deny beyond window limit");
    }

    #[test]
    fn clone_shares_state() {
        let limiter = RateLimiter::new_token_bucket(10, 2);
        let limiter2 = limiter.clone();

        assert!(limiter.check("shared"));
        assert!(limiter2.check("shared"));

        // Both clones consumed from the same bucket -- should now be empty.
        assert!(!limiter.check("shared"));
        assert!(!limiter2.check("shared"));
    }
}
