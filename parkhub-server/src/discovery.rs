//! mDNS/DNS-SD Service Discovery
//!
//! Broadcasts server presence on the local network for autodiscovery.

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

use crate::config::ServerConfig;

/// mDNS service for broadcasting server presence
pub struct MdnsService {
    daemon: ServiceDaemon,
    service_fullname: String,
}

impl MdnsService {
    /// Create and register a new mDNS service
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;

        // Build service properties
        let mut properties = HashMap::new();
        properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        properties.insert(
            "protocol".to_string(),
            parkhub_common::PROTOCOL_VERSION.to_string(),
        );
        properties.insert("tls".to_string(), config.enable_tls.to_string());

        // Get hostname
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "parkhub-server".to_string());

        // Create service info
        let service_type = parkhub_common::MDNS_SERVICE_TYPE;
        let instance_name = format!("{} ({})", config.server_name, hostname);

        let service = ServiceInfo::new(
            service_type,
            &instance_name,
            &format!("{}.local.", hostname),
            "",
            config.port,
            properties,
        )?;

        // Register the service
        daemon.register(service.clone())?;

        Ok(Self {
            daemon,
            service_fullname: service.get_fullname().to_string(),
        })
    }

    /// Unregister the service
    pub fn unregister(&self) -> Result<()> {
        self.daemon.unregister(&self.service_fullname)?;
        Ok(())
    }
}

impl Drop for MdnsService {
    fn drop(&mut self) {
        let _ = self.unregister();
    }
}
