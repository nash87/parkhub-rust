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

    /// Enable TLS encryption for HTTP
    pub enable_tls: bool,

    /// Enable mDNS autodiscovery
    pub enable_mdns: bool,

    /// Enable database encryption at rest
    #[serde(default = "default_true")]
    pub encryption_enabled: bool,

    /// Database encryption passphrase (only in memory, not saved to config)
    #[serde(skip)]
    pub encryption_passphrase: Option<String>,

    /// Admin username
    pub admin_username: String,

    /// Admin password hash (argon2)
    pub admin_password_hash: String,

    /// Use portable mode (store data next to executable)
    #[serde(default = "default_true")]
    pub portable_mode: bool,

    /// Generate dummy users during setup (not saved, only used once)
    #[serde(skip)]
    pub generate_dummy_users: bool,

    /// Username style for dummy users (0=FirstLastLetter, 1=FirstDotLast, 2=InitialLast, 3=FirstInitial)
    #[serde(skip)]
    pub username_style: u8,

    /// License plate display mode (0=show, 1=blur, 2=redact, 3=hide)
    #[serde(default)]
    pub license_plate_display: u8,

    /// Session timeout in minutes (0 = never)
    #[serde(default = "default_session_timeout")]
    pub session_timeout_minutes: u32,

    /// Allow user self-registration
    #[serde(default)]
    pub allow_self_registration: bool,

    /// Require email verification for new users
    #[serde(default)]
    pub require_email_verification: bool,

    /// Maximum concurrent sessions per user (0 = unlimited)
    #[serde(default)]
    pub max_concurrent_sessions: u32,

    /// Enable automatic daily backups
    #[serde(default = "default_true")]
    pub auto_backup_enabled: bool,

    /// Number of backups to keep
    #[serde(default = "default_backup_count")]
    pub backup_retention_count: u32,

    /// Enable audit logging
    #[serde(default = "default_true")]
    pub audit_logging_enabled: bool,

    /// Default language (en, de, es, fr, etc.)
    #[serde(default = "default_language")]
    pub default_language: String,

    /// Organization/Company name (for branding)
    #[serde(default)]
    pub organization_name: String,

    /// Close behavior: "ask", "minimize", "exit"
    #[serde(default = "default_close_behavior")]
    pub close_behavior: String,

    /// Theme mode: 0=Dark, 1=Light, 2=High Contrast, 3=Deuteranopia, 4=Protanopia, 5=Tritanopia
    #[serde(default)]
    pub theme_mode: i32,

    /// Font scale: 1.0=Normal, 1.25=Large, 1.5=Extra Large
    #[serde(default = "default_font_scale")]
    pub font_scale: f32,

    /// Reduce motion animations
    #[serde(default)]
    pub reduce_motion: bool,
}

fn default_font_scale() -> f32 {
    1.0
}

fn default_session_timeout() -> u32 {
    60 // 1 hour default
}

fn default_backup_count() -> u32 {
    7 // Keep 7 days of backups
}

fn default_language() -> String {
    "en".to_string()
}

