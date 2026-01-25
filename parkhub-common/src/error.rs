//! Error Types
//!
//! Common error types used across ParkHub.

use thiserror::Error;

/// Common errors for the ParkHub system
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
