mod config;
mod context;
mod service;
mod upstream;

use anyhow::Result;
use pingora_core::server::Server;
use pingora_proxy::http_proxy_service;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::ProxyConfig;
use crate::service::Layer7WafProxy;

fn main() -> Result<()> {
    // Initialize tracing
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .json()
        .init();

    // Parse command-line args for config path
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/layer7waf.yaml".to_string());

    info!(config_path = %config_path, "starting Layer 7 WAF");

    // Load configuration
    let proxy_config = ProxyConfig::load(&config_path)?;
    let app_config = proxy_config.config.clone();

    // Create Pingora server
    let mut server = Server::new(None)?;
    server.bootstrap();

    // Create the WAF proxy service
    let waf_proxy = Layer7WafProxy::new(app_config.clone());
    let _metrics = waf_proxy.metrics.clone();

    let mut proxy_service = http_proxy_service(&server.configuration, waf_proxy);

    // Add listeners from config
    for listen_addr in &app_config.server.listen {
        info!(addr = %listen_addr, "adding listener");
        proxy_service.add_tcp(listen_addr);
    }

    // Add TLS if configured
    if let Some(ref tls) = app_config.server.tls {
        let cert_path = tls.cert.to_string_lossy().to_string();
        let key_path = tls.key.to_string_lossy().to_string();
        info!(cert = %cert_path, key = %key_path, "TLS configured");
        // TLS listeners would be added here with pingora TLS support
    }

    server.add_service(proxy_service);

    // Launch admin API in background
    let admin_listen = app_config.server.admin.listen.clone();
    let admin_config = app_config.clone();

    server.add_service(pingora_core::services::background::background_service(
        "admin API",
        AdminBackgroundService {
            listen_addr: admin_listen,
            config: admin_config,
        },
    ));

    info!("Layer 7 WAF started successfully");
    server.run_forever();
}

/// Background service to run the admin API alongside Pingora.
struct AdminBackgroundService {
    listen_addr: String,
    config: layer7waf_common::AppConfig,
}

#[async_trait::async_trait]
impl pingora_core::services::background::BackgroundService for AdminBackgroundService {
    async fn start(&self, mut shutdown: pingora_core::server::ShutdownWatch) {
        info!(addr = %self.listen_addr, "starting admin API");

        let state = layer7waf_admin::new_shared_state(self.config.clone());

        tokio::select! {
            result = layer7waf_admin::run_admin_server(state, &self.listen_addr) => {
                if let Err(e) = result {
                    error!(error = %e, "admin API server error");
                }
            }
            _ = shutdown.changed() => {
                info!("admin API shutting down");
            }
        }
    }
}
