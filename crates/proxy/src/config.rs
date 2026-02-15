use anyhow::Result;
use layer7waf_common::AppConfig;
use std::path::PathBuf;
use tracing::info;

/// Resolved configuration with the source path for reloading.
pub struct ProxyConfig {
    pub config: AppConfig,
    pub config_path: PathBuf,
}

impl ProxyConfig {
    pub fn load(path: &str) -> Result<Self> {
        info!(path = path, "loading configuration");
        let config = AppConfig::load(path)?;
        Ok(Self {
            config,
            config_path: PathBuf::from(path),
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        let path_str = self.config_path.to_string_lossy().to_string();
        info!(path = %path_str, "reloading configuration");
        self.config = AppConfig::load(&path_str)?;
        Ok(())
    }
}
