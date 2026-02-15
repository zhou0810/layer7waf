use async_trait::async_trait;
use bytes::Bytes;
use http::StatusCode;
use layer7waf_bot_detect::{BotCheckResult, BotDetector};
use layer7waf_common::{AppConfig, WafMode};
use layer7waf_coraza::{WafAction, WafEngine, WafTransaction};
use layer7waf_ip_reputation::IpReputation;
use layer7waf_rate_limit::RateLimiter;
use pingora_core::prelude::*;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use prometheus::{HistogramVec, IntCounter, IntCounterVec, Registry};
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};

use crate::context::{BlockReason, RequestContext};
use crate::upstream::UpstreamSelector;

pub struct Layer7WafProxy {
    pub config: Arc<RwLock<AppConfig>>,
    pub waf_engine: Option<Arc<WafEngine>>,
    pub upstreams: Vec<UpstreamSelector>,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub ip_reputation: Arc<IpReputation>,
    pub bot_detector: Option<Arc<BotDetector>>,
    pub metrics: Arc<ProxyMetrics>,
}

pub struct ProxyMetrics {
    pub registry: Registry,
    pub requests_total: IntCounter,
    pub requests_blocked: IntCounter,
    pub requests_rate_limited: IntCounter,
    pub request_duration: HistogramVec,
    pub rule_hits: IntCounterVec,
    pub bots_detected: IntCounter,
    pub challenges_issued: IntCounter,
    pub challenges_solved: IntCounter,
}

impl ProxyMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total =
            IntCounter::new("layer7waf_requests_total", "Total requests processed").unwrap();
        let requests_blocked =
            IntCounter::new("layer7waf_requests_blocked", "Total requests blocked by WAF")
                .unwrap();
        let requests_rate_limited = IntCounter::new(
            "layer7waf_requests_rate_limited",
            "Total requests rate limited",
        )
        .unwrap();
        let request_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "layer7waf_request_duration_seconds",
                "Request duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0]),
            &["upstream"],
        )
        .unwrap();
        let rule_hits = IntCounterVec::new(
            prometheus::Opts::new("layer7waf_rule_hits_total", "WAF rule hit counts"),
            &["rule_id"],
        )
        .unwrap();

        let bots_detected =
            IntCounter::new("layer7waf_bots_detected", "Total bots detected").unwrap();
        let challenges_issued =
            IntCounter::new("layer7waf_challenges_issued", "Total JS challenges issued").unwrap();
        let challenges_solved =
            IntCounter::new("layer7waf_challenges_solved", "Total JS challenges solved").unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry
            .register(Box::new(requests_blocked.clone()))
            .unwrap();
        registry
            .register(Box::new(requests_rate_limited.clone()))
            .unwrap();
        registry
            .register(Box::new(request_duration.clone()))
            .unwrap();
        registry.register(Box::new(rule_hits.clone())).unwrap();
        registry.register(Box::new(bots_detected.clone())).unwrap();
        registry
            .register(Box::new(challenges_issued.clone()))
            .unwrap();
        registry
            .register(Box::new(challenges_solved.clone()))
            .unwrap();

        Self {
            registry,
            requests_total,
            requests_blocked,
            requests_rate_limited,
            request_duration,
            rule_hits,
            bots_detected,
            challenges_issued,
            challenges_solved,
        }
    }
}

impl Layer7WafProxy {
    pub fn new(config: AppConfig) -> Self {
        // Build upstream selectors
        let upstreams: Vec<UpstreamSelector> = config
            .upstreams
            .iter()
            .map(UpstreamSelector::from_config)
            .collect();

        // Initialize WAF engine if rules are configured
        let waf_engine = if !config.waf.rules.is_empty() {
            let directives = build_waf_directives(&config);
            match WafEngine::new(&directives) {
                Ok(engine) => {
                    info!("WAF engine initialized with {} rule patterns", config.waf.rules.len());
                    Some(Arc::new(engine))
                }
                Err(e) => {
                    error!("failed to initialize WAF engine: {}", e);
                    None
                }
            }
        } else {
            info!("no WAF rules configured, WAF engine disabled");
            None
        };

        // Initialize rate limiter
        let rate_limiter = if config.rate_limit.enabled {
            let limiter = RateLimiter::new_token_bucket(
                config.rate_limit.default_rps,
                config.rate_limit.default_burst,
            );
            limiter.start_cleanup_task();
            info!(
                rps = config.rate_limit.default_rps,
                burst = config.rate_limit.default_burst,
                "rate limiter enabled"
            );
            Some(Arc::new(limiter))
        } else {
            None
        };

        // Initialize IP reputation
        let ip_reputation = Arc::new(IpReputation::new());
        if let Some(ref path) = config.ip_reputation.blocklist {
            match ip_reputation.load_blocklist(path) {
                Ok(count) => info!(count, path = %path.display(), "loaded IP blocklist"),
                Err(e) => warn!(error = %e, "failed to load IP blocklist"),
            }
        }
        if let Some(ref path) = config.ip_reputation.allowlist {
            match ip_reputation.load_allowlist(path) {
                Ok(count) => info!(count, path = %path.display(), "loaded IP allowlist"),
                Err(e) => warn!(error = %e, "failed to load IP allowlist"),
            }
        }

        // Initialize bot detector
        let bot_detector = if config.bot_detection.enabled {
            info!(
                mode = ?config.bot_detection.mode,
                threshold = config.bot_detection.score_threshold,
                "bot detection enabled"
            );
            Some(Arc::new(BotDetector::new(config.bot_detection.clone())))
        } else {
            None
        };

        let metrics = Arc::new(ProxyMetrics::new());

        Self {
            config: Arc::new(RwLock::new(config)),
            waf_engine,
            upstreams,
            rate_limiter,
            ip_reputation,
            bot_detector,
            metrics,
        }
    }

