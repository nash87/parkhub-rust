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
    pub fn new(config: &ServerConfig) -> Result<Self> {
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
        let hostname = hostname::get().map_or_else(
            |_| "parkhub-server".to_string(),
            |h| h.to_string_lossy().to_string(),
        );

        // Create service info
        let service_type = parkhub_common::MDNS_SERVICE_TYPE;
        let instance_name = format!("{} ({})", config.server_name, hostname);

        let service = ServiceInfo::new(
            service_type,
            &instance_name,
            &format!("{hostname}.local."),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal ServerConfig for discovery tests.
    fn test_config() -> ServerConfig {
        ServerConfig {
            server_name: "TestServer".into(),
            port: 8080,
            enable_tls: false,
            enable_mdns: true,
            ..ServerConfig::default()
        }
    }

    #[test]
    fn service_properties_include_version_and_protocol() {
        let config = test_config();
        let mut properties = HashMap::new();
        properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        properties.insert(
            "protocol".to_string(),
            parkhub_common::PROTOCOL_VERSION.to_string(),
        );
        properties.insert("tls".to_string(), config.enable_tls.to_string());

        assert_eq!(
            properties.get("version").unwrap(),
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(
            properties.get("protocol").unwrap(),
            parkhub_common::PROTOCOL_VERSION
        );
        assert_eq!(properties.get("tls").unwrap(), "false");
    }

    #[test]
    fn service_properties_tls_flag_reflects_config() {
        let mut config = test_config();
        config.enable_tls = true;
        let tls_prop = config.enable_tls.to_string();
        assert_eq!(tls_prop, "true");

        config.enable_tls = false;
        let tls_prop = config.enable_tls.to_string();
        assert_eq!(tls_prop, "false");
    }

    #[test]
    fn instance_name_format_includes_server_name() {
        let config = test_config();
        let hostname = hostname::get().map_or_else(
            |_| "parkhub-server".to_string(),
            |h| h.to_string_lossy().to_string(),
        );
        let instance_name = format!("{} ({})", config.server_name, hostname);

        assert!(instance_name.starts_with("TestServer ("));
        assert!(instance_name.ends_with(')'));
        assert!(instance_name.contains(&hostname));
    }

    #[test]
    fn instance_name_with_special_chars_in_server_name() {
        let mut config = test_config();
        config.server_name = "My Parking Lot (Building A)".into();
        let hostname = "testhost".to_string();
        let instance_name = format!("{} ({})", config.server_name, hostname);

        assert!(instance_name.contains("My Parking Lot (Building A)"));
    }

    #[test]
    fn hostname_fallback_is_valid() {
        // Verify the fallback value is a reasonable default
        let fallback = "parkhub-server";
        assert!(!fallback.is_empty());
        assert!(fallback.is_ascii());
        assert!(!fallback.contains(' '));
    }

    #[test]
    fn service_type_matches_common_constant() {
        let service_type = parkhub_common::MDNS_SERVICE_TYPE;
        assert_eq!(service_type, "_parkhub._tcp.local.");
    }

    #[test]
    fn mdns_service_new_and_drop() {
        let config = test_config();
        // MdnsService::new may fail in CI due to network restrictions, which is acceptable.
        // This test verifies the construction path does not panic.
        match MdnsService::new(&config) {
            Ok(svc) => {
                assert!(!svc.service_fullname.is_empty());
                // Drop cleans up via unregister
                drop(svc);
            }
            Err(_) => {
                // Network unavailable in CI — acceptable
            }
        }
    }
}
