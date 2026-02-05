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
    pub fn new(event_type: AuditEventType) -> AuditEntryBuilder {
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

    pub fn ip(mut self, ip: IpAddr) -> Self {
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

    pub fn success(mut self, success: bool) -> Self {
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

/// Convenience functions for common audit events
pub mod events {
    use super::*;

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
}
