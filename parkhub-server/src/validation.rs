//! Input Validation
//!
//! Provides validation for API request payloads using the validator crate.

use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::AppError;

/// Validated JSON extractor
///
/// Extracts and validates JSON payloads in a single step.
/// Returns proper error responses for both parsing and validation failures.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Extract JSON
        let Json(value): Json<T> = Json::from_request(req, state)
            .await
            .map_err(|e: JsonRejection| AppError::InvalidInput(e.to_string()))?;

        // Validate
        value.validate()?;

        Ok(ValidatedJson(value))
    }
}

// === Common Validation Rules ===

use once_cell::sync::Lazy;
use regex::Regex;

/// Email regex pattern
pub static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

/// License plate regex (flexible for different formats)
pub static LICENSE_PLATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z0-9\-\s]{2,15}$").unwrap()
});

/// Username regex (alphanumeric + underscore, 3-30 chars)
pub static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]{2,29}$").unwrap()
});

/// Custom validator for license plates
pub fn validate_license_plate(plate: &str) -> Result<(), validator::ValidationError> {
    let normalized = plate.to_uppercase().replace(['-', ' '], "");
    if normalized.len() < 2 || normalized.len() > 10 {
        return Err(validator::ValidationError::new("invalid_license_plate"));
    }
    if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(validator::ValidationError::new("invalid_license_plate"));
    }
    Ok(())
}

/// Custom validator for booking duration
pub fn validate_booking_duration(minutes: i32) -> Result<(), validator::ValidationError> {
    if minutes < 15 {
        let mut err = validator::ValidationError::new("too_short");
        err.message = Some("Minimum booking duration is 15 minutes".into());
        return Err(err);
    }
    if minutes > 24 * 60 {
        let mut err = validator::ValidationError::new("too_long");
        err.message = Some("Maximum booking duration is 24 hours".into());
        return Err(err);
    }
    Ok(())
}

/// Custom validator for future datetime
pub fn validate_future_time(time: &chrono::DateTime<chrono::Utc>) -> Result<(), validator::ValidationError> {
    if *time <= chrono::Utc::now() {
        let mut err = validator::ValidationError::new("not_in_future");
        err.message = Some("Time must be in the future".into());
        return Err(err);
    }
    Ok(())
}

/// Custom validator for password strength
pub fn validate_password_strength(password: &str) -> Result<(), validator::ValidationError> {
    if password.len() < 8 {
        let mut err = validator::ValidationError::new("too_short");
        err.message = Some("Password must be at least 8 characters".into());
        return Err(err);
    }

    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if !has_lowercase || !has_uppercase || !has_digit {
        let mut err = validator::ValidationError::new("weak_password");
        err.message = Some("Password must contain lowercase, uppercase, and digit".into());
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_license_plate() {
        assert!(validate_license_plate("ABC-123").is_ok());
        assert!(validate_license_plate("B AB 1234").is_ok());
        assert!(validate_license_plate("MUC-X-1234").is_ok());
        assert!(validate_license_plate("A").is_err()); // Too short
        assert!(validate_license_plate("ABCDEFGHIJKLMNO").is_err()); // Too long
    }

    #[test]
    fn test_validate_booking_duration() {
        assert!(validate_booking_duration(30).is_ok());
        assert!(validate_booking_duration(120).is_ok());
        assert!(validate_booking_duration(10).is_err()); // Too short
        assert!(validate_booking_duration(25 * 60).is_err()); // Too long
    }

    #[test]
    fn test_validate_password_strength() {
        assert!(validate_password_strength("Password123").is_ok());
        assert!(validate_password_strength("SecurePass1").is_ok());
        assert!(validate_password_strength("short").is_err()); // Too short
        assert!(validate_password_strength("alllowercase1").is_err()); // No uppercase
        assert!(validate_password_strength("ALLUPPERCASE1").is_err()); // No lowercase
        assert!(validate_password_strength("NoDigitsHere").is_err()); // No digit
    }

    #[test]
    fn test_email_regex() {
        assert!(EMAIL_REGEX.is_match("test@example.com"));
        assert!(EMAIL_REGEX.is_match("user.name@domain.co.uk"));
        assert!(!EMAIL_REGEX.is_match("invalid"));
        assert!(!EMAIL_REGEX.is_match("@nodomain.com"));
    }

    #[test]
    fn test_username_regex() {
        assert!(USERNAME_REGEX.is_match("john_doe"));
        assert!(USERNAME_REGEX.is_match("User123"));
        assert!(!USERNAME_REGEX.is_match("ab")); // Too short
        assert!(!USERNAME_REGEX.is_match("123user")); // Starts with number
    }
}
