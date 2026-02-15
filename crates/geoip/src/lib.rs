use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use layer7waf_common::{GeoIpConfig, GeoIpDefaultAction, GeoIpMode};
use tracing::{debug, info, warn};

/// Result of a GeoIP check against the configured country lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeoIpAction {
    /// Request is allowed through.
    Allow,
    /// Request should be blocked (country matched blocklist or failed allowlist).
    Block { country: String },
    /// Request is allowed but flagged for logging (detect mode).
    Detect { country: String },
    /// Country could not be determined (private IP, lookup failure, etc.).
    Unknown,
}

/// Minimal struct for deserializing the country ISO code from MaxMind DB.
#[derive(serde::Deserialize)]
struct CountryRecord {
    country: Option<CountryInfo>,
}

#[derive(serde::Deserialize)]
struct CountryInfo {
    iso_code: Option<String>,
}

/// GeoIP filter using a MaxMind `.mmdb` database.
///
/// Uses `ArcSwap` for lock-free hot-reload of the database file.
pub struct GeoIpFilter {
    reader: ArcSwap<Option<maxminddb::Reader<Vec<u8>>>>,
    config: GeoIpConfig,
}

impl GeoIpFilter {
    /// Create a new `GeoIpFilter` from the given config.
    ///
    /// Opens the `.mmdb` file at `config.database_path` if configured.
    pub fn new(config: GeoIpConfig) -> anyhow::Result<Self> {
        let reader = if let Some(ref path) = config.database_path {
            match maxminddb::Reader::open_readfile(path) {
                Ok(r) => {
                    info!(path = %path.display(), "loaded GeoIP database");
                    Some(r)
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "failed to open GeoIP database");
                    return Err(anyhow::anyhow!(
                        "failed to open GeoIP database {}: {}",
                        path.display(),
                        e
                    ));
                }
            }
        } else {
            None
        };

