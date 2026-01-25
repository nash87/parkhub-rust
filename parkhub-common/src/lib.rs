//! ParkHub Common Library
//!
//! Shared types, API models, and protocol definitions used by both
//! the server and client applications.

pub mod models;
pub mod protocol;
pub mod error;

pub use models::*;
pub use protocol::*;
pub use error::*;

/// Protocol version for client-server compatibility checks
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Default server port
pub const DEFAULT_PORT: u16 = 7878;

/// mDNS service type for autodiscovery
pub const MDNS_SERVICE_TYPE: &str = "_parkhub._tcp.local.";
