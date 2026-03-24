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
    pub const fn code(&self) -> &'static str {
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
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials
            | Self::InvalidToken
            | Self::TokenExpired
            | Self::Unauthorized => StatusCode::UNAUTHORIZED,
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

impl From<redb::Error> for AppError {
    fn from(e: redb::Error) -> Self {
        tracing::error!("Database error: {:?}", e);
        Self::Database(e.to_string())
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
                    message: e
                        .message
                        .as_ref()
                        .map_or_else(|| e.code.to_string(), std::string::ToString::to_string),
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
    use axum::body::to_bytes;

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
        assert_eq!(AppError::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(
            AppError::InvalidInput("bad".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::AlreadyExists("thing".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::RateLimited.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            AppError::Internal.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::Database("oops".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_into_response_not_found() {
        let err = AppError::NotFound("parking lot".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "NOT_FOUND");
        assert!(json["message"].as_str().unwrap().contains("parking lot"));
    }

    #[tokio::test]
    async fn test_into_response_unauthorized() {
        let err = AppError::Unauthorized;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "UNAUTHORIZED");
        assert!(json.get("message").is_some());
    }

    #[tokio::test]
    async fn test_into_response_bad_request() {
        let err = AppError::InvalidInput("missing field".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "INVALID_INPUT");
    }

    #[tokio::test]
    async fn test_into_response_internal() {
        let err = AppError::Internal;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "INTERNAL_ERROR");
    }

    #[tokio::test]
    async fn test_into_response_validation_failed() {
        let errors = vec![FieldError {
            field: "email".into(),
            message: "invalid format".into(),
        }];
        let err = AppError::ValidationFailed(errors);
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "VALIDATION_FAILED");
        assert!(json["details"].is_array());
        assert_eq!(json["details"][0]["field"], "email");
    }

    // ── HEAD: exhaustive error codes ────────────────────────────────────────

    #[test]
    fn test_all_error_codes_exhaustive() {
        assert_eq!(AppError::InvalidCredentials.code(), "INVALID_CREDENTIALS");
        assert_eq!(AppError::TokenExpired.code(), "TOKEN_EXPIRED");
        assert_eq!(AppError::InvalidToken.code(), "INVALID_TOKEN");
        assert_eq!(AppError::Unauthorized.code(), "UNAUTHORIZED");
        assert_eq!(AppError::Forbidden.code(), "FORBIDDEN");
        assert_eq!(
            AppError::ValidationFailed(vec![]).code(),
            "VALIDATION_FAILED"
        );
        assert_eq!(AppError::InvalidInput("x".into()).code(), "INVALID_INPUT");
        assert_eq!(AppError::NotFound("y".into()).code(), "NOT_FOUND");
        assert_eq!(AppError::AlreadyExists("z".into()).code(), "ALREADY_EXISTS");
        assert_eq!(AppError::Conflict("c".into()).code(), "CONFLICT");
        assert_eq!(AppError::SlotNotAvailable.code(), "SLOT_NOT_AVAILABLE");
        assert_eq!(
            AppError::BookingNotModifiable.code(),
            "BOOKING_NOT_MODIFIABLE"
        );
        assert_eq!(AppError::InvalidBookingTime.code(), "INVALID_BOOKING_TIME");
        assert_eq!(AppError::RateLimited.code(), "RATE_LIMITED");
        assert_eq!(AppError::Database("e".into()).code(), "DATABASE_ERROR");
        assert_eq!(AppError::Internal.code(), "INTERNAL_ERROR");
    }

    #[test]
    fn test_all_status_codes_exhaustive() {
        assert_eq!(
            AppError::TokenExpired.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::InvalidToken.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::SlotNotAvailable.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::BookingNotModifiable.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::InvalidBookingTime.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::Conflict("x".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::AlreadyExists("x".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::ValidationFailed(vec![]).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_error_display_messages() {
        assert_eq!(
            AppError::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
        assert_eq!(AppError::TokenExpired.to_string(), "Token expired");
        assert_eq!(AppError::Unauthorized.to_string(), "Unauthorized");
        assert_eq!(AppError::Forbidden.to_string(), "Forbidden");
        assert_eq!(AppError::SlotNotAvailable.to_string(), "Slot not available");
        assert_eq!(
            AppError::NotFound("lot".into()).to_string(),
            "Resource not found: lot"
        );
        assert_eq!(
            AppError::InvalidInput("bad data".into()).to_string(),
            "Invalid input: bad data"
        );
        assert_eq!(AppError::Internal.to_string(), "Internal server error");
    }

    #[tokio::test]
    async fn test_into_response_forbidden() {
        let resp = AppError::Forbidden.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "FORBIDDEN");
    }

    #[tokio::test]
    async fn test_into_response_rate_limited() {
        let resp = AppError::RateLimited.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "RATE_LIMITED");
    }

    #[tokio::test]
    async fn test_into_response_conflict() {
        let resp = AppError::Conflict("booking overlap".into()).into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "CONFLICT");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("booking overlap"));
    }

    #[tokio::test]
    async fn test_into_response_slot_not_available() {
        let resp = AppError::SlotNotAvailable.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "SLOT_NOT_AVAILABLE");
    }

    #[tokio::test]
    async fn test_into_response_database_error() {
        let resp = AppError::Database("connection lost".into()).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "DATABASE_ERROR");
    }

    #[tokio::test]
    async fn test_validation_error_details_absent_for_non_validation_errors() {
        let resp = AppError::NotFound("thing".into()).into_response();
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json.get("details").is_none(),
            "non-validation errors must not include a `details` field"
        );
    }

    #[test]
    fn test_field_error_is_cloneable() {
        let fe = FieldError {
            field: "name".into(),
            message: "too short".into(),
        };
        let cloned = fe.clone();
        assert_eq!(cloned.field, fe.field);
        assert_eq!(cloned.message, fe.message);
    }

    // ── Copilot: Full code() coverage for all variants ──────────────────────

    #[test]
    fn test_all_error_codes() {
        assert_eq!(AppError::InvalidCredentials.code(), "INVALID_CREDENTIALS");
        assert_eq!(AppError::TokenExpired.code(), "TOKEN_EXPIRED");
        assert_eq!(AppError::InvalidToken.code(), "INVALID_TOKEN");
        assert_eq!(AppError::Unauthorized.code(), "UNAUTHORIZED");
        assert_eq!(AppError::Forbidden.code(), "FORBIDDEN");
        assert_eq!(
            AppError::ValidationFailed(vec![]).code(),
            "VALIDATION_FAILED"
        );
        assert_eq!(AppError::InvalidInput("x".into()).code(), "INVALID_INPUT");
        assert_eq!(AppError::NotFound("x".into()).code(), "NOT_FOUND");
        assert_eq!(AppError::AlreadyExists("x".into()).code(), "ALREADY_EXISTS");
        assert_eq!(AppError::Conflict("x".into()).code(), "CONFLICT");
        assert_eq!(AppError::SlotNotAvailable.code(), "SLOT_NOT_AVAILABLE");
        assert_eq!(
            AppError::BookingNotModifiable.code(),
            "BOOKING_NOT_MODIFIABLE"
        );
        assert_eq!(
            AppError::InvalidBookingTime.code(),
            "INVALID_BOOKING_TIME"
        );
        assert_eq!(AppError::RateLimited.code(), "RATE_LIMITED");
        assert_eq!(AppError::Database("x".into()).code(), "DATABASE_ERROR");
        assert_eq!(AppError::Internal.code(), "INTERNAL_ERROR");
    }

    // ── Copilot: Full status_code() coverage for all variants ───────────────

    #[test]
    fn test_all_status_codes() {
        assert_eq!(
            AppError::TokenExpired.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::InvalidToken.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::SlotNotAvailable.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::BookingNotModifiable.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::InvalidBookingTime.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::Conflict("x".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::ValidationFailed(vec![]).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    // ── Copilot: From<serde_json::Error> ────────────────────────────────────

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let app_err = AppError::from(json_err);
        assert_eq!(app_err.code(), "INVALID_INPUT");
        assert_eq!(app_err.status_code(), StatusCode::BAD_REQUEST);
    }

    // ── Copilot: IntoResponse for remaining variants ────────────────────────

    #[tokio::test]
    async fn test_into_response_already_exists() {
        let err = AppError::AlreadyExists("user@example.com".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "ALREADY_EXISTS");
    }

    // ── Copilot: validation_failed details absent for non-validation errors ─

    #[tokio::test]
    async fn test_non_validation_error_has_no_details() {
        let err = AppError::NotFound("slot-1".into());
        let resp = err.into_response();
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // "details" should not be present (skipped when None)
        assert!(json.get("details").is_none());
    }

    // ── Copilot: FieldError struct ──────────────────────────────────────────

    #[test]
    fn test_field_error_clone() {
        let fe = FieldError {
            field: "email".into(),
            message: "required".into(),
        };
        let cloned = fe.clone();
        assert_eq!(cloned.field, "email");
        assert_eq!(cloned.message, "required");
    }
}
