use layer7waf_coraza::WafTransaction;
use std::time::Instant;

/// Per-request context carried through the Pingora proxy pipeline.
pub struct RequestContext {
    /// Coraza WAF transaction for this request.
    pub waf_tx: Option<WafTransaction>,

    /// Matched route index (into the config's routes vec).
    pub route_index: Option<usize>,

    /// Client IP address string.
    pub client_ip: String,

    /// Request start time for latency measurement.
    pub request_start: Instant,

    /// Whether the request was blocked (and by what).
    pub block_reason: Option<BlockReason>,

    /// HTTP method (cached for logging).
    pub method: String,

    /// Request URI (cached for logging).
    pub uri: String,

    /// Response status code (set during response phase).
    pub response_status: u16,

    /// Bot detection score (set during request phase).
    pub bot_score: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum BlockReason {
    Waf { status: u16 },
    RateLimit,
    IpBlocked,
    BotDetected { score: f64 },
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            waf_tx: None,
            route_index: None,
            client_ip: String::new(),
            request_start: Instant::now(),
            block_reason: None,
            method: String::new(),
            uri: String::new(),
            response_status: 0,
            bot_score: None,
        }
    }
}
