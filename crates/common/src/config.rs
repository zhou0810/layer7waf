use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level WAF configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub upstreams: Vec<UpstreamConfig>,
    pub routes: Vec<RouteConfig>,
    pub waf: WafConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub ip_reputation: IpReputationConfig,
    #[serde(default)]
    pub bot_detection: BotDetectionConfig,
    #[serde(default)]
    pub anti_scraping: AntiScrapingConfig,
    #[serde(default)]
    pub geoip: GeoIpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen: Vec<String>,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    #[serde(default)]
    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "default_admin_listen")]
    pub listen: String,
    #[serde(default = "default_true")]
    pub dashboard: bool,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            listen: default_admin_listen(),
            dashboard: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    pub servers: Vec<UpstreamServer>,
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamServer {
    pub addr: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_health_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_health_path")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default = "default_path_prefix")]
    pub path_prefix: String,
    pub upstream: String,
    #[serde(default)]
    pub waf: RouteWafConfig,
    #[serde(default)]
    pub rate_limit: Option<RouteRateLimitConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteWafConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_waf_mode")]
    pub mode: WafMode,
}

impl Default for RouteWafConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: WafMode::Block,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WafMode {
    Block,
    Detect,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRateLimitConfig {
    pub rps: u64,
    pub burst: u64,
    #[serde(default = "default_rate_limit_algorithm")]
    pub algorithm: RateLimitAlgorithm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitAlgorithm {
    TokenBucket,
    SlidingWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default = "default_body_limit")]
    pub request_body_limit: usize,
    #[serde(default)]
    pub audit_log: AuditLogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_audit_log_path")]
    pub path: PathBuf,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: default_audit_log_path(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rps")]
    pub default_rps: u64,
    #[serde(default = "default_burst")]
    pub default_burst: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_rps: default_rps(),
            default_burst: default_burst(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputationConfig {
    #[serde(default)]
    pub blocklist: Option<PathBuf>,
    #[serde(default)]
    pub allowlist: Option<PathBuf>,
}

impl Default for IpReputationConfig {
    fn default() -> Self {
        Self {
            blocklist: None,
            allowlist: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotDetectionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_bot_detection_mode")]
    pub mode: BotDetectionMode,
    #[serde(default)]
    pub js_challenge: JsChallengeConfig,
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
    #[serde(default)]
    pub known_bots_allowlist: Vec<String>,
}

impl Default for BotDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: BotDetectionMode::Challenge,
            js_challenge: JsChallengeConfig::default(),
            score_threshold: default_score_threshold(),
            known_bots_allowlist: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BotDetectionMode {
    Block,
    Challenge,
    Detect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsChallengeConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_challenge_difficulty")]
    pub difficulty: u32,
    #[serde(default = "default_challenge_ttl")]
    pub ttl_secs: u64,
    #[serde(default = "default_challenge_secret")]
    pub secret: String,
}

impl Default for JsChallengeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            difficulty: default_challenge_difficulty(),
            ttl_secs: default_challenge_ttl(),
            secret: default_challenge_secret(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiScrapingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_anti_scraping_mode")]
    pub mode: AntiScrapingMode,
    #[serde(default)]
    pub captcha: CaptchaConfig,
    #[serde(default)]
    pub honeypot: HoneypotConfig,
    #[serde(default)]
    pub obfuscation: ObfuscationConfig,
    #[serde(default = "default_scraping_score_threshold")]
    pub score_threshold: f64,
}

impl Default for AntiScrapingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AntiScrapingMode::Detect,
            captcha: CaptchaConfig::default(),
            honeypot: HoneypotConfig::default(),
            obfuscation: ObfuscationConfig::default(),
            score_threshold: default_scraping_score_threshold(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AntiScrapingMode {
    Block,
    Challenge,
    Detect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_captcha_ttl")]
    pub ttl_secs: u64,
    #[serde(default = "default_challenge_secret")]
    pub secret: String,
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_secs: default_captcha_ttl(),
            secret: default_challenge_secret(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoneypotConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_trap_path_prefix")]
    pub trap_path_prefix: String,
}

impl Default for HoneypotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trap_path_prefix: default_trap_path_prefix(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfuscationConfig {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub database_path: Option<PathBuf>,
    #[serde(default)]
    pub blocked_countries: Vec<String>,
    #[serde(default)]
    pub allowed_countries: Vec<String>,
    #[serde(default = "default_geoip_mode")]
    pub mode: GeoIpMode,
    #[serde(default = "default_geoip_default_action")]
    pub default_action: GeoIpDefaultAction,
}

impl Default for GeoIpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            database_path: None,
            blocked_countries: vec![],
            allowed_countries: vec![],
            mode: GeoIpMode::Block,
            default_action: GeoIpDefaultAction::Allow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeoIpMode {
    Block,
    Detect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeoIpDefaultAction {
    Allow,
    Block,
}

// Default value helpers
fn default_admin_listen() -> String {
    "127.0.0.1:9090".to_string()
}
fn default_true() -> bool {
    true
}
fn default_weight() -> u32 {
    1
}
fn default_health_interval() -> u64 {
    10
}
fn default_health_path() -> String {
    "/health".to_string()
}
fn default_path_prefix() -> String {
    "/".to_string()
}
fn default_waf_mode() -> WafMode {
    WafMode::Block
}
fn default_rate_limit_algorithm() -> RateLimitAlgorithm {
    RateLimitAlgorithm::TokenBucket
}
fn default_body_limit() -> usize {
    13_107_200 // ~12.5 MB
}
fn default_audit_log_path() -> PathBuf {
    PathBuf::from("/var/log/layer7waf/audit.log")
}
fn default_rps() -> u64 {
    100
}
fn default_burst() -> u64 {
    200
}
fn default_bot_detection_mode() -> BotDetectionMode {
    BotDetectionMode::Challenge
}
fn default_score_threshold() -> f64 {
    0.7
}
fn default_challenge_difficulty() -> u32 {
    16
}
fn default_challenge_ttl() -> u64 {
    3600
}
fn default_anti_scraping_mode() -> AntiScrapingMode {
    AntiScrapingMode::Detect
}
fn default_scraping_score_threshold() -> f64 {
    0.6
}
fn default_captcha_ttl() -> u64 {
    1800
}
fn default_trap_path_prefix() -> String {
    "/.well-known/l7w-trap".to_string()
}
fn default_geoip_mode() -> GeoIpMode {
    GeoIpMode::Block
}
fn default_geoip_default_action() -> GeoIpDefaultAction {
    GeoIpDefaultAction::Allow
}
fn default_challenge_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("l7w-{:x}", ts)
}

impl AppConfig {
    /// Load configuration from a YAML file.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration for consistency.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.server.listen.is_empty() {
            anyhow::bail!("server.listen must have at least one address");
        }

        for route in &self.routes {
            let upstream_exists = self.upstreams.iter().any(|u| u.name == route.upstream);
            if !upstream_exists {
                anyhow::bail!(
                    "route references unknown upstream '{}' (host={:?}, path={})",
                    route.upstream,
                    route.host,
                    route.path_prefix
                );
            }
        }

        for upstream in &self.upstreams {
            if upstream.servers.is_empty() {
                anyhow::bail!("upstream '{}' has no servers", upstream.name);
            }
        }

        Ok(())
    }
}
