//! Audit Logging
//!
//! Records security-relevant events for compliance and debugging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::{info, warn};
use uuid::Uuid;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication
    LoginSuccess,
    LoginFailed,
    Logout,
    TokenRefresh,
    PasswordChanged,
    PasswordResetRequested,

    // User management
    UserCreated,
    UserUpdated,
    UserDeleted,
    UserDeactivated,
    UserActivated,
    RoleChanged,

    // Bookings
    BookingCreated,
    BookingUpdated,
    BookingCancelled,
    BookingExtended,
    CheckIn,
    CheckOut,

    // Vehicles
    VehicleAdded,
    VehicleRemoved,

    // Admin actions
    LotCreated,
    LotUpdated,
    LotDeleted,
    SlotStatusChanged,
    ConfigChanged,

    // Settings
    SettingsChanged,

    // Security
    RateLimitExceeded,
    InvalidTokenUsed,
    UnauthorizedAccess,
    SuspiciousActivity,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique event ID
    pub id: Uuid,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// User ID (if authenticated)
    pub user_id: Option<Uuid>,
    /// Username (if known)
    pub username: Option<String>,
    /// Client IP address
    pub ip_address: Option<IpAddr>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Resource type (e.g., "booking", "user")
    pub resource_type: Option<String>,
    /// Resource ID
    pub resource_id: Option<String>,
    /// Additional details (JSON)
    pub details: Option<serde_json::Value>,
    /// Was the action successful?
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry builder
    #[allow(clippy::new_ret_no_self)]
    pub const fn new(event_type: AuditEventType) -> AuditEntryBuilder {
        AuditEntryBuilder {
            event_type,
            user_id: None,
            username: None,
            ip_address: None,
            user_agent: None,
            resource_type: None,
            resource_id: None,
            details: None,
            success: true,
            error: None,
        }
    }
}

/// Builder for audit entries
pub struct AuditEntryBuilder {
    event_type: AuditEventType,
    user_id: Option<Uuid>,
    username: Option<String>,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
    resource_type: Option<String>,
    resource_id: Option<String>,
    details: Option<serde_json::Value>,
    success: bool,
    error: Option<String>,
}

impl AuditEntryBuilder {
    pub fn user(mut self, user_id: Uuid, username: &str) -> Self {
        self.user_id = Some(user_id);
        self.username = Some(username.to_string());
        self
    }

    pub const fn ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }

    pub fn resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Convenience: set a simple string detail.
    pub fn detail(self, message: &str) -> Self {
        self.details(serde_json::json!({"message": message}))
    }

    pub const fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn error(mut self, error: &str) -> Self {
        self.success = false;
        self.error = Some(error.to_string());
        self
    }

    /// Build and log the audit entry
    pub fn log(self) -> AuditEntry {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: self.event_type,
            user_id: self.user_id,
            username: self.username,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            resource_type: self.resource_type,
            resource_id: self.resource_id,
            details: self.details,
            success: self.success,
            error: self.error,
        };

        // Log to structured logging
        if entry.success {
            info!(
                event_type = ?entry.event_type,
                user_id = ?entry.user_id,
                username = ?entry.username,
                ip = ?entry.ip_address,
                resource_type = ?entry.resource_type,
                resource_id = ?entry.resource_id,
                "Audit: {:?}",
                entry.event_type
            );
        } else {
            warn!(
                event_type = ?entry.event_type,
                user_id = ?entry.user_id,
                username = ?entry.username,
                ip = ?entry.ip_address,
                error = ?entry.error,
                "Audit FAILED: {:?}",
                entry.event_type
            );
        }

        entry
    }
}

impl AuditEntry {
    /// Persist this audit entry to the database (non-blocking best-effort).
    /// Call after `.log()` when you have DB access.
    pub async fn persist(&self, db: &crate::db::Database) {
        let log_entry = crate::db::AuditLogEntry {
            id: self.id,
            timestamp: self.timestamp,
            event_type: format!("{:?}", self.event_type),
            user_id: self.user_id,
            username: self.username.clone(),
            details: self.details.as_ref().map(std::string::ToString::to_string),
        };
        if let Err(e) = db.save_audit_log(&log_entry).await {
            tracing::warn!("Failed to persist audit entry: {e}");
        }
    }
}

/// Convenience functions for common audit events
pub mod events {
    use super::{AuditEntry, AuditEventType, IpAddr, Uuid};

    pub fn login_success(user_id: Uuid, username: &str, ip: IpAddr) -> AuditEntry {
        AuditEntry::new(AuditEventType::LoginSuccess)
            .user(user_id, username)
            .ip(ip)
            .log()
    }

