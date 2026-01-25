//! Server Discovery
//!
//! Discovers ParkHub servers on the local network using mDNS/DNS-SD.

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::AppState;

/// Discover servers on the local network
pub async fn discover_servers(state: Arc<RwLock<AppState>>) -> Result<()> {
    let daemon = ServiceDaemon::new()?;

    // Browse for ParkHub services
    let receiver = daemon.browse(parkhub_common::MDNS_SERVICE_TYPE)?;

    info!("Starting server discovery...");

    loop {
        match receiver.recv_async().await {
            Ok(event) => match event {
                ServiceEvent::ServiceResolved(info) => {
                    info!("Discovered server: {}", info.get_fullname());

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
                        .unwrap_or_else(|| info.get_hostname().to_string());

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
                _ => {}
            },
            Err(e) => {
                tracing::error!("mDNS discovery error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
