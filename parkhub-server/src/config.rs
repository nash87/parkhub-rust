//! Server Configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Display name for this server
    pub server_name: String,

    /// Port to listen on
    pub port: u16,

    /// Enable TLS encryption
    pub enable_tls: bool,

    /// Enable mDNS autodiscovery
    pub enable_mdns: bool,

    /// Admin username
    pub admin_username: String,

    /// Admin password hash (argon2)
    pub admin_password_hash: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_name: "ParkHub Server".to_string(),
            port: parkhub_common::DEFAULT_PORT,
            enable_tls: true,
            enable_mdns: true,
            admin_username: "admin".to_string(),
            admin_password_hash: String::new(), // Must be set during setup
        }
    }
}

impl ServerConfig {
    /// Load configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