    pub fn login_failed(_username: &str, ip: IpAddr, reason: &str) -> AuditEntry {
        AuditEntry::new(AuditEventType::LoginFailed)
            .ip(ip)
            .error(reason)
            .log()
    }

    pub fn booking_created(user_id: Uuid, username: &str, booking_id: Uuid) -> AuditEntry {
        AuditEntry::new(AuditEventType::BookingCreated)
            .user(user_id, username)
            .resource("booking", &booking_id.to_string())
            .log()
    }

    pub fn unauthorized_access(ip: IpAddr, path: &str) -> AuditEntry {
        AuditEntry::new(AuditEventType::UnauthorizedAccess)
            .ip(ip)
            .details(serde_json::json!({ "path": path }))
            .error("Unauthorized access attempt")
            .log()
    }

    pub fn rate_limit_exceeded(ip: IpAddr, endpoint: &str) -> AuditEntry {
        AuditEntry::new(AuditEventType::RateLimitExceeded)
            .ip(ip)
            .details(serde_json::json!({ "endpoint": endpoint }))
            .error("Rate limit exceeded")
            .log()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditEntry::new(AuditEventType::LoginSuccess)
            .user(Uuid::new_v4(), "testuser")
            .ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .log();

        assert!(entry.success);
        assert!(entry.username.is_some());
        assert!(entry.ip_address.is_some());
    }

