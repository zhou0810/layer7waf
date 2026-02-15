use layer7waf_rate_limit::RateLimiter;

#[test]
fn test_token_bucket_basic() {
    let limiter = RateLimiter::new_token_bucket(10, 5);

    // Should allow burst of 5
    for _ in 0..5 {
        assert!(limiter.check("test-client"));
    }

    // 6th request should be denied
    assert!(!limiter.check("test-client"));
}

#[test]
fn test_token_bucket_different_keys() {
    let limiter = RateLimiter::new_token_bucket(10, 2);

    assert!(limiter.check("client-a"));
    assert!(limiter.check("client-a"));
    assert!(!limiter.check("client-a"));

    // Different client should have its own bucket
    assert!(limiter.check("client-b"));
    assert!(limiter.check("client-b"));
    assert!(!limiter.check("client-b"));
}

#[test]
fn test_sliding_window_basic() {
    let limiter = RateLimiter::new_sliding_window(5, 1);

    // Should allow 5 requests per 1-second window
    for _ in 0..5 {
        assert!(limiter.check("test-client"));
    }

    // 6th should be denied
    assert!(!limiter.check("test-client"));
}
