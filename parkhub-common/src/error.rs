//! Error Types
//!
//! Common error types used across `ParkHub`.

use thiserror::Error;

/// Common errors for the `ParkHub` system
#[derive(Error, Debug)]
pub enum ParkHubError {
    // Authentication errors
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("User not found: {0}")]
    UserNotFound(String),

    // Booking errors
    #[error("Slot not available")]
    SlotNotAvailable,

    #[error("Booking not found: {0}")]
    BookingNotFound(String),

    #[error("Booking conflict: slot already booked for this time")]
    BookingConflict,

    #[error("Invalid booking time: {0}")]
    InvalidBookingTime(String),

    // Database errors
    #[error("Database error: {0}")]
    Database(String),

    // Network errors
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Server not found")]
    ServerNotFound,

    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolMismatch { expected: String, actual: String },

    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Server not configured")]
    ServerNotConfigured,

    // General errors
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Error codes for API responses
pub mod error_codes {
    pub const INVALID_CREDENTIALS: &str = "INVALID_CREDENTIALS";
    pub const TOKEN_EXPIRED: &str = "TOKEN_EXPIRED";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const USER_NOT_FOUND: &str = "USER_NOT_FOUND";
    pub const SLOT_NOT_AVAILABLE: &str = "SLOT_NOT_AVAILABLE";
    pub const BOOKING_NOT_FOUND: &str = "BOOKING_NOT_FOUND";
    pub const BOOKING_CONFLICT: &str = "BOOKING_CONFLICT";
    pub const INVALID_BOOKING_TIME: &str = "INVALID_BOOKING_TIME";
    pub const DATABASE_ERROR: &str = "DATABASE_ERROR";
    pub const CONNECTION_ERROR: &str = "CONNECTION_ERROR";
    pub const SERVER_NOT_FOUND: &str = "SERVER_NOT_FOUND";
    pub const PROTOCOL_MISMATCH: &str = "PROTOCOL_MISMATCH";
    pub const CONFIG_ERROR: &str = "CONFIG_ERROR";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HEAD tests ──────────────────────────────────────────────────────────

    #[test]
    fn display_invalid_credentials() {
        let err = ParkHubError::InvalidCredentials;
        assert_eq!(err.to_string(), "Invalid credentials");
    }

    #[test]
    fn display_token_expired() {
        assert_eq!(ParkHubError::TokenExpired.to_string(), "Token expired");
    }

    #[test]
    fn display_unauthorized() {
        assert_eq!(
            ParkHubError::Unauthorized.to_string(),
            "Unauthorized access"
        );
    }

    #[test]
    fn display_user_not_found_includes_id() {
        let err = ParkHubError::UserNotFound("user-42".into());
        assert_eq!(err.to_string(), "User not found: user-42");
    }

    #[test]
    fn display_slot_not_available() {
        assert_eq!(
            ParkHubError::SlotNotAvailable.to_string(),
            "Slot not available"
        );
    }

    #[test]
    fn display_booking_not_found_includes_id() {
        let err = ParkHubError::BookingNotFound("bk-99".into());
        assert_eq!(err.to_string(), "Booking not found: bk-99");
    }

    #[test]
    fn display_booking_conflict() {
        assert_eq!(
            ParkHubError::BookingConflict.to_string(),
            "Booking conflict: slot already booked for this time"
        );
    }

    #[test]
    fn display_invalid_booking_time_includes_reason() {
        let err = ParkHubError::InvalidBookingTime("end before start".into());
        assert_eq!(err.to_string(), "Invalid booking time: end before start");
    }

    #[test]
    fn display_database_error() {
        let err = ParkHubError::Database("disk full".into());
        assert_eq!(err.to_string(), "Database error: disk full");
    }

    #[test]
    fn display_connection_error() {
        let err = ParkHubError::Connection("timeout".into());
        assert_eq!(err.to_string(), "Connection error: timeout");
    }

    #[test]
    fn display_server_not_found() {
        assert_eq!(ParkHubError::ServerNotFound.to_string(), "Server not found");
    }