    fn find_route(&self, host: Option<&str>, path: &str) -> Option<usize> {
        let config = self.config.read().unwrap();
        for (i, route) in config.routes.iter().enumerate() {
            let host_match = match (&route.host, host) {
                (Some(route_host), Some(req_host)) => req_host == route_host.as_str(),
                (Some(_), None) => false,
                (None, _) => true, // wildcard host
            };

            if host_match && path.starts_with(&route.path_prefix) {
                return Some(i);
            }
        }
        None
    }

    fn find_upstream(&self, name: &str) -> Option<&UpstreamSelector> {
        self.upstreams.iter().find(|u| u.name == name)
    }
}

#[async_trait]
impl ProxyHttp for Layer7WafProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        self.metrics.requests_total.inc();

        // Extract request info
        let header = session.req_header();
        ctx.method = header.method.as_str().to_string();
        ctx.uri = header.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/").to_string();

        // Extract client IP from X-Forwarded-For or socket
        ctx.client_ip = session
            .req_header()
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| {
                session
                    .client_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_default()
            });

        // Remove port from IP if present
        if let Some(ip_part) = ctx.client_ip.rsplit_once(':') {
            if ctx.client_ip.starts_with('[') || !ctx.client_ip.contains('.') {
                // IPv6 - keep as is
            } else {
                ctx.client_ip = ip_part.0.to_string();
            }
        }

        let host = session
            .req_header()
            .headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Route matching
        let path = session
            .req_header()
            .uri
            .path()
            .to_string();
        ctx.route_index = self.find_route(host.as_deref(), &path);

        // 1. IP reputation check
        if let Ok(addr) = ctx.client_ip.parse() {
            match self.ip_reputation.check(addr) {
                layer7waf_ip_reputation::IpAction::Block => {
                    info!(client_ip = %ctx.client_ip, "request blocked by IP blocklist");
                    ctx.block_reason = Some(BlockReason::IpBlocked);
                    self.metrics.requests_blocked.inc();
                    let mut resp = ResponseHeader::build(StatusCode::FORBIDDEN, Some(4)).unwrap();
                    resp.insert_header("content-type", "text/plain").unwrap();
                    session.set_keepalive(None);
                    session
                        .write_response_header(Box::new(resp), false)
                        .await?;
                    session
                        .write_response_body(Some(Bytes::from("Forbidden: IP blocked\n")), true)
                        .await?;
                    return Ok(true);
                }
                layer7waf_ip_reputation::IpAction::Allow => {
                    debug!(client_ip = %ctx.client_ip, "IP allowlisted, skipping checks");
                    return Ok(false);
                }
                layer7waf_ip_reputation::IpAction::None => {}
            }
        }

        // 2. Rate limiting
        if let Some(ref limiter) = self.rate_limiter {
            if !limiter.check(&ctx.client_ip) {
                info!(client_ip = %ctx.client_ip, "request rate limited");
                ctx.block_reason = Some(BlockReason::RateLimit);
                self.metrics.requests_rate_limited.inc();
                self.metrics.requests_blocked.inc();
                let mut resp =
                    ResponseHeader::build(StatusCode::TOO_MANY_REQUESTS, Some(4)).unwrap();
                resp.insert_header("content-type", "text/plain").unwrap();
                resp.insert_header("retry-after", "1").unwrap();
                session.set_keepalive(None);
                session
                    .write_response_header(Box::new(resp), false)
                    .await?;
                session
                    .write_response_body(Some(Bytes::from("Rate limit exceeded\n")), true)
                    .await?;
                return Ok(true);
            }
        }

        // 2.5 Bot detection
        if let Some(ref detector) = self.bot_detector {
            let headers: Vec<(String, String)> = session
                .req_header()
                .headers
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            let cookie_header = session
                .req_header()
                .headers
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let result = detector.check(
                &ctx.client_ip,
                &headers,
                &ctx.method,
                cookie_header.as_deref(),
            );

            match result {
                BotCheckResult::Block => {
                    info!(client_ip = %ctx.client_ip, "request blocked by bot detection");
                    ctx.block_reason = Some(BlockReason::BotDetected { score: 1.0 });
                    self.metrics.bots_detected.inc();
                    self.metrics.requests_blocked.inc();
                    let mut resp =
                        ResponseHeader::build(StatusCode::FORBIDDEN, Some(4)).unwrap();
                    resp.insert_header("content-type", "text/plain").unwrap();
                    session.set_keepalive(None);
                    session
                        .write_response_header(Box::new(resp), false)
                        .await?;
                    session
                        .write_response_body(Some(Bytes::from("Forbidden: Bot detected\n")), true)
                        .await?;
                    return Ok(true);
                }
                BotCheckResult::Challenge(html) => {
                    info!(client_ip = %ctx.client_ip, "issuing JS challenge for bot detection");
                    self.metrics.challenges_issued.inc();
                    let body_bytes = Bytes::from(html);
                    let mut resp =
                        ResponseHeader::build(StatusCode::OK, Some(4)).unwrap();
                    resp.insert_header("content-type", "text/html; charset=utf-8")
                        .unwrap();
                    resp.insert_header("cache-control", "no-store").unwrap();
                    session.set_keepalive(None);
                    session
                        .write_response_header(Box::new(resp), false)
                        .await?;
                    session
                        .write_response_body(Some(body_bytes), true)
                        .await?;
                    return Ok(true);
                }
                BotCheckResult::Detect { score } => {
                    ctx.bot_score = Some(score);
                    if score >= 0.7 {
                        self.metrics.bots_detected.inc();
                    }
                    debug!(client_ip = %ctx.client_ip, score, "bot detection score (detect mode)");
                }
                BotCheckResult::Allow => {
                    // Check if this was a solved challenge (cookie present means solved)
                    if cookie_header
                        .as_deref()
                        .map(|c| c.contains("__l7w_bc="))
                        .unwrap_or(false)
                    {
                        self.metrics.challenges_solved.inc();
                    }
                }
            }
        }

        // 3. WAF check (request headers phase)
        let waf_mode = ctx.route_index.and_then(|i| {
            let config = self.config.read().unwrap();
            config.routes.get(i).map(|r| r.waf.clone())
        });

        if let Some(ref waf_config) = waf_mode {
            if waf_config.enabled && waf_config.mode != WafMode::Off {
                if let Some(ref engine) = self.waf_engine {
                    let tx = WafTransaction::new(engine);

                    // Collect headers
                    let headers: Vec<(String, String)> = session
                        .req_header()
                        .headers
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.as_str().to_string(),
                                v.to_str().unwrap_or("").to_string(),
                            )
                        })
                        .collect();

                    let protocol = format!(
                        "HTTP/{}",
                        if session.req_header().version == http::Version::HTTP_2 {
                            "2.0"
                        } else {
                            "1.1"
                        }
                    );

                    let action =
                        tx.process_request_headers(&ctx.method, &ctx.uri, &protocol, &headers);

                    match action {
                        WafAction::Block { status } if waf_config.mode == WafMode::Block => {
                            info!(
                                client_ip = %ctx.client_ip,
                                uri = %ctx.uri,
                                status,
                                "request blocked by WAF"
                            );
                            ctx.block_reason = Some(BlockReason::Waf { status });
                            self.metrics.requests_blocked.inc();
                            let code = StatusCode::from_u16(status)
                                .unwrap_or(StatusCode::FORBIDDEN);
                            let mut resp =
                                ResponseHeader::build(code, Some(4)).unwrap();
                            resp.insert_header("content-type", "text/plain").unwrap();
                            session.set_keepalive(None);
                            session
                                .write_response_header(Box::new(resp), false)
                                .await?;
                            session
                                .write_response_body(
                                    Some(Bytes::from("Forbidden: WAF rule triggered\n")),
                                    true,
                                )
                                .await?;
                            return Ok(true);
                        }
                        WafAction::Block { status } => {
                            // Detect mode: log but don't block
                            warn!(
                                client_ip = %ctx.client_ip,
                                uri = %ctx.uri,
                                status,
                                "WAF rule triggered (detect mode, not blocking)"
                            );
                        }
                        WafAction::Redirect { status, ref url } => {
                            if waf_config.mode == WafMode::Block {
                                let code = StatusCode::from_u16(status)
                                    .unwrap_or(StatusCode::FOUND);
                                let mut resp =
                                    ResponseHeader::build(code, Some(4)).unwrap();
                                resp.insert_header("location", url).unwrap();
                                session.set_keepalive(None);
                                session
                                    .write_response_header(Box::new(resp), false)
                                    .await?;
                                session
                                    .write_response_body(None, true)
                                    .await?;
                                return Ok(true);
                            }
                        }
                        WafAction::Pass => {}
                    }

                    ctx.waf_tx = Some(tx);
                }
            }
        }

        Ok(false) // continue to upstream
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let config = self.config.read().unwrap();
        let upstream_name = ctx
            .route_index
            .and_then(|i| config.routes.get(i))
            .map(|r| r.upstream.as_str())
            .unwrap_or_else(|| {
                config
                    .routes
                    .first()
                    .map(|r| r.upstream.as_str())
                    .unwrap_or("backend")
            });

        let addr = self
            .find_upstream(upstream_name)
            .and_then(|u| u.select())
            .ok_or_else(|| {
                Error::new(ErrorType::ConnectProxyFailure)
            })?;

        debug!(upstream = upstream_name, addr, "selected upstream peer");

        // Parse addr into host:port
        let peer = HttpPeer::new(addr, false, String::new());
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Add X-Forwarded-For header
        if !ctx.client_ip.is_empty() {
            upstream_request
                .insert_header("x-real-ip", &ctx.client_ip)
                .unwrap();
        }
        // Add X-Request-ID for tracing
        upstream_request
            .insert_header("x-waf-processed", "true")
            .unwrap();
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        ctx.response_status = upstream_response.status.as_u16();

        // WAF response phase check
        if let Some(ref tx) = ctx.waf_tx {
            let headers: Vec<(String, String)> = upstream_response
                .headers
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            let action =
                tx.process_response_headers(upstream_response.status.as_u16(), &headers);

            match action {
                WafAction::Block { status } => {
                    warn!(
                        client_ip = %ctx.client_ip,
                        uri = %ctx.uri,
                        status,
                        "response blocked by WAF"
                    );
                    ctx.block_reason = Some(BlockReason::Waf { status });
                    self.metrics.requests_blocked.inc();
                }
                _ => {}
            }
        }

        // Add security headers
        upstream_response
            .insert_header("x-content-type-options", "nosniff")
            .unwrap();
        upstream_response
            .insert_header("x-frame-options", "DENY")
            .unwrap();

        Ok(())
    }

    async fn logging(&self, _session: &mut Session, _error: Option<&pingora_core::Error>, ctx: &mut Self::CTX) {
        let duration = ctx.request_start.elapsed();
        let duration_secs = duration.as_secs_f64();

        // Record duration metric
        let upstream_label = ctx
            .route_index
            .and_then(|i| {
                let config = self.config.read().unwrap();
                config.routes.get(i).map(|r| r.upstream.clone())
            })
            .unwrap_or_else(|| "unknown".to_string());
        self.metrics
            .request_duration
            .with_label_values(&[&upstream_label])
            .observe(duration_secs);

        // Structured log
        let blocked = ctx.block_reason.is_some();
        info!(
            client_ip = %ctx.client_ip,
            method = %ctx.method,
            uri = %ctx.uri,
            status = ctx.response_status,
            duration_ms = duration.as_millis() as u64,
            blocked,
            block_reason = ?ctx.block_reason,
            "request completed"
        );

        // Clean up WAF transaction (Drop will handle it)
        ctx.waf_tx.take();
    }
}

/// Build WAF directives string from config rule glob patterns.
fn build_waf_directives(config: &AppConfig) -> String {
    let mut directives = String::new();

    // Add SecRuleEngine
    directives.push_str("SecRuleEngine On\n");

    // Expand glob patterns and include rule files
    for pattern in &config.waf.rules {
        match glob::glob(pattern) {
            Ok(paths) => {
                for entry in paths.flatten() {
                    directives.push_str(&format!("Include {}\n", entry.display()));
                }
            }
            Err(e) => {
                warn!(pattern = %pattern, error = %e, "invalid rule glob pattern");
            }
        }
    }

    // Set request body limit
    directives.push_str(&format!(
        "SecRequestBodyLimit {}\n",
        config.waf.request_body_limit
    ));

    directives
}
