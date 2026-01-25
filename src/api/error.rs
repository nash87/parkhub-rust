//! API Error Types
//!
//! Comprehensive error handling for the parking API client.

use std::fmt;

/// API Error types
#[derive(Debug, Clone)]
pub enum ApiError {
    /// Network error - could not connect to server
    NetworkError(String),
    /// Server returned an error response
    ServerError { status: u16, message: String },
    /// Request timeout
    Timeout,
    /// Authentication failed
    Unauthorized,
    /// Resource not found
    NotFound(String),
    /// Validation error
    ValidationError(String),
    /// Slot already booked by someone else
    SlotUnavailable,
    /// User has reached booking limit
    BookingLimitReached,
    /// Invalid booking time
    InvalidBookingTime(String),
    /// Payment required
    PaymentRequired,
    /// Rate limited
    RateLimited { retry_after: u64 },
    /// Serialization/Deserialization error
    SerializationError(String),
    /// Local database error
    DatabaseError(String),
    /// Unknown error
    Unknown(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::ServerError { status, message } => {
                write!(f, "Server error ({}): {}", status, message)
            }
            ApiError::Timeout => write!(f, "Request timed out"),
            ApiError::Unauthorized => write!(f, "Authentication required"),
            ApiError::NotFound(resource) => write!(f, "Not found: {}", resource),
            ApiError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ApiError::SlotUnavailable => write!(f, "Parking slot is no longer available"),
            ApiError::BookingLimitReached => {
                write!(f, "You have reached your maximum booking limit")
            }
            ApiError::InvalidBookingTime(msg) => write!(f, "Invalid booking time: {}", msg),
            ApiError::PaymentRequired => write!(f, "Payment is required to complete booking"),
            ApiError::RateLimited { retry_after } => {
                write!(f, "Rate limited. Try again in {} seconds", retry_after)
            }
            ApiError::SerializationError(msg) => write!(f, "Data error: {}", msg),
            ApiError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ApiError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ApiError::Timeout
        } else if err.is_connect() {
            ApiError::NetworkError("Could not connect to server".to_string())
        } else if err.is_decode() {
            ApiError::SerializationError(err.to_string())
        } else {
            ApiError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::SerializationError(err.to_string())
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;