    #[test]
    fn display_protocol_mismatch() {
        let err = ParkHubError::ProtocolMismatch {
            expected: "2.0".into(),
            actual: "1.0".into(),
        };
        assert_eq!(
            err.to_string(),
            "Protocol version mismatch: expected 2.0, got 1.0"
        );
    }

    #[test]
    fn display_config_error() {
        let err = ParkHubError::Config("missing port".into());
        assert_eq!(err.to_string(), "Configuration error: missing port");
    }

    #[test]
    fn display_server_not_configured() {
        assert_eq!(
            ParkHubError::ServerNotConfigured.to_string(),
            "Server not configured"
        );
    }

    #[test]
    fn display_not_found() {
        let err = ParkHubError::NotFound("resource".into());
        assert_eq!(err.to_string(), "Not found: resource");
    }

    #[test]
    fn display_invalid_input() {
        let err = ParkHubError::InvalidInput("bad data".into());
        assert_eq!(err.to_string(), "Invalid input: bad data");
    }

    #[test]
    fn display_internal_error() {
        let err = ParkHubError::Internal("panic".into());
        assert_eq!(err.to_string(), "Internal error: panic");
    }

    #[test]
    fn error_codes_are_uppercase_snake_case() {
        let codes = [
            error_codes::INVALID_CREDENTIALS,
            error_codes::TOKEN_EXPIRED,
            error_codes::UNAUTHORIZED,
            error_codes::USER_NOT_FOUND,
            error_codes::SLOT_NOT_AVAILABLE,
            error_codes::BOOKING_NOT_FOUND,
            error_codes::BOOKING_CONFLICT,
            error_codes::INVALID_BOOKING_TIME,
            error_codes::DATABASE_ERROR,
            error_codes::CONNECTION_ERROR,
            error_codes::SERVER_NOT_FOUND,
            error_codes::PROTOCOL_MISMATCH,
            error_codes::CONFIG_ERROR,
            error_codes::NOT_FOUND,
            error_codes::INVALID_INPUT,
            error_codes::INTERNAL_ERROR,
        ];
        for code in codes {
            assert!(!code.is_empty(), "Error code must not be empty");
            assert!(
                code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "Error code {code} must be UPPER_SNAKE_CASE"
            );
        }
    }

    #[test]
    fn error_codes_are_unique() {
        let codes = [
            error_codes::INVALID_CREDENTIALS,
            error_codes::TOKEN_EXPIRED,
            error_codes::UNAUTHORIZED,
            error_codes::USER_NOT_FOUND,
            error_codes::SLOT_NOT_AVAILABLE,
            error_codes::BOOKING_NOT_FOUND,
            error_codes::BOOKING_CONFLICT,
            error_codes::INVALID_BOOKING_TIME,
            error_codes::DATABASE_ERROR,
            error_codes::CONNECTION_ERROR,
            error_codes::SERVER_NOT_FOUND,
            error_codes::PROTOCOL_MISMATCH,
            error_codes::CONFIG_ERROR,
            error_codes::NOT_FOUND,
            error_codes::INVALID_INPUT,
            error_codes::INTERNAL_ERROR,
        ];
        let mut seen = std::collections::HashSet::new();
        for code in codes {
            assert!(seen.insert(code), "Duplicate error code: {code}");
        }
    }