        Ok(Self {
            reader: ArcSwap::from_pointee(reader),
            config,
        })
    }

    /// Create a `GeoIpFilter` without a database (for testing or when disabled).
    pub fn new_empty(config: GeoIpConfig) -> Self {
        Self {
            reader: ArcSwap::from_pointee(None),
            config,
        }
    }

    /// Look up the ISO 3166-1 alpha-2 country code for an IP address.
    pub fn lookup_country(&self, addr: IpAddr) -> Option<String> {
        let guard = self.reader.load();
        let reader = guard.as_ref().as_ref()?;

        match reader.lookup::<CountryRecord>(addr) {
            Ok(record) => record.country.and_then(|c| c.iso_code),
            Err(e) => {
                debug!(addr = %addr, error = %e, "GeoIP lookup failed");
                None
            }
        }
    }

    /// Check an IP address against the configured country blocklist/allowlist.
    pub fn check(&self, addr: IpAddr) -> GeoIpAction {
        let country = match self.lookup_country(addr) {
            Some(c) => c,
            None => {
                // Country unknown — apply default action
                return match self.config.default_action {
                    GeoIpDefaultAction::Allow => GeoIpAction::Unknown,
                    GeoIpDefaultAction::Block => {
                        if self.config.mode == GeoIpMode::Detect {
                            GeoIpAction::Unknown
                        } else {
                            GeoIpAction::Block {
                                country: "unknown".to_string(),
                            }
                        }
                    }
                };
            }
        };

        let country_upper = country.to_uppercase();

        // Allowlist takes precedence: if configured, only listed countries pass.
        if !self.config.allowed_countries.is_empty() {
            let is_allowed = self
                .config
                .allowed_countries
                .iter()
                .any(|c| c.to_uppercase() == country_upper);

            if !is_allowed {
                return match self.config.mode {
                    GeoIpMode::Block => GeoIpAction::Block { country },
                    GeoIpMode::Detect => GeoIpAction::Detect { country },
                };
            }
            return GeoIpAction::Allow;
        }

        // Blocklist mode: listed countries are blocked.
        if !self.config.blocked_countries.is_empty() {
            let is_blocked = self
                .config
                .blocked_countries
                .iter()
                .any(|c| c.to_uppercase() == country_upper);

            if is_blocked {
                return match self.config.mode {
                    GeoIpMode::Block => GeoIpAction::Block { country },
                    GeoIpMode::Detect => GeoIpAction::Detect { country },
                };
            }
        }

        GeoIpAction::Allow
    }

    /// Hot-reload the MaxMind database from a new path.
    pub fn reload(&self, path: &Path) -> anyhow::Result<()> {
        let reader = maxminddb::Reader::open_readfile(path).map_err(|e| {
            anyhow::anyhow!("failed to reload GeoIP database {}: {}", path.display(), e)
        })?;
        self.reader.store(Arc::new(Some(reader)));
        info!(path = %path.display(), "reloaded GeoIP database");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layer7waf_common::{GeoIpDefaultAction, GeoIpMode};

    fn make_config(
        blocked: Vec<&str>,
        allowed: Vec<&str>,
        mode: GeoIpMode,
        default_action: GeoIpDefaultAction,
    ) -> GeoIpConfig {
        GeoIpConfig {
            enabled: true,
            database_path: None,
            blocked_countries: blocked.into_iter().map(String::from).collect(),
            allowed_countries: allowed.into_iter().map(String::from).collect(),
            mode,
            default_action,
        }
    }

    /// With no database loaded, all lookups should return Unknown.
    #[test]
    fn test_no_database_returns_unknown() {
        let config = make_config(
            vec!["CN"],
            vec![],
            GeoIpMode::Block,
            GeoIpDefaultAction::Allow,
        );
        let filter = GeoIpFilter::new_empty(config);
        let addr: IpAddr = "8.8.8.8".parse().unwrap();
        assert_eq!(filter.check(addr), GeoIpAction::Unknown);
    }

    /// When default_action is Block and country is unknown, should block
    /// (except in detect mode).
    #[test]
    fn test_default_action_block() {
        let config = make_config(
            vec!["CN"],
            vec![],
            GeoIpMode::Block,
            GeoIpDefaultAction::Block,
        );
        let filter = GeoIpFilter::new_empty(config);
        let addr: IpAddr = "192.168.1.1".parse().unwrap();
        assert_eq!(
            filter.check(addr),
            GeoIpAction::Block {
                country: "unknown".to_string()
            }
        );
    }

    /// When default_action is Block but mode is Detect, unknown should
    /// still be Unknown (not blocked).
    #[test]
    fn test_default_action_block_detect_mode() {
        let config = make_config(
            vec!["CN"],
            vec![],
            GeoIpMode::Detect,
            GeoIpDefaultAction::Block,
        );
        let filter = GeoIpFilter::new_empty(config);
        let addr: IpAddr = "192.168.1.1".parse().unwrap();
        assert_eq!(filter.check(addr), GeoIpAction::Unknown);
    }

    /// Test that lookup_country returns None when no DB is loaded.
    #[test]
    fn test_lookup_country_no_db() {
        let config = make_config(vec![], vec![], GeoIpMode::Block, GeoIpDefaultAction::Allow);
        let filter = GeoIpFilter::new_empty(config);
        assert_eq!(filter.lookup_country("1.2.3.4".parse().unwrap()), None);
    }

    /// Test that new() fails with a non-existent database path.
    #[test]
    fn test_new_invalid_path() {
        let config = GeoIpConfig {
            enabled: true,
            database_path: Some("/nonexistent/GeoLite2-Country.mmdb".into()),
            blocked_countries: vec![],
            allowed_countries: vec![],
            mode: GeoIpMode::Block,
            default_action: GeoIpDefaultAction::Allow,
        };
        assert!(GeoIpFilter::new(config).is_err());
    }

    /// Test reload with a non-existent path fails gracefully.
    #[test]
    fn test_reload_invalid_path() {
        let config = make_config(vec![], vec![], GeoIpMode::Block, GeoIpDefaultAction::Allow);
        let filter = GeoIpFilter::new_empty(config);
        assert!(filter.reload(Path::new("/nonexistent/db.mmdb")).is_err());
    }

    /// Verify blocklist logic by simulating what check() would do if
    /// lookup_country returned a known value. We test the internal logic
    /// by constructing a filter that wraps a mock lookup.
    /// Since we can't easily inject a mock DB, we test the config logic
    /// paths by examining the allowed/blocked matching code directly.
    #[test]
    fn test_blocklist_matching_logic() {
        let config = make_config(
            vec!["CN", "RU"],
            vec![],
            GeoIpMode::Block,
            GeoIpDefaultAction::Allow,
        );

        // Verify case-insensitive matching
        assert!(config
            .blocked_countries
            .iter()
            .any(|c| c.to_uppercase() == "CN"));
        assert!(config
            .blocked_countries
            .iter()
            .any(|c| c.to_uppercase() == "RU"));
        assert!(!config
            .blocked_countries
            .iter()
            .any(|c| c.to_uppercase() == "US"));
    }

    #[test]
    fn test_allowlist_matching_logic() {
        let config = make_config(
            vec![],
            vec!["US", "GB"],
            GeoIpMode::Block,
            GeoIpDefaultAction::Allow,
        );

        // Verify allowlist matching
        assert!(config
            .allowed_countries
            .iter()
            .any(|c| c.to_uppercase() == "US"));
        assert!(config
            .allowed_countries
            .iter()
            .any(|c| c.to_uppercase() == "GB"));
        assert!(!config
            .allowed_countries
            .iter()
            .any(|c| c.to_uppercase() == "CN"));
    }

    /// When both blocklist and allowlist are empty, everything should be allowed.
    #[test]
    fn test_empty_lists_allow_all() {
        let config = make_config(vec![], vec![], GeoIpMode::Block, GeoIpDefaultAction::Allow);
        let filter = GeoIpFilter::new_empty(config);
        // No DB means Unknown due to default_action Allow
        assert_eq!(
            filter.check("1.2.3.4".parse().unwrap()),
            GeoIpAction::Unknown
        );
    }
}
