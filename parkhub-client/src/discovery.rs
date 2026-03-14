//! Server Discovery
//!
//! Discovers ParkHub servers on the local network using mDNS/DNS-SD
//! with fallback to localhost probing.

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::AppState;

/// Probe localhost for a running server
async fn probe_localhost(state: Arc<RwLock<AppState>>) -> bool {
    let ports = [7878u16, 8080, 3000];
    let mut found = false;

    for port in ports {
        // Try HTTP first (most common for development)
        let url = format!("http://localhost:{}/health", port);
        debug!("Probing {}", url);
        match reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                info!("Found local server at localhost:{}", port);

                let server_info = parkhub_common::ServerInfo {
                    name: format!("Local Server (localhost:{})", port),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    protocol_version: parkhub_common::PROTOCOL_VERSION.to_string(),
                    host: "localhost".to_string(),
                    port,
                    tls: false,
                    fingerprint: None,
                };

                let mut state = state.write().await;
                if !state
                    .discovered_servers
                    .iter()
                    .any(|s| s.host == "localhost" && s.port == port)
                {
                    state.discovered_servers.push(server_info);
                    found = true;
                }
                continue;
            }
            Ok(resp) => {
                debug!("localhost:{} returned status {}", port, resp.status());
            }
            Err(e) => {
                debug!("localhost:{} probe failed: {}", port, e);
            }
        }

        // Also try 127.0.0.1
        let url = format!("http://127.0.0.1:{}/health", port);
        debug!("Probing {}", url);
        match reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                info!("Found local server at 127.0.0.1:{}", port);

                let server_info = parkhub_common::ServerInfo {
                    name: format!("Local Server (127.0.0.1:{})", port),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    protocol_version: parkhub_common::PROTOCOL_VERSION.to_string(),
                    host: "127.0.0.1".to_string(),
                    port,
                    tls: false,
                    fingerprint: None,
                };

                let mut state = state.write().await;
                if !state
                    .discovered_servers
                    .iter()
                    .any(|s| s.host == "127.0.0.1" && s.port == port)
                {
                    state.discovered_servers.push(server_info);
                    found = true;
                }
                continue;
            }
            Ok(resp) => {
                debug!("127.0.0.1:{} returned status {}", port, resp.status());
            }
            Err(e) => {
                debug!("127.0.0.1:{} probe failed: {}", port, e);
            }
        }
    }

    found
}

/// Discover servers on the local network
/// Returns after initial discovery phase (doesn't block indefinitely)
pub async fn discover_servers(state: Arc<RwLock<AppState>>) -> Result<()> {
    info!("Starting server discovery...");

    // First, probe localhost for a local server (fast and reliable)
    // Run synchronously to ensure we find local servers immediately
    let found_local = probe_localhost(state.clone()).await;
    if found_local {
        info!("Found local server via localhost probe");
    }

    // Then try mDNS discovery with a timeout
    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            warn!("mDNS not available: {}. Using localhost probe only.", e);
            return Ok(());
        }
    };

    // Browse for ParkHub services
    let receiver = match daemon.browse(parkhub_common::MDNS_SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            warn!("mDNS browse failed: {}. Using localhost probe only.", e);
            return Ok(());
        }
    };

    info!("mDNS discovery started, scanning for 5 seconds...");

    // Do a bounded discovery scan (5 seconds) instead of infinite loop
    let discovery_timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < discovery_timeout {
        match timeout(Duration::from_millis(500), receiver.recv_async()).await {
            Ok(Ok(event)) => match event {
                ServiceEvent::ServiceResolved(info) => {
                    info!("Discovered server via mDNS: {}", info.get_fullname());

                    // Extract server info from mDNS properties
                    let properties = info.get_properties();
                    let version = properties
                        .get_property_val_str("version")
                        .unwrap_or("unknown");
                    let protocol = properties
                        .get_property_val_str("protocol")
                        .unwrap_or("unknown");
                    let tls = properties
                        .get_property_val_str("tls")
                        .map(|s| s == "true")
                        .unwrap_or(false);

                    // Get first address
                    let host = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| info.get_hostname().trim_end_matches('.').to_string());

                    let server_info = parkhub_common::ServerInfo {
                        name: info.get_fullname().to_string(),
                        version: version.to_string(),
                        protocol_version: protocol.to_string(),
                        host,
                        port: info.get_port(),
                        tls,
                        fingerprint: None,
                    };

                    // Add to discovered servers
                    let mut state = state.write().await;
                    if !state
                        .discovered_servers
                        .iter()
                        .any(|s| s.name == server_info.name)
                    {
                        state.discovered_servers.push(server_info);
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    debug!("Server removed: {}", fullname);

                    let mut state = state.write().await;
                    state.discovered_servers.retain(|s| s.name != fullname);
                }
                ServiceEvent::SearchStarted(_) => {
                    debug!("mDNS search started");
                }
                _ => {}
            },
            Ok(Err(e)) => {
                warn!("mDNS discovery error: {}", e);
            }
            Err(_) => {
                // Timeout - this is normal, continue checking elapsed time
            }
        }
    }

    // Stop the browse cleanly before daemon is dropped to avoid channel errors
    let _ = daemon.stop_browse(parkhub_common::MDNS_SERVICE_TYPE);
    // Give daemon a moment to process the stop
    tokio::time::sleep(Duration::from_millis(100)).await;

    info!("Discovery scan complete");
    Ok(())
}