    #[test]
    fn test_failed_audit_entry() {
        let entry = AuditEntry::new(AuditEventType::LoginFailed)
            .ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)))
            .error("Invalid password")
            .log();

        assert!(!entry.success);
        assert!(entry.error.is_some());
    }

    #[test]
    fn test_audit_entry_has_unique_id() {
        let e1 = AuditEntry::new(AuditEventType::LoginSuccess).log();
        let e2 = AuditEntry::new(AuditEventType::LoginSuccess).log();
        assert_ne!(e1.id, e2.id);
    }

    #[test]
    fn test_audit_entry_timestamp_set() {
        let before = Utc::now();
        let entry = AuditEntry::new(AuditEventType::UserCreated).log();
        let after = Utc::now();
        assert!(entry.timestamp >= before);
        assert!(entry.timestamp <= after);
    }

    #[test]
    fn test_audit_entry_with_resource() {
        let booking_id = Uuid::new_v4();
        let entry = AuditEntry::new(AuditEventType::BookingCreated)
            .resource("booking", &booking_id.to_string())
            .log();

        assert_eq!(entry.resource_type.as_deref(), Some("booking"));
        assert_eq!(
            entry.resource_id.as_deref(),
            Some(booking_id.to_string().as_str())
        );
    }

    #[test]
    fn test_audit_entry_with_details() {
        let details = serde_json::json!({"key": "value", "count": 42});
        let entry = AuditEntry::new(AuditEventType::ConfigChanged)
            .details(details.clone())
            .log();

        assert_eq!(entry.details, Some(details));
    }

    #[test]
    fn test_audit_entry_with_user_agent() {
        let entry = AuditEntry::new(AuditEventType::LoginSuccess)
            .user_agent("Mozilla/5.0")
            .log();

        assert_eq!(entry.user_agent.as_deref(), Some("Mozilla/5.0"));
    }

    #[test]
    fn test_audit_entry_default_success_true() {
        let entry = AuditEntry::new(AuditEventType::UserCreated).log();
        assert!(entry.success);
        assert!(entry.error.is_none());
    }

    #[test]
    fn test_audit_entry_explicit_success_false() {
        let entry = AuditEntry::new(AuditEventType::BookingCancelled)
            .success(false)
            .log();
        assert!(!entry.success);
    }

    #[test]
    fn test_audit_entry_error_sets_success_false() {
        let entry = AuditEntry::new(AuditEventType::LoginFailed)
            .error("bad credentials")
            .log();
        assert!(!entry.success);
        assert_eq!(entry.error.as_deref(), Some("bad credentials"));
    }

    #[test]
    fn test_audit_entry_without_user() {
        let entry = AuditEntry::new(AuditEventType::RateLimitExceeded)
            .ip(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
            .log();

        assert!(entry.user_id.is_none());
        assert!(entry.username.is_none());
        assert!(entry.ip_address.is_some());
    }

    #[test]
    fn test_audit_entry_all_event_types() {
        // Verify all event types can be used without panic
        let event_types = vec![
            AuditEventType::LoginSuccess,
            AuditEventType::LoginFailed,
            AuditEventType::Logout,
            AuditEventType::TokenRefresh,
            AuditEventType::PasswordChanged,
            AuditEventType::PasswordResetRequested,
            AuditEventType::UserCreated,
            AuditEventType::UserUpdated,
            AuditEventType::UserDeleted,
            AuditEventType::UserDeactivated,
            AuditEventType::UserActivated,
            AuditEventType::RoleChanged,
            AuditEventType::BookingCreated,
            AuditEventType::BookingUpdated,
            AuditEventType::BookingCancelled,
            AuditEventType::BookingExtended,
            AuditEventType::CheckIn,
            AuditEventType::CheckOut,
            AuditEventType::VehicleAdded,
            AuditEventType::VehicleRemoved,
            AuditEventType::LotCreated,
            AuditEventType::LotUpdated,
            AuditEventType::LotDeleted,
            AuditEventType::SlotStatusChanged,
            AuditEventType::ConfigChanged,
            AuditEventType::RateLimitExceeded,
            AuditEventType::InvalidTokenUsed,
            AuditEventType::UnauthorizedAccess,
            AuditEventType::SuspiciousActivity,
        ];

        for event_type in event_types {
            let entry = AuditEntry::new(event_type).log();
            assert!(entry.success);
        }
    }

    #[test]
    fn test_audit_event_type_serialization() {
        let serialized = serde_json::to_string(&AuditEventType::LoginSuccess).unwrap();
        assert_eq!(serialized, "\"login_success\"");

        let serialized = serde_json::to_string(&AuditEventType::BookingCreated).unwrap();
        assert_eq!(serialized, "\"booking_created\"");

        let serialized = serde_json::to_string(&AuditEventType::RateLimitExceeded).unwrap();
        assert_eq!(serialized, "\"rate_limit_exceeded\"");
    }

    #[test]
    fn test_audit_event_type_deserialization() {
        let event: AuditEventType = serde_json::from_str("\"login_failed\"").unwrap();
        assert!(matches!(event, AuditEventType::LoginFailed));

        let event: AuditEventType = serde_json::from_str("\"user_created\"").unwrap();
        assert!(matches!(event, AuditEventType::UserCreated));
    }

    #[test]
    fn test_audit_entry_serialization_roundtrip() {
        let user_id = Uuid::new_v4();
        let entry = AuditEntry::new(AuditEventType::BookingCreated)
            .user(user_id, "florian")
            .ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)))
            .resource("booking", "abc-123")
            .details(serde_json::json!({"slot": 5}))
            .log();

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: AuditEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, entry.id);
        assert_eq!(deserialized.user_id, Some(user_id));
        assert_eq!(deserialized.username.as_deref(), Some("florian"));
        assert_eq!(deserialized.resource_type.as_deref(), Some("booking"));
        assert!(deserialized.success);
    }

    #[test]
    fn test_convenience_login_success() {
        let user_id = Uuid::new_v4();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let entry = events::login_success(user_id, "admin", ip);

        assert!(entry.success);
        assert_eq!(entry.user_id, Some(user_id));
        assert_eq!(entry.username.as_deref(), Some("admin"));
        assert_eq!(entry.ip_address, Some(ip));
    }

    #[test]
    fn test_convenience_login_failed() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let entry = events::login_failed("hacker", ip, "wrong password");

        assert!(!entry.success);
        assert_eq!(entry.error.as_deref(), Some("wrong password"));
        assert_eq!(entry.ip_address, Some(ip));
    }

    #[test]
    fn test_convenience_booking_created() {
        let user_id = Uuid::new_v4();
        let booking_id = Uuid::new_v4();
        let entry = events::booking_created(user_id, "user1", booking_id);

        assert!(entry.success);
        assert_eq!(entry.resource_type.as_deref(), Some("booking"));
        assert_eq!(
            entry.resource_id.as_deref(),
            Some(booking_id.to_string().as_str())
        );
    }

    #[test]
    fn test_convenience_unauthorized_access() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let entry = events::unauthorized_access(ip, "/admin/secret");

        assert!(!entry.success);
        assert!(entry.details.is_some());
        let details = entry.details.unwrap();
        assert_eq!(details["path"], "/admin/secret");
    }

    #[test]
    fn test_convenience_rate_limit_exceeded() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 10, 10, 10));
        let entry = events::rate_limit_exceeded(ip, "/api/login");

        assert!(!entry.success);
        assert!(entry.details.is_some());
        let details = entry.details.unwrap();
        assert_eq!(details["endpoint"], "/api/login");
    }
}
