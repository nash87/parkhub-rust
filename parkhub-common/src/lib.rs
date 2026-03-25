//! `ParkHub` Common Library
//!
//! Shared types, API models, and protocol definitions used by both
//! the server and client applications.

pub mod error;
pub mod models;
pub mod protocol;

pub use error::*;
pub use models::*;
pub use protocol::*;

/// Protocol version for client-server compatibility checks
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Default server port
pub const DEFAULT_PORT: u16 = 7878;

/// mDNS service type for autodiscovery
pub const MDNS_SERVICE_TYPE: &str = "_parkhub._tcp.local.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_semver() {
        let parts: Vec<&str> = PROTOCOL_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "PROTOCOL_VERSION must be semver (x.y.z)");
        for part in &parts {
            part.parse::<u32>()
                .expect("each semver component must be a non-negative integer");
        }
    }

    #[test]
    fn default_port_is_non_privileged() {
        let port = std::hint::black_box(DEFAULT_PORT as u32);
        assert!(
            port >= 1024,
            "DEFAULT_PORT should be a non-privileged port (>= 1024)"
        );
    }

    #[test]
    fn default_port_is_within_valid_range() {
        let port = std::hint::black_box(DEFAULT_PORT as u32);
        assert!(u16::try_from(port).is_ok());
    }

    #[test]
    fn mdns_service_type_follows_dns_sd_convention() {
        assert!(
            MDNS_SERVICE_TYPE.starts_with('_'),
            "DNS-SD service type must start with underscore"
        );
        assert!(
            MDNS_SERVICE_TYPE.ends_with(".local."),
            "mDNS service must end with .local."
        );
        assert!(
            MDNS_SERVICE_TYPE.contains("._tcp.") || MDNS_SERVICE_TYPE.contains("._udp."),
            "DNS-SD service type must specify _tcp or _udp transport"
        );
    }

    #[test]
    fn mdns_service_type_contains_parkhub() {
        assert!(
            MDNS_SERVICE_TYPE.contains("parkhub"),
            "service type must identify the application"
        );
    }

    #[test]
    fn public_modules_are_accessible() {
        // Verify re-exports work — compile-time check that key types are reachable.
        let _: fn() -> ApiResponse<()> = || ApiResponse::success(());
        let _: fn() -> ParkHubError = || ParkHubError::NotFound("test".into());
    }

    #[test]
    fn protocol_version_value() {
        assert_eq!(PROTOCOL_VERSION, "1.0.0");
    }

    #[test]
    fn default_port_value() {
        assert_eq!(DEFAULT_PORT, 7878);
    }
}
