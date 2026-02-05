//! API Error Handling
//!
//! Provides structured error responses for the REST API.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// API Error Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional field-level errors (for validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
}

/// Field-level validation error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldError {
    /// Field name
    pub field: String,
    /// Error message
    pub message: String,
}

/// Application errors
#[derive(Debug, Error)]
pub enum AppError {
    // === Authentication Errors ===
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    // === Validation Errors ===
    #[error("Validation failed")]
    ValidationFailed(Vec<FieldError>),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    // === Resource Errors ===
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    // === Business Logic Errors ===
    #[error("Slot not available")]
    SlotNotAvailable,

    #[error("Booking cannot be modified")]
    BookingNotModifiable,

    #[error("Invalid booking time")]
    InvalidBookingTime,

    // === Rate Limiting ===
    #[error("Too many requests")]
    RateLimited,

    // === Server Errors ===
    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error")]
    Internal,
}

impl AppError {
    /// Get error code for this error type
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::ValidationFailed(_) => "VALIDATION_FAILED",
            Self::InvalidInput(_) => "INVALID_INPUT",
            Self::NotFound(_) => "NOT_FOUND",
            Self::AlreadyExists(_) => "ALREADY_EXISTS",
            Self::Conflict(_) => "CONFLICT",
            Self::SlotNotAvailable => "SLOT_NOT_AVAILABLE",
            Self::BookingNotModifiable => "BOOKING_NOT_MODIFIABLE",
            Self::InvalidBookingTime => "INVALID_BOOKING_TIME",
            Self::RateLimited => "RATE_LIMITED",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Internal => "INTERNAL_ERROR",
        }
    }

    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials | Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::TokenExpired => StatusCode::UNAUTHORIZED,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::ValidationFailed(_) | Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::AlreadyExists(_) | Self::Conflict(_) => StatusCode::CONFLICT,
            Self::SlotNotAvailable | Self::BookingNotModifiable | Self::InvalidBookingTime => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) | Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let details = if let Self::ValidationFailed(errors) = &self {
            Some(errors.clone())
        } else {
            None
        };

        let body = ApiError {
            code: self.code().to_string(),
            message: self.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

// Convert from common error types
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Internal error: {:?}", err);
        Self::Internal
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::InvalidInput(err.to_string())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let field_errors: Vec<FieldError> = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| FieldError {
                    field: field.to_string(),
                    message: e.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| {
                        e.code.to_string()
                    }),
                })
            })
            .collect();

        Self::ValidationFailed(field_errors)
    }
}

/// Result type alias for API handlers
pub type ApiResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(AppError::InvalidCredentials.code(), "INVALID_CREDENTIALS");
        assert_eq!(AppError::NotFound("user".into()).code(), "NOT_FOUND");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            AppError::InvalidCredentials.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::NotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(AppError::RateLimited.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }
}
