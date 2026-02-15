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

    /// Anti-scraping score (set during request phase).
    pub scraping_score: Option<f64>,

    /// Whether the request hit a honeypot trap.
    pub is_trap_request: bool,

    /// Whether the response body should be processed for honeypot/obfuscation injection.
    pub should_process_response: bool,

    /// Content-Type of the upstream response.
    pub response_content_type: Option<String>,

    /// Buffer for collecting response body chunks for rewriting.
    pub response_body_buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum BlockReason {
    Waf { status: u16 },
    RateLimit,
    IpBlocked,
    BotDetected { score: f64 },
    ScraperDetected { score: f64 },
    HoneypotTriggered,
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
            scraping_score: None,
            is_trap_request: false,
            should_process_response: false,
            response_content_type: None,
            response_body_buffer: Vec::new(),
        }
    }
}
