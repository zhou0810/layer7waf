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
