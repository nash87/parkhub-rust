//! Input Validation
//!
//! Provides validation for API request payloads using the validator crate.

use axum::{
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

        Ok(Self(value))
    }
}

// === Common Validation Rules ===

use std::sync::LazyLock;
use regex::Regex;

/// Email regex pattern
pub static EMAIL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

/// License plate regex (flexible for different formats)
pub static LICENSE_PLATE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z0-9\-\s]{2,15}$").unwrap());

/// Username regex (alphanumeric + underscore, 3-30 chars)
pub static USERNAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]{2,29}$").unwrap());

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
pub fn validate_future_time(
    time: &chrono::DateTime<chrono::Utc>,
) -> Result<(), validator::ValidationError> {
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
    fn test_license_plate_normalizes_case() {
        assert!(validate_license_plate("abc-123").is_ok());
        assert!(validate_license_plate("muc-x-1234").is_ok());
    }

    #[test]
    fn test_license_plate_strips_separators() {
        // Dashes and spaces are stripped, so "A-B" becomes "AB" (len 2, valid)
        assert!(validate_license_plate("A-B").is_ok());
        assert!(validate_license_plate("A B").is_ok());
    }

    #[test]
    fn test_license_plate_rejects_special_chars() {
        assert!(validate_license_plate("ABC!123").is_err());
        assert!(validate_license_plate("AB@CD").is_err());
        assert!(validate_license_plate("AB#CD").is_err());
    }

    #[test]
    fn test_license_plate_boundary_lengths() {
        // Exactly 2 chars after normalization = OK
        assert!(validate_license_plate("AB").is_ok());
        // Exactly 10 chars after normalization = OK
        assert!(validate_license_plate("ABCDEFGHIJ").is_ok());
        // 11 chars = too long
        assert!(validate_license_plate("ABCDEFGHIJK").is_err());
        // 1 char = too short
        assert!(validate_license_plate("A").is_err());
    }

    #[test]
    fn test_license_plate_empty() {
        assert!(validate_license_plate("").is_err());
    }

    #[test]
    fn test_validate_booking_duration() {
        assert!(validate_booking_duration(30).is_ok());
        assert!(validate_booking_duration(120).is_ok());
        assert!(validate_booking_duration(10).is_err()); // Too short
        assert!(validate_booking_duration(25 * 60).is_err()); // Too long
    }

    #[test]
    fn test_booking_duration_boundary_values() {
        // Exactly 15 minutes = OK (minimum)
        assert!(validate_booking_duration(15).is_ok());
        // 14 minutes = too short
        assert!(validate_booking_duration(14).is_err());
        // Exactly 24*60 = 1440 minutes = OK (maximum)
        assert!(validate_booking_duration(24 * 60).is_ok());
        // 1441 minutes = too long
        assert!(validate_booking_duration(24 * 60 + 1).is_err());
    }

    #[test]
    fn test_booking_duration_error_messages() {
        let err = validate_booking_duration(5).unwrap_err();
        assert_eq!(err.code.as_ref(), "too_short");
        assert!(err.message.as_ref().unwrap().contains("15 minutes"));

        let err = validate_booking_duration(2000).unwrap_err();
        assert_eq!(err.code.as_ref(), "too_long");
        assert!(err.message.as_ref().unwrap().contains("24 hours"));
    }

    #[test]
    fn test_booking_duration_zero_and_negative() {
        assert!(validate_booking_duration(0).is_err());
        assert!(validate_booking_duration(-1).is_err());
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
    fn test_password_minimum_length() {
        // Exactly 8 chars with all requirements met
        assert!(validate_password_strength("Abcdefg1").is_ok());
        // 7 chars = too short
        assert!(validate_password_strength("Abcdef1").is_err());
    }

    #[test]
    fn test_password_error_codes() {
        let err = validate_password_strength("short").unwrap_err();
        assert_eq!(err.code.as_ref(), "too_short");

        let err = validate_password_strength("nouppercase1234").unwrap_err();
        assert_eq!(err.code.as_ref(), "weak_password");
    }

    #[test]
    fn test_password_empty() {
        assert!(validate_password_strength("").is_err());
    }

    #[test]
    fn test_password_with_special_chars() {
        assert!(validate_password_strength("P@ssw0rd!").is_ok());
        assert!(validate_password_strength("Str0ng#Pass").is_ok());
    }

    #[test]
    fn test_email_regex() {
        assert!(EMAIL_REGEX.is_match("test@example.com"));
        assert!(EMAIL_REGEX.is_match("user.name@domain.co.uk"));
        assert!(!EMAIL_REGEX.is_match("invalid"));
        assert!(!EMAIL_REGEX.is_match("@nodomain.com"));
    }

    #[test]
    fn test_email_regex_edge_cases() {
        assert!(EMAIL_REGEX.is_match("a@b.co"));
        assert!(EMAIL_REGEX.is_match("user+tag@example.com"));
        assert!(EMAIL_REGEX.is_match("first.last@sub.domain.com"));
        assert!(!EMAIL_REGEX.is_match(""));
        assert!(!EMAIL_REGEX.is_match("user@"));
        assert!(!EMAIL_REGEX.is_match("user@.com"));
        assert!(!EMAIL_REGEX.is_match("user@domain"));
        assert!(!EMAIL_REGEX.is_match("user@domain.c")); // TLD too short
    }

    #[test]
    fn test_username_regex() {
        assert!(USERNAME_REGEX.is_match("john_doe"));
        assert!(USERNAME_REGEX.is_match("User123"));
        assert!(!USERNAME_REGEX.is_match("ab")); // Too short
        assert!(!USERNAME_REGEX.is_match("123user")); // Starts with number
    }

    #[test]
    fn test_username_regex_boundary_lengths() {
        // 3 chars minimum (1 letter + 2 more)
        assert!(USERNAME_REGEX.is_match("abc"));
        // 30 chars maximum
        assert!(USERNAME_REGEX.is_match("a23456789012345678901234567890"));
        // 31 chars = too long
        assert!(!USERNAME_REGEX.is_match("a234567890123456789012345678901"));
    }

    #[test]
    fn test_username_regex_special_chars() {
        assert!(USERNAME_REGEX.is_match("user_name"));
        assert!(!USERNAME_REGEX.is_match("user-name")); // No dashes
        assert!(!USERNAME_REGEX.is_match("user.name")); // No dots
        assert!(!USERNAME_REGEX.is_match("user name")); // No spaces
        assert!(!USERNAME_REGEX.is_match("user@name")); // No @
    }

    #[test]
    fn test_username_must_start_with_letter() {
        assert!(USERNAME_REGEX.is_match("abc"));
        assert!(USERNAME_REGEX.is_match("Abc"));
        assert!(!USERNAME_REGEX.is_match("_abc")); // Starts with underscore
        assert!(!USERNAME_REGEX.is_match("1abc")); // Starts with digit
    }

    #[test]
    fn test_validate_future_time() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        assert!(validate_future_time(&future).is_ok());

        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        assert!(validate_future_time(&past).is_err());
    }

    #[test]
    fn test_validate_future_time_error_code() {
        let past = chrono::Utc::now() - chrono::Duration::seconds(10);
        let err = validate_future_time(&past).unwrap_err();
        assert_eq!(err.code.as_ref(), "not_in_future");
    }
}