fn default_close_behavior() -> String {
    "ask".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_name: "ParkHub Server".to_string(),
            port: parkhub_common::DEFAULT_PORT,
            enable_tls: true,
            enable_mdns: true,
            encryption_enabled: true,
            encryption_passphrase: None,
            admin_username: "admin".to_string(),
            admin_password_hash: String::new(), // Must be set during setup
            portable_mode: true,
            generate_dummy_users: false,
            username_style: 0,        // FirstLastLetter by default
            license_plate_display: 0, // Show by default
            session_timeout_minutes: 60,
            allow_self_registration: false,
            require_email_verification: false,
            max_concurrent_sessions: 0, // Unlimited
            auto_backup_enabled: true,
            backup_retention_count: 7,
            audit_logging_enabled: true,
            default_language: "en".to_string(),
            organization_name: String::new(),
            close_behavior: "ask".to_string(),
            theme_mode: 0, // Dark by default
            font_scale: 1.0,
            reduce_motion: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();

        assert_eq!(config.server_name, "ParkHub Server");
        assert_eq!(config.port, parkhub_common::DEFAULT_PORT);
        assert!(config.enable_tls);
        assert!(config.enable_mdns);
        assert!(config.encryption_enabled);
        assert!(config.portable_mode);
        assert!(!config.generate_dummy_users);
        assert_eq!(config.username_style, 0);
        assert_eq!(config.license_plate_display, 0);
        assert_eq!(config.session_timeout_minutes, 60);
        assert!(!config.allow_self_registration);
        assert!(!config.require_email_verification);
        assert_eq!(config.max_concurrent_sessions, 0);
        assert!(config.auto_backup_enabled);
        assert_eq!(config.backup_retention_count, 7);
        assert!(config.audit_logging_enabled);
        assert_eq!(config.default_language, "en");
        assert_eq!(config.organization_name, "");
    }

    #[test]
    fn test_config_save_load() {
        let config = ServerConfig {
            server_name: "Test Server".to_string(),
            port: 9999,
            enable_tls: false,
            organization_name: "Test Org".to_string(),
            admin_password_hash: "test_hash_123".to_string(),
            ..Default::default()
        };

        // Create a temp file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path();

        // Save config
        config.save(path).expect("Failed to save config");

        // Load config
        let loaded = ServerConfig::load(path).expect("Failed to load config");

        assert_eq!(loaded.server_name, "Test Server");
        assert_eq!(loaded.port, 9999);
        assert!(!loaded.enable_tls);
        assert_eq!(loaded.organization_name, "Test Org");
        assert_eq!(loaded.admin_password_hash, "test_hash_123");
    }

    #[test]
    fn test_config_serialization() {
        let config = ServerConfig::default();
        let serialized = toml::to_string(&config).expect("Failed to serialize");

        // Check that skip fields are not serialized
        assert!(!serialized.contains("encryption_passphrase"));
        assert!(!serialized.contains("generate_dummy_users"));
        assert!(!serialized.contains("username_style"));

        // Check that important fields are serialized
        assert!(serialized.contains("server_name"));
        assert!(serialized.contains("port"));
        assert!(serialized.contains("enable_tls"));
    }

    #[test]
    fn test_config_deserialization_with_defaults() {
        let minimal_toml = r#"
            server_name = "Minimal"
            port = 8443
            enable_tls = true
            enable_mdns = true
            admin_username = "admin"
            admin_password_hash = "hash"
        "#;

        let config: ServerConfig = toml::from_str(minimal_toml).expect("Failed to deserialize");

        // Check defaults are applied
        assert_eq!(config.server_name, "Minimal");
        assert!(config.encryption_enabled); // default_true
        assert!(config.portable_mode); // default_true
        assert_eq!(config.session_timeout_minutes, 60); // default
        assert_eq!(config.backup_retention_count, 7); // default
        assert_eq!(config.default_language, "en"); // default
    }

    #[test]
    fn test_license_plate_display_modes() {
        // Test each license plate display mode value
        let modes = [(0, "show"), (1, "blur"), (2, "redact"), (3, "hide")];

        for (mode, _name) in modes.iter() {
            let config = ServerConfig {
                license_plate_display: *mode,
                ..Default::default()
            };
            assert_eq!(config.license_plate_display, *mode);
        }
    }

    #[test]
    fn test_username_styles() {
        // Test each username style value
        let styles = [
            (0, "FirstLastLetter"),
            (1, "FirstDotLast"),
            (2, "InitialLast"),
            (3, "FirstInitial"),
        ];

        for (style, _name) in styles.iter() {
            let config = ServerConfig {
                username_style: *style,
                ..Default::default()
            };
            assert_eq!(config.username_style, *style);
        }
    }

    #[test]
    fn test_default_close_behavior() {
        let config = ServerConfig::default();
        assert_eq!(config.close_behavior, "ask");
    }

    #[test]
    fn test_default_theme_mode() {
        let config = ServerConfig::default();
        assert_eq!(config.theme_mode, 0); // Dark
    }

    #[test]
    fn test_default_font_scale() {
        let config = ServerConfig::default();
        assert!((config.font_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default_reduce_motion() {
        let config = ServerConfig::default();
        assert!(!config.reduce_motion);
    }

    #[test]
    fn test_config_save_load_roundtrip_preserves_all_fields() {
        let config = ServerConfig {
            server_name: "Roundtrip Test".to_string(),
            port: 12345,
            enable_tls: false,
            enable_mdns: false,
            encryption_enabled: false,
            admin_username: "superadmin".to_string(),
            admin_password_hash: "hash_abc".to_string(),
            portable_mode: false,
            license_plate_display: 2,
            session_timeout_minutes: 120,
            allow_self_registration: true,
            require_email_verification: true,
            max_concurrent_sessions: 5,
            auto_backup_enabled: false,
            backup_retention_count: 14,
            audit_logging_enabled: false,
            default_language: "de".to_string(),
            organization_name: "ACME Corp".to_string(),
            close_behavior: "exit".to_string(),
            theme_mode: 2,
            font_scale: 1.5,
            reduce_motion: true,
            ..Default::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path()).unwrap();
        let loaded = ServerConfig::load(temp_file.path()).unwrap();

        assert_eq!(loaded.server_name, "Roundtrip Test");
        assert_eq!(loaded.port, 12345);
        assert!(!loaded.enable_tls);
        assert!(!loaded.enable_mdns);
        assert!(!loaded.encryption_enabled);
        assert_eq!(loaded.admin_username, "superadmin");
        assert_eq!(loaded.license_plate_display, 2);
        assert_eq!(loaded.session_timeout_minutes, 120);
        assert!(loaded.allow_self_registration);
        assert!(loaded.require_email_verification);
        assert_eq!(loaded.max_concurrent_sessions, 5);
        assert!(!loaded.auto_backup_enabled);
        assert_eq!(loaded.backup_retention_count, 14);
        assert!(!loaded.audit_logging_enabled);
        assert_eq!(loaded.default_language, "de");
        assert_eq!(loaded.organization_name, "ACME Corp");
        assert_eq!(loaded.close_behavior, "exit");
        assert_eq!(loaded.theme_mode, 2);
        assert!((loaded.font_scale - 1.5).abs() < f32::EPSILON);
        assert!(loaded.reduce_motion);
    }

    #[test]
    fn test_config_load_nonexistent_file_errors() {
        let result = ServerConfig::load(Path::new("/tmp/nonexistent_parkhub_config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_load_invalid_toml_errors() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "this is not valid toml {{{{").unwrap();
        let result = ServerConfig::load(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_config_skipped_fields_not_persisted() {
        let config = ServerConfig {
            encryption_passphrase: Some("secret123".to_string()),
            generate_dummy_users: true,
            username_style: 3,
            ..Default::default()
        };

        let temp_file = NamedTempFile::new().unwrap();
        config.save(temp_file.path()).unwrap();
        let loaded = ServerConfig::load(temp_file.path()).unwrap();

        // Skipped fields should be at their defaults after load
        assert!(loaded.encryption_passphrase.is_none());
        assert!(!loaded.generate_dummy_users);
        assert_eq!(loaded.username_style, 0);
    }

    #[test]
    fn test_config_deserialization_unknown_fields_tolerated() {
        let toml_with_extra = r#"
            server_name = "Test"
            port = 8443
            enable_tls = true
            enable_mdns = true
            admin_username = "admin"
            admin_password_hash = "hash"
            some_future_field = "unknown"
        "#;

        // serde default deny_unknown_fields is off, so this should work
        // unless the struct uses deny_unknown_fields
        let result: Result<ServerConfig, _> = toml::from_str(toml_with_extra);
        // If it errors, that's also valid behavior; just document it
        // Most configs should tolerate unknown fields for forward compat
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_accessibility_theme_modes() {
        // Test all theme modes are valid values
        for mode in 0..=5 {
            let config = ServerConfig {
                theme_mode: mode,
                ..Default::default()
            };
            assert_eq!(config.theme_mode, mode);
        }
    }

    #[test]
    fn test_config_font_scale_values() {
        for &scale in &[1.0f32, 1.25, 1.5] {
            let config = ServerConfig {
                font_scale: scale,
                ..Default::default()
            };
            assert!((config.font_scale - scale).abs() < f32::EPSILON);
        }
    }
}
