//! Input Validation
//!
//! Provides validation for API request payloads using the validator crate.

use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
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

use regex::Regex;
use std::sync::LazyLock;

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

// ── Property-based tests ───────────────────────────────────────────────────
//
// These mirror the parkhub-common proptest pattern (see
// parkhub-common/tests/validation_properties.rs). They cover the four
// pure server-side validators with boundary, adversarial, and invariant
// properties at the default 256 cases per block.
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // ── validate_license_plate ────────────────────────────────────────────

    proptest! {
        /// Any string of 2–10 uppercase ASCII alphanumerics validates.
        /// Anchors the happy path so the validator can't accidentally
        /// over-reject.
        #[test]
        fn license_plate_alphanumeric_2_to_10_accepted(
            plate in "[A-Z0-9]{2,10}",
        ) {
            prop_assert!(validate_license_plate(&plate).is_ok());
        }

        /// Length below 2 (after stripping `-` and ` `) is always rejected.
        #[test]
        fn license_plate_too_short_rejected(plate in "[A-Z0-9]{0,1}") {
            prop_assert!(validate_license_plate(&plate).is_err());
        }

        /// Length above 10 (after stripping `-` and ` `) is always rejected.
        #[test]
        fn license_plate_too_long_rejected(plate in "[A-Z0-9]{11,40}") {
            prop_assert!(validate_license_plate(&plate).is_err());
        }

        /// Case-invariance: `validate_license_plate` normalises to upper
        /// before checking, so the lowercase version of any valid plate
        /// must also validate.
        #[test]
        fn license_plate_case_invariant(plate in "[A-Z0-9]{2,10}") {
            prop_assert_eq!(
                validate_license_plate(&plate).is_ok(),
                validate_license_plate(&plate.to_lowercase()).is_ok(),
            );
        }

        /// Inserting `-` or ` ` separators into any valid 2–10 plate
        /// must not change the validation result. The validator strips
        /// these before length-checking.
        #[test]
        fn license_plate_separator_invariant(
            plate in "[A-Z0-9]{2,10}",
            sep in prop_oneof![Just('-'), Just(' ')],
        ) {
            // Insert one separator at the midpoint.
            let mid = plate.len() / 2;
            let mut with_sep = String::with_capacity(plate.len() + 1);
            with_sep.push_str(&plate[..mid]);
            with_sep.push(sep);
            with_sep.push_str(&plate[mid..]);
            prop_assert_eq!(
                validate_license_plate(&plate).is_ok(),
                validate_license_plate(&with_sep).is_ok(),
            );
        }
    }

    // ── validate_booking_duration ─────────────────────────────────────────

    proptest! {
        /// `[15, 1440]` is the documented closed interval — every value
        /// inside accepts.
        #[test]
        fn booking_duration_inside_bounds_accepted(minutes in 15i32..=24*60) {
            prop_assert!(validate_booking_duration(minutes).is_ok());
        }

        /// Anything strictly below 15 (including the full negative range
        /// and `i32::MIN`) is rejected with `too_short`.
        #[test]
        fn booking_duration_below_15_rejected(minutes in i32::MIN..15) {
            let err = validate_booking_duration(minutes).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "too_short");
        }

        /// Anything strictly above 1440 (including `i32::MAX`) is
        /// rejected with `too_long`.
        #[test]
        fn booking_duration_above_1440_rejected(minutes in (24*60 + 1)..=i32::MAX) {
            let err = validate_booking_duration(minutes).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "too_long");
        }
    }

    // ── validate_future_time ──────────────────────────────────────────────

    proptest! {
        /// Any time `now + delta` for positive `delta_secs` is accepted.
        /// Skips a small floor (~5s) to absorb clock drift between
        /// `Utc::now()` calls.
        #[test]
        fn future_time_accepted(delta_secs in 5i64..=(365 * 24 * 60 * 60)) {
            let t = chrono::Utc::now() + chrono::Duration::seconds(delta_secs);
            prop_assert!(validate_future_time(&t).is_ok());
        }

        /// Any time `now - delta` for positive `delta_secs` is rejected
        /// with `not_in_future`.
        #[test]
        fn past_time_rejected(delta_secs in 1i64..=(365 * 24 * 60 * 60)) {
            let t = chrono::Utc::now() - chrono::Duration::seconds(delta_secs);
            let err = validate_future_time(&t).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "not_in_future");
        }
    }

    // ── validate_password_strength ────────────────────────────────────────

    proptest! {
        /// Any password ≥ 8 chars containing at least one lowercase, one
        /// uppercase, and one digit is accepted. The strategy hand-builds
        /// candidates with all three classes guaranteed.
        #[test]
        fn password_with_all_classes_accepted(
            lower in "[a-z]{2,16}",
            upper in "[A-Z]{2,16}",
            digit in "[0-9]{2,16}",
        ) {
            let pw = format!("{lower}{upper}{digit}");
            prop_assume!(pw.len() >= 8); // belt + suspenders
            prop_assert!(validate_password_strength(&pw).is_ok());
        }

        /// Anything strictly shorter than 8 chars is rejected with
        /// `too_short`, regardless of character mix.
        #[test]
        fn password_under_8_rejected(pw in "[A-Za-z0-9]{0,7}") {
            let err = validate_password_strength(&pw).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "too_short");
        }

        /// All-lowercase ≥ 8 chars is rejected with `weak_password`
        /// (missing uppercase + digit).
        #[test]
        fn password_all_lowercase_rejected(pw in "[a-z]{8,32}") {
            let err = validate_password_strength(&pw).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "weak_password");
        }

        /// All-uppercase ≥ 8 chars is rejected with `weak_password`
        /// (missing lowercase + digit).
        #[test]
        fn password_all_uppercase_rejected(pw in "[A-Z]{8,32}") {
            let err = validate_password_strength(&pw).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "weak_password");
        }

        /// All-digit ≥ 8 chars is rejected with `weak_password`
        /// (missing lowercase + uppercase).
        #[test]
        fn password_all_digit_rejected(pw in "[0-9]{8,32}") {
            let err = validate_password_strength(&pw).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "weak_password");
        }

        /// Missing exactly one class (e.g. only lower+digit, no upper)
        /// is rejected with `weak_password`. Catches a regression where
        /// the validator accepts only-2-of-3 instead of all-3.
        #[test]
        fn password_missing_one_class_rejected(
            lower in "[a-z]{4,16}",
            digit in "[0-9]{4,16}",
        ) {
            let pw = format!("{lower}{digit}");
            prop_assume!(pw.len() >= 8);
            let err = validate_password_strength(&pw).unwrap_err();
            prop_assert_eq!(err.code.as_ref(), "weak_password");
        }
    }
}
