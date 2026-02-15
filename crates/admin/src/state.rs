use std::sync::{Arc, RwLock};

use layer7waf_common::AppConfig;
use prometheus::{HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry};
use serde::{Deserialize, Serialize};

/// Shared state type alias used across all route handlers.
pub type SharedState = Arc<AppState>;

/// Central application state holding configuration, metrics, and audit logs.
pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub metrics: WafMetrics,
    pub audit_log: RwLock<Vec<AuditLogEntry>>,
    pub custom_rules: RwLock<Vec<String>>,
    pub start_time: std::time::Instant,
}

/// Prometheus metrics collected by the WAF.
pub struct WafMetrics {
    pub registry: Registry,
    pub requests_total: IntCounter,
    pub requests_blocked: IntCounter,
    pub request_duration: HistogramVec,
    pub rule_hits: IntCounterVec,
    pub rate_limited_total: IntCounter,
    pub bots_detected: IntCounter,
    pub challenges_issued: IntCounter,
    pub challenges_solved: IntCounter,
    pub scrapers_blocked: IntCounter,
    pub traps_triggered: IntCounter,
    pub captchas_issued: IntCounter,
    pub captchas_solved: IntCounter,
    pub responses_obfuscated: IntCounter,
    pub geoip_blocked: IntCounter,
    pub geoip_lookups: IntCounter,
}

/// A single audit log entry representing a processed request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: String,
    pub client_ip: String,
    pub method: String,
    pub uri: String,
    pub rule_id: Option<String>,
    pub action: String,
    pub status: u16,
}

impl WafMetrics {
    /// Create a new WafMetrics instance with all counters and histograms
    /// registered against a fresh Prometheus registry.
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = IntCounter::with_opts(
            Opts::new("waf_requests_total", "Total number of requests processed"),
        )
        .expect("failed to create requests_total counter");

        let requests_blocked = IntCounter::with_opts(
            Opts::new("waf_requests_blocked", "Total number of requests blocked by WAF rules"),
        )
        .expect("failed to create requests_blocked counter");

        let request_duration = HistogramVec::new(
            HistogramOpts::new("waf_request_duration_seconds", "Request processing duration in seconds")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0]),
            &["method", "status"],
        )
        .expect("failed to create request_duration histogram");

        let rule_hits = IntCounterVec::new(
            Opts::new("waf_rule_hits_total", "Number of times each WAF rule was triggered"),
            &["rule_id"],
        )
        .expect("failed to create rule_hits counter");

        let rate_limited_total = IntCounter::with_opts(
            Opts::new("waf_rate_limited_total", "Total number of requests rate-limited"),
        )
        .expect("failed to create rate_limited_total counter");

        let bots_detected = IntCounter::with_opts(
            Opts::new("waf_bots_detected", "Total number of bots detected"),
        )
        .expect("failed to create bots_detected counter");

        let challenges_issued = IntCounter::with_opts(
            Opts::new("waf_challenges_issued", "Total number of JS challenges issued"),
        )
        .expect("failed to create challenges_issued counter");

        let challenges_solved = IntCounter::with_opts(
            Opts::new("waf_challenges_solved", "Total number of JS challenges solved"),
        )
        .expect("failed to create challenges_solved counter");

        let scrapers_blocked = IntCounter::with_opts(
            Opts::new("waf_scrapers_blocked", "Total number of scrapers blocked"),
        )
        .expect("failed to create scrapers_blocked counter");

        let traps_triggered = IntCounter::with_opts(
            Opts::new("waf_traps_triggered", "Total number of honeypot traps triggered"),
        )
        .expect("failed to create traps_triggered counter");

        let captchas_issued = IntCounter::with_opts(
            Opts::new("waf_captchas_issued", "Total number of CAPTCHAs issued"),
        )
        .expect("failed to create captchas_issued counter");

        let captchas_solved = IntCounter::with_opts(
            Opts::new("waf_captchas_solved", "Total number of CAPTCHAs solved"),
        )
        .expect("failed to create captchas_solved counter");

        let responses_obfuscated = IntCounter::with_opts(
            Opts::new("waf_responses_obfuscated", "Total number of responses obfuscated"),
        )
        .expect("failed to create responses_obfuscated counter");

        let geoip_blocked = IntCounter::with_opts(
            Opts::new("waf_geoip_blocked", "Total number of requests blocked by GeoIP"),
        )
        .expect("failed to create geoip_blocked counter");

        let geoip_lookups = IntCounter::with_opts(
            Opts::new("waf_geoip_lookups", "Total number of GeoIP lookups performed"),
        )
        .expect("failed to create geoip_lookups counter");

        registry.register(Box::new(requests_total.clone())).expect("failed to register requests_total");
        registry.register(Box::new(requests_blocked.clone())).expect("failed to register requests_blocked");
        registry.register(Box::new(request_duration.clone())).expect("failed to register request_duration");
        registry.register(Box::new(rule_hits.clone())).expect("failed to register rule_hits");
        registry.register(Box::new(rate_limited_total.clone())).expect("failed to register rate_limited_total");
        registry.register(Box::new(bots_detected.clone())).expect("failed to register bots_detected");
        registry.register(Box::new(challenges_issued.clone())).expect("failed to register challenges_issued");
        registry.register(Box::new(challenges_solved.clone())).expect("failed to register challenges_solved");
        registry.register(Box::new(scrapers_blocked.clone())).expect("failed to register scrapers_blocked");
        registry.register(Box::new(traps_triggered.clone())).expect("failed to register traps_triggered");
        registry.register(Box::new(captchas_issued.clone())).expect("failed to register captchas_issued");
        registry.register(Box::new(captchas_solved.clone())).expect("failed to register captchas_solved");
        registry.register(Box::new(responses_obfuscated.clone())).expect("failed to register responses_obfuscated");
        registry.register(Box::new(geoip_blocked.clone())).expect("failed to register geoip_blocked");
        registry.register(Box::new(geoip_lookups.clone())).expect("failed to register geoip_lookups");

        Self {
            registry,
            requests_total,
            requests_blocked,
            request_duration,
            rule_hits,
            rate_limited_total,
            bots_detected,
            challenges_issued,
            challenges_solved,
            scrapers_blocked,
            traps_triggered,
            captchas_issued,
            captchas_solved,
            responses_obfuscated,
            geoip_blocked,
            geoip_lookups,
        }
    }
}

impl AppState {
    /// Create a new AppState from the given configuration.
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: RwLock::new(config),
            metrics: WafMetrics::new(),
            audit_log: RwLock::new(Vec::new()),
            custom_rules: RwLock::new(Vec::new()),
            start_time: std::time::Instant::now(),
        }
    }
}