    #[test]
    fn parkhub_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(ParkHubError::Internal("test".into()));
        assert!(!err.to_string().is_empty());
    }

    // ── Copilot branch tests ────────────────────────────────────────────────

    #[test]
    fn test_display_invalid_credentials() {
        assert_eq!(
            ParkHubError::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
    }

    #[test]
    fn test_display_token_expired() {
        assert_eq!(ParkHubError::TokenExpired.to_string(), "Token expired");
    }

    #[test]
    fn test_display_unauthorized() {
        assert_eq!(
            ParkHubError::Unauthorized.to_string(),
            "Unauthorized access"
        );
    }

    #[test]
    fn test_display_user_not_found() {
        let err = ParkHubError::UserNotFound("alice".to_string());
        assert_eq!(err.to_string(), "User not found: alice");
    }

    #[test]
    fn test_display_slot_not_available() {
        assert_eq!(
            ParkHubError::SlotNotAvailable.to_string(),
            "Slot not available"
        );
    }

    #[test]
    fn test_display_booking_not_found() {
        let err = ParkHubError::BookingNotFound("b-42".to_string());
        assert_eq!(err.to_string(), "Booking not found: b-42");
    }

    #[test]
    fn test_display_booking_conflict() {
        assert_eq!(
            ParkHubError::BookingConflict.to_string(),
            "Booking conflict: slot already booked for this time"
        );
    }

    #[test]
    fn test_display_invalid_booking_time() {
        let err = ParkHubError::InvalidBookingTime("past date".to_string());
        assert_eq!(err.to_string(), "Invalid booking time: past date");
    }

    #[test]
    fn test_display_database() {
        let err = ParkHubError::Database("connection refused".to_string());
        assert_eq!(err.to_string(), "Database error: connection refused");
    }

    #[test]
    fn test_display_connection() {
        let err = ParkHubError::Connection("timeout".to_string());
        assert_eq!(err.to_string(), "Connection error: timeout");
    }

    #[test]
    fn test_display_server_not_found() {
        assert_eq!(ParkHubError::ServerNotFound.to_string(), "Server not found");
    }

    #[test]
    fn test_display_protocol_mismatch() {
        let err = ParkHubError::ProtocolMismatch {
            expected: "1.0.0".to_string(),
            actual: "2.0.0".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Protocol version mismatch: expected 1.0.0, got 2.0.0"
        );
    }

    #[test]
    fn test_display_config() {
        let err = ParkHubError::Config("missing key".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing key");
    }

    #[test]
    fn test_display_server_not_configured() {
        assert_eq!(
            ParkHubError::ServerNotConfigured.to_string(),
            "Server not configured"
        );
    }

    #[test]
    fn test_display_not_found() {
        let err = ParkHubError::NotFound("resource X".to_string());
        assert_eq!(err.to_string(), "Not found: resource X");
    }

    #[test]
    fn test_display_invalid_input() {
        let err = ParkHubError::InvalidInput("bad value".to_string());
        assert_eq!(err.to_string(), "Invalid input: bad value");
    }

    #[test]
    fn test_display_internal() {
        let err = ParkHubError::Internal("unexpected panic".to_string());
        assert_eq!(err.to_string(), "Internal error: unexpected panic");
    }

    #[test]
    fn test_debug_implements() {
        let err = ParkHubError::Unauthorized;
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn test_error_codes_values() {
        assert_eq!(error_codes::INVALID_CREDENTIALS, "INVALID_CREDENTIALS");
        assert_eq!(error_codes::TOKEN_EXPIRED, "TOKEN_EXPIRED");
        assert_eq!(error_codes::UNAUTHORIZED, "UNAUTHORIZED");
        assert_eq!(error_codes::USER_NOT_FOUND, "USER_NOT_FOUND");
        assert_eq!(error_codes::SLOT_NOT_AVAILABLE, "SLOT_NOT_AVAILABLE");
        assert_eq!(error_codes::BOOKING_NOT_FOUND, "BOOKING_NOT_FOUND");
        assert_eq!(error_codes::BOOKING_CONFLICT, "BOOKING_CONFLICT");
        assert_eq!(error_codes::INVALID_BOOKING_TIME, "INVALID_BOOKING_TIME");
        assert_eq!(error_codes::DATABASE_ERROR, "DATABASE_ERROR");
        assert_eq!(error_codes::CONNECTION_ERROR, "CONNECTION_ERROR");
        assert_eq!(error_codes::SERVER_NOT_FOUND, "SERVER_NOT_FOUND");
        assert_eq!(error_codes::PROTOCOL_MISMATCH, "PROTOCOL_MISMATCH");
        assert_eq!(error_codes::CONFIG_ERROR, "CONFIG_ERROR");
        assert_eq!(error_codes::NOT_FOUND, "NOT_FOUND");
        assert_eq!(error_codes::INVALID_INPUT, "INVALID_INPUT");
        assert_eq!(error_codes::INTERNAL_ERROR, "INTERNAL_ERROR");
    }

    #[test]
    fn test_std_error_source_is_none() {
        use std::error::Error;
        let err = ParkHubError::InvalidCredentials;
        assert!(err.source().is_none());
    }
}
