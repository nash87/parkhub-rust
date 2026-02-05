//! Request DTOs with Validation
//!
//! Defines all API request payloads with built-in validation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::validation::{validate_booking_duration, validate_license_plate, validate_password_strength};

// ═══════════════════════════════════════════════════════════════════════════════
// AUTHENTICATION REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Login request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    /// Username or email
    #[validate(length(min = 3, max = 100, message = "Username must be 3-100 characters"))]
    pub username: String,

    /// Password
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Registration request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// Username (3-30 alphanumeric + underscore)
    #[validate(length(min = 3, max = 30, message = "Username must be 3-30 characters"))]
    #[validate(regex(path = "*crate::validation::USERNAME_REGEX", message = "Invalid username format"))]
    pub username: String,

    /// Email address
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    /// Password (min 8 chars, must include upper, lower, digit)
    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,

    /// Full name
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,

    /// Phone number (optional)
    #[validate(length(max = 20, message = "Phone number too long"))]
    pub phone: Option<String>,
}

/// Password change request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: String,

    /// New password
    #[validate(custom(function = "validate_password_strength"))]
    pub new_password: String,
}

/// Token refresh request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequest {
    /// Refresh token
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create booking request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateBookingRequest {
    /// Parking lot ID
    pub lot_id: Uuid,

    /// Parking slot ID
    pub slot_id: Uuid,

    /// Booking start time (must be in future)
    pub start_time: DateTime<Utc>,

    /// Booking duration in minutes (15 min - 24 hours)
    #[validate(custom(function = "validate_booking_duration"))]
    pub duration_minutes: i32,

    /// Vehicle ID (for returning users)
    pub vehicle_id: Option<Uuid>,

    /// License plate (required if no vehicle_id)
    #[validate(custom(function = "validate_license_plate"))]
    pub license_plate: Option<String>,

    /// Optional notes
    #[validate(length(max = 500, message = "Notes too long"))]
    pub notes: Option<String>,
}

/// Extend booking request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ExtendBookingRequest {
    /// Additional minutes to extend (15 min - 8 hours)
    #[validate(range(min = 15, max = 480, message = "Extension must be 15 min to 8 hours"))]
    pub additional_minutes: i32,
}

/// Update booking request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateBookingRequest {
    /// New start time (optional)
    pub start_time: Option<DateTime<Utc>>,

    /// New duration in minutes (optional)
    #[validate(custom(function = "validate_booking_duration"))]
    pub duration_minutes: Option<i32>,

    /// Update notes
    #[validate(length(max = 500, message = "Notes too long"))]
    pub notes: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VEHICLE REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create/Update vehicle request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VehicleRequest {
    /// License plate
    #[validate(custom(function = "validate_license_plate"))]
    pub license_plate: String,

    /// Vehicle make (e.g., "BMW", "Toyota")
    #[validate(length(max = 50, message = "Make too long"))]
    pub make: Option<String>,

    /// Vehicle model (e.g., "X5", "Camry")
    #[validate(length(max = 50, message = "Model too long"))]
    pub model: Option<String>,

    /// Vehicle color
    #[validate(length(max = 30, message = "Color too long"))]
    pub color: Option<String>,

    /// Vehicle type
    pub vehicle_type: Option<String>,

    /// Set as default vehicle
    #[serde(default)]
    pub is_default: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// USER REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Update user profile request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileRequest {
    /// Full name
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,

    /// Email address
    #[validate(email(message = "Invalid email address"))]
    pub email: Option<String>,

    /// Phone number
    #[validate(length(max = 20, message = "Phone number too long"))]
    pub phone: Option<String>,

    /// Profile picture URL
    #[validate(url(message = "Invalid URL"))]
    pub picture: Option<String>,
}

/// Update user preferences request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePreferencesRequest {
    /// Default booking duration in minutes
    #[validate(range(min = 15, max = 480, message = "Duration must be 15 min to 8 hours"))]
    pub default_duration_minutes: Option<i32>,

    /// Enable notifications
    pub notifications_enabled: Option<bool>,

    /// Enable email reminders
    pub email_reminders: Option<bool>,

    /// Preferred language (ISO 639-1)
    #[validate(length(min = 2, max = 5, message = "Invalid language code"))]
    pub language: Option<String>,

    /// Preferred theme (light/dark/system)
    #[validate(length(max = 10))]
    pub theme: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create parking lot request (admin)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateParkingLotRequest {
    /// Lot name
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,

    /// Address
    #[validate(length(min = 1, max = 500, message = "Address too long"))]
    pub address: String,

    /// Latitude
    #[validate(range(min = -90.0, max = 90.0, message = "Invalid latitude"))]
    pub latitude: f64,

    /// Longitude
    #[validate(range(min = -180.0, max = 180.0, message = "Invalid longitude"))]
    pub longitude: f64,

    /// Total number of slots
    #[validate(range(min = 1, max = 10000, message = "Slots must be 1-10000"))]
    pub total_slots: i32,
}

/// Update parking lot request (admin)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateParkingLotRequest {
    /// Lot name
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,

    /// Address
    #[validate(length(min = 1, max = 500, message = "Address too long"))]
    pub address: Option<String>,

    /// Lot status
    pub status: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUERY PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Pagination parameters
#[derive(Debug, Deserialize, Validate, ToSchema, Default)]
pub struct PaginationParams {
    /// Page number (1-based)
    #[validate(range(min = 1, message = "Page must be >= 1"))]
    #[serde(default = "default_page")]
    pub page: i32,

    /// Items per page
    #[validate(range(min = 1, max = 100, message = "Per page must be 1-100"))]
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 { 1 }
fn default_per_page() -> i32 { 20 }

/// Booking list filters
#[derive(Debug, Deserialize, Validate, ToSchema, Default)]
pub struct BookingFiltersParams {
    /// Filter by status
    pub status: Option<String>,

    /// Filter by lot ID
    pub lot_id: Option<Uuid>,

    /// From date
    pub from_date: Option<DateTime<Utc>>,

    /// To date
    pub to_date: Option<DateTime<Utc>>,

    /// Pagination
    #[serde(flatten)]
    #[validate(nested)]
    pub pagination: PaginationParams,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_validation() {
        let valid = LoginRequest {
            username: "testuser".to_string(),
            password: "password".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = LoginRequest {
            username: "ab".to_string(), // Too short
            password: "".to_string(),   // Empty
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_register_request_validation() {
        let valid = RegisterRequest {
            username: "newuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
            name: "Test User".to_string(),
            phone: None,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_create_booking_validation() {
        let valid = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 60,
            vehicle_id: None,
            license_plate: Some("ABC-123".to_string()),
            notes: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: Utc::now(),
            duration_minutes: 5, // Too short
            vehicle_id: None,
            license_plate: Some("A".to_string()), // Too short
            notes: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_pagination_defaults() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 0); // Default struct default, not serde default
    }
}
