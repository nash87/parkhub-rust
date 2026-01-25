//! Configuration loading module

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app: AppInfo,
    pub server: ServerConfig,
    pub oauth: OAuthConfig,
    pub development: DevConfig,
    pub i18n: I18nConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub local_url: String,
    pub production_url: String,
    pub active: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DevConfig {
    pub enabled: bool,
    pub show_dev_panel: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct I18nConfig {
    pub default_locale: String,
    pub available: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DevUsersData {
    pub users: Vec<DevUserConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DevUserConfig {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub role: String,
    pub color: String,
}

/// Load application configuration from embedded config file
pub fn load_config() -> Result<AppConfig> {
    let config_str = include_str!("../config/config.toml");
    let config: AppConfig = toml::from_str(config_str).context("Failed to parse config.toml")?;
    Ok(config)
}

/// Load development users from embedded JSON file
pub fn load_dev_users() -> Result<Vec<DevUserConfig>> {
    let users_str = include_str!("../config/dev-users.json");
    let data: DevUsersData =
        serde_json::from_str(users_str).context("Failed to parse dev-users.json")?;
    Ok(data.users)
}
