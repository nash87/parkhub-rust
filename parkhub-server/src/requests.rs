//! Request DTOs with Validation
//!
//! Defines all API request payloads with built-in validation.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::validation::{
    validate_booking_duration, validate_license_plate, validate_password_strength,
};

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
    #[validate(regex(
        path = "*crate::validation::USERNAME_REGEX",
        message = "Invalid username format"
    ))]
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
    #[serde(default)]
    pub vehicle_id: Option<Uuid>,

    /// License plate (required if no `vehicle_id`)
    #[serde(default)]
    #[validate(custom(function = "validate_license_plate"))]
    pub license_plate: Option<String>,

    /// Optional notes
    #[serde(default)]
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
    #[serde(default)]
    #[validate(length(max = 50, message = "Make too long"))]
    pub make: Option<String>,

    /// Vehicle model (e.g., "X5", "Camry")
    #[serde(default)]
    #[validate(length(max = 50, message = "Model too long"))]
    pub model: Option<String>,

    /// Vehicle color
    #[serde(default)]
    #[validate(length(max = 30, message = "Color too long"))]
    pub color: Option<String>,

    /// Vehicle type
    #[serde(default)]
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

/// Update user monthly credit quota (admin)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateQuotaRequest {
    /// Monthly credit quota (0 = unlimited/no quota, max 999)
    #[validate(range(min = 0, max = 999, message = "Quota must be 0-999"))]
    pub monthly_quota: i32,
}

/// Create parking lot request (admin)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateParkingLotRequest {
    /// Lot name
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,

    /// Address (optional)
    #[serde(default)]
    #[validate(length(max = 500, message = "Address too long"))]
    pub address: Option<String>,

    /// Latitude (optional, defaults to 0.0)
    #[serde(default)]
    #[validate(range(min = -90.0, max = 90.0, message = "Invalid latitude"))]
    pub latitude: Option<f64>,

    /// Longitude (optional, defaults to 0.0)
    #[serde(default)]
    #[validate(range(min = -180.0, max = 180.0, message = "Invalid longitude"))]
    pub longitude: Option<f64>,

    /// Total number of parking slots to auto-generate (defaults to 10)
    #[serde(default = "default_total_slots")]
    #[validate(range(min = 1, max = 500, message = "Slots must be 1-500"))]
    pub total_slots: i32,

    /// Hourly parking rate (optional)
    #[serde(default)]
    #[validate(range(min = 0.0, max = 1000.0, message = "Hourly rate must be 0-1000"))]
    pub hourly_rate: Option<f64>,

    /// Daily maximum charge (optional)
    #[serde(default)]
    #[validate(range(min = 0.0, max = 10000.0, message = "Daily max must be 0-10000"))]
    pub daily_max: Option<f64>,

    /// Monthly pass price (optional)
    #[serde(default)]
    #[validate(range(min = 0.0, max = 100_000.0, message = "Monthly pass must be 0-100000"))]
    pub monthly_pass: Option<f64>,

    /// Currency code (defaults to "EUR")
    #[serde(default = "default_currency")]
    #[validate(length(min = 3, max = 3, message = "Currency must be a 3-letter code"))]
    pub currency: String,

    /// Lot status (defaults to "open"). Valid: "open", "closed", "full", "maintenance"
    #[serde(default)]
    pub status: Option<String>,
}

/// Update parking lot request (admin)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateParkingLotRequest {
    /// Lot name
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,

    /// Address
    #[validate(length(max = 500, message = "Address too long"))]
    pub address: Option<String>,

    /// Latitude
    #[validate(range(min = -90.0, max = 90.0, message = "Invalid latitude"))]
    pub latitude: Option<f64>,

    /// Longitude
    #[validate(range(min = -180.0, max = 180.0, message = "Invalid longitude"))]
    pub longitude: Option<f64>,

    /// Total number of slots (changing this will NOT auto-generate/remove slots)
    #[validate(range(min = 1, max = 10000, message = "Slots must be 1-10000"))]
    pub total_slots: Option<i32>,

    /// Hourly parking rate
    #[validate(range(min = 0.0, max = 1000.0, message = "Hourly rate must be 0-1000"))]
    pub hourly_rate: Option<f64>,

    /// Daily maximum charge
    #[validate(range(min = 0.0, max = 10000.0, message = "Daily max must be 0-10000"))]
    pub daily_max: Option<f64>,

    /// Monthly pass price
    #[validate(range(min = 0.0, max = 100_000.0, message = "Monthly pass must be 0-100000"))]
    pub monthly_pass: Option<f64>,

    /// Currency code
    #[validate(length(min = 3, max = 3, message = "Currency must be a 3-letter code"))]
    pub currency: Option<String>,

    /// Lot status. Valid: "open", "closed", "full", "maintenance"
    pub status: Option<String>,
}

fn default_currency() -> String {
    "EUR".to_string()
}

const fn default_total_slots() -> i32 {
    10
}

/// Parse a status string into a `LotStatus` enum.
/// Returns None for unrecognized values.
pub fn parse_lot_status(s: &str) -> Option<parkhub_common::models::LotStatus> {
    use parkhub_common::models::LotStatus;
    match s {
        "open" => Some(LotStatus::Open),
        "closed" => Some(LotStatus::Closed),
        "full" => Some(LotStatus::Full),
        "maintenance" => Some(LotStatus::Maintenance),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUERY PARAMETERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Pagination parameters
#[derive(Debug, Deserialize, Validate, ToSchema, utoipa::IntoParams, Default)]
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

const fn default_page() -> i32 {
    1
}
const fn default_per_page() -> i32 {
    20
}

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

    // ── CreateParkingLotRequest serde tests ──────────────────────────────────

    #[test]
    fn test_create_lot_request_minimal_json() {
        let json = r#"{"name": "Test Lot"}"#;
        let req: CreateParkingLotRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Test Lot");
        assert_eq!(req.total_slots, 10); // default
        assert_eq!(req.currency, "EUR"); // default
        assert!(req.address.is_none());
        assert!(req.latitude.is_none());
        assert!(req.longitude.is_none());
        assert!(req.hourly_rate.is_none());
        assert!(req.status.is_none());
    }

    #[test]
    fn test_create_lot_request_full_json() {
        let json = r#"{
            "name": "Full Lot",
            "address": "123 Main St",
            "latitude": 48.137154,
            "longitude": 11.576124,
            "total_slots": 50,
            "hourly_rate": 2.50,
            "daily_max": 20.0,
            "monthly_pass": 150.0,
            "currency": "USD",
            "status": "open"
        }"#;
        let req: CreateParkingLotRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Full Lot");
        assert_eq!(req.address.as_deref(), Some("123 Main St"));
        assert!((req.latitude.unwrap() - 48.137154).abs() < 1e-6);
        assert!((req.longitude.unwrap() - 11.576124).abs() < 1e-6);
        assert_eq!(req.total_slots, 50);
        assert!((req.hourly_rate.unwrap() - 2.50).abs() < 1e-6);
        assert!((req.daily_max.unwrap() - 20.0).abs() < 1e-6);
        assert!((req.monthly_pass.unwrap() - 150.0).abs() < 1e-6);
        assert_eq!(req.currency, "USD");
        assert_eq!(req.status.as_deref(), Some("open"));
    }

    #[test]
    fn test_create_lot_request_validation_name_too_long() {
        let req = CreateParkingLotRequest {
            name: "A".repeat(101),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 10,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_validation_slots_out_of_range() {
        let too_many = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 501,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(too_many.validate().is_err());

        let zero = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 0,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(zero.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_validation_invalid_latitude() {
        let req = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: Some(91.0),
            longitude: None,
            total_slots: 10,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_validation_invalid_longitude() {
        let req = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: None,
            longitude: Some(-181.0),
            total_slots: 10,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_validation_currency_too_short() {
        let req = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 10,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EU".to_string(),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_validation_valid_boundaries() {
        let req = CreateParkingLotRequest {
            name: "A".to_string(), // min 1
            address: None,
            latitude: Some(-90.0),  // min boundary
            longitude: Some(180.0), // max boundary
            total_slots: 1,         // min boundary
            hourly_rate: Some(0.0), // min boundary
            daily_max: Some(10000.0),
            monthly_pass: Some(100000.0),
            currency: "CHF".to_string(),
            status: None,
        };
        assert!(req.validate().is_ok());
    }

    // ── UpdateParkingLotRequest serde tests ──────────────────────────────────

    #[test]
    fn test_update_lot_request_empty_json() {
        let json = r#"{}"#;
        let req: UpdateParkingLotRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.address.is_none());
        assert!(req.total_slots.is_none());
        assert!(req.status.is_none());
        assert!(req.currency.is_none());
    }

    #[test]
    fn test_update_lot_request_partial_json() {
        let json = r#"{"name": "Updated", "status": "closed"}"#;
        let req: UpdateParkingLotRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Updated"));
        assert_eq!(req.status.as_deref(), Some("closed"));
        assert!(req.address.is_none());
    }

    #[test]
    fn test_update_lot_request_validation_name_empty() {
        let req = UpdateParkingLotRequest {
            name: Some("".to_string()),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: None,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: None,
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_lot_request_validation_slots_too_large() {
        let req = UpdateParkingLotRequest {
            name: None,
            address: None,
            latitude: None,
            longitude: None,
            total_slots: Some(10001),
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: None,
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_lot_request_validation_negative_hourly_rate() {
        let req = UpdateParkingLotRequest {
            name: None,
            address: None,
            latitude: None,
            longitude: None,
            total_slots: None,
            hourly_rate: Some(-1.0),
            daily_max: None,
            monthly_pass: None,
            currency: None,
            status: None,
        };
        assert!(req.validate().is_err());
    }

    // ── UpdateQuotaRequest serde/validation tests ────────────────────────────

    #[test]
    fn test_update_quota_request_deserialize() {
        let json = r#"{"monthly_quota": 50}"#;
        let req: UpdateQuotaRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.monthly_quota, 50);
    }

    #[test]
    fn test_update_quota_request_zero_is_valid() {
        let req = UpdateQuotaRequest { monthly_quota: 0 };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_quota_request_max_boundary() {
        let req = UpdateQuotaRequest { monthly_quota: 999 };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_quota_request_over_max() {
        let req = UpdateQuotaRequest {
            monthly_quota: 1000,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_quota_request_negative() {
        let req = UpdateQuotaRequest { monthly_quota: -1 };
        assert!(req.validate().is_err());
    }

    // ── parse_lot_status tests ───────────────────────────────────────────────

    #[test]
    fn test_parse_lot_status_valid() {
        assert!(parse_lot_status("open").is_some());
        assert!(parse_lot_status("closed").is_some());
        assert!(parse_lot_status("full").is_some());
        assert!(parse_lot_status("maintenance").is_some());
    }

    #[test]
    fn test_parse_lot_status_invalid() {
        assert!(parse_lot_status("").is_none());
        assert!(parse_lot_status("Open").is_none()); // case sensitive
        assert!(parse_lot_status("unknown").is_none());
        assert!(parse_lot_status("CLOSED").is_none());
    }

    // ── Booking request edge cases ───────────────────────────────────────────

    #[test]
    fn test_create_booking_request_boundary_duration() {
        // Minimum valid duration: 15 min
        let min_valid = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 15,
            vehicle_id: None,
            license_plate: Some("AB-CD-123".to_string()),
            notes: None,
        };
        assert!(min_valid.validate().is_ok());

        // Max valid duration: 1440 min (24 hours)
        let max_valid = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 1440,
            vehicle_id: None,
            license_plate: Some("AB-CD-123".to_string()),
            notes: None,
        };
        assert!(max_valid.validate().is_ok());
    }

    #[test]
    fn test_extend_booking_request_validation() {
        let valid = ExtendBookingRequest {
            additional_minutes: 60,
        };
        assert!(valid.validate().is_ok());

        let too_short = ExtendBookingRequest {
            additional_minutes: 10,
        };
        assert!(too_short.validate().is_err());

        let too_long = ExtendBookingRequest {
            additional_minutes: 500,
        };
        assert!(too_long.validate().is_err());
    }

    #[test]
    fn test_update_profile_request_invalid_email() {
        let req = UpdateProfileRequest {
            name: None,
            email: Some("not-an-email".to_string()),
            phone: None,
            picture: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_profile_request_invalid_url() {
        let req = UpdateProfileRequest {
            name: None,
            email: None,
            phone: None,
            picture: Some("not-a-url".to_string()),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_preferences_request_valid() {
        let req = UpdatePreferencesRequest {
            default_duration_minutes: Some(60),
            notifications_enabled: Some(true),
            email_reminders: Some(false),
            language: Some("de".to_string()),
            theme: Some("dark".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_preferences_request_duration_out_of_range() {
        let req = UpdatePreferencesRequest {
            default_duration_minutes: Some(10), // below 15 min
            notifications_enabled: None,
            email_reminders: None,
            language: None,
            theme: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_change_password_request_weak_password() {
        let req = ChangePasswordRequest {
            current_password: "old_pass".to_string(),
            new_password: "weak".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_vehicle_request_valid() {
        let req = VehicleRequest {
            license_plate: "M-AB-1234".to_string(),
            make: Some("BMW".to_string()),
            model: Some("X5".to_string()),
            color: Some("Black".to_string()),
            vehicle_type: Some("suv".to_string()),
            is_default: true,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_vehicle_request_plate_too_short() {
        let req = VehicleRequest {
            license_plate: "A".to_string(),
            make: None,
            model: None,
            color: None,
            vehicle_type: None,
            is_default: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_notes_too_long() {
        let req = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 60,
            vehicle_id: None,
            license_plate: Some("AB-CD-123".to_string()),
            notes: Some("x".repeat(501)),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_booking_filters_deserialize() {
        let json = r#"{"status": "confirmed", "page": 2, "per_page": 10}"#;
        let params: BookingFiltersParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.status.as_deref(), Some("confirmed"));
        assert_eq!(params.pagination.page, 2);
        assert_eq!(params.pagination.per_page, 10);
    }

    #[test]
    fn test_booking_filters_defaults_from_serde() {
        let json = r#"{}"#;
        let params: BookingFiltersParams = serde_json::from_str(json).unwrap();
        assert!(params.status.is_none());
        assert!(params.lot_id.is_none());
        assert_eq!(params.pagination.page, 1);
        assert_eq!(params.pagination.per_page, 20);
    }

    #[test]
    fn test_refresh_token_request_validation() {
        let valid = RefreshTokenRequest {
            refresh_token: "some-valid-token".to_string(),
        };
        assert!(valid.validate().is_ok());

        let empty = RefreshTokenRequest {
            refresh_token: "".to_string(),
        };
        assert!(empty.validate().is_err());
    }

    // ── LoginRequest serde round-trip ────────────────────────────────────────

    #[test]
    fn test_login_request_serde_from_json_value() {
        let json = serde_json::json!({"username": "admin", "password": "secret"});
        let req: LoginRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.username, "admin");
        assert_eq!(req.password, "secret");
    }

    #[test]
    fn test_login_request_missing_password_fails() {
        let json = serde_json::json!({"username": "admin"});
        let result: Result<LoginRequest, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_login_request_missing_username_fails() {
        let json = serde_json::json!({"password": "secret"});
        let result: Result<LoginRequest, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_login_request_exactly_min_username_length() {
        // 3 chars = valid
        let req = LoginRequest {
            username: "abc".to_string(),
            password: "pass".to_string(),
        };
        assert!(req.validate().is_ok());

        // 2 chars = invalid
        let req_short = LoginRequest {
            username: "ab".to_string(),
            password: "pass".to_string(),
        };
        assert!(req_short.validate().is_err());
    }

    #[test]
    fn test_login_request_max_username_length() {
        let req_ok = LoginRequest {
            username: "a".repeat(100),
            password: "pass".to_string(),
        };
        assert!(req_ok.validate().is_ok());

        let req_over = LoginRequest {
            username: "a".repeat(101),
            password: "pass".to_string(),
        };
        assert!(req_over.validate().is_err());
    }

    // ── RegisterRequest serde / validation edge cases ────────────────────────

    #[test]
    fn test_register_request_serde_from_json() {
        let json = serde_json::json!({
            "username": "john_doe",
            "email": "john@example.com",
            "password": "Secure1Pass",
            "name": "John Doe",
            "phone": "+49123456789"
        });
        let req: RegisterRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.username, "john_doe");
        assert_eq!(req.phone.as_deref(), Some("+49123456789"));
    }

    #[test]
    fn test_register_request_no_phone_is_ok() {
        let json = serde_json::json!({
            "username": "newuser",
            "email": "new@example.com",
            "password": "StrongPass1",
            "name": "New User"
        });
        let req: RegisterRequest = serde_json::from_value(json).unwrap();
        assert!(req.phone.is_none());
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_register_request_phone_too_long_fails() {
        let req = RegisterRequest {
            username: "userxyz".to_string(),
            email: "u@example.com".to_string(),
            password: "StrongPass1".to_string(),
            name: "User".to_string(),
            phone: Some("0".repeat(21)),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_register_request_username_must_start_with_letter() {
        let req = RegisterRequest {
            username: "1invalid".to_string(),
            email: "a@example.com".to_string(),
            password: "StrongPass1".to_string(),
            name: "A".to_string(),
            phone: None,
        };
        assert!(req.validate().is_err());
    }

    // ── CreateBookingRequest serde ───────────────────────────────────────────

    #[test]
    fn test_create_booking_request_serde_from_json() {
        let lot_id = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let start = chrono::Utc::now() + chrono::Duration::hours(2);
        let json = serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "start_time": start,
            "duration_minutes": 90,
            "license_plate": "M-XY-1234"
        });
        let req: CreateBookingRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.lot_id, lot_id);
        assert_eq!(req.duration_minutes, 90);
        assert!(req.vehicle_id.is_none());
        assert!(req.notes.is_none());
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_booking_request_duration_14_minutes_fails() {
        let req = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: chrono::Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 14,
            vehicle_id: None,
            license_plate: Some("AB-CD-123".to_string()),
            notes: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_booking_request_duration_1441_minutes_fails() {
        let req = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: chrono::Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 1441,
            vehicle_id: None,
            license_plate: Some("AB-CD-123".to_string()),
            notes: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_booking_request_with_vehicle_id_no_plate() {
        let req = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: chrono::Utc::now() + chrono::Duration::hours(1),
            duration_minutes: 60,
            vehicle_id: Some(Uuid::new_v4()),
            license_plate: None,
            notes: Some("covered spot please".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    // ── VehicleRequest serde ─────────────────────────────────────────────────

    #[test]
    fn test_vehicle_request_serde_from_json() {
        let json = serde_json::json!({
            "license_plate": "MUC-AB-123",
            "make": "Audi",
            "model": "A4",
            "color": "Silver"
        });
        let req: VehicleRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.license_plate, "MUC-AB-123");
        assert_eq!(req.make.as_deref(), Some("Audi"));
        // is_default should default to false
        assert!(!req.is_default);
    }

    #[test]
    fn test_vehicle_request_make_too_long_fails() {
        let req = VehicleRequest {
            license_plate: "AB-12".to_string(),
            make: Some("A".repeat(51)),
            model: None,
            color: None,
            vehicle_type: None,
            is_default: false,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_vehicle_request_color_too_long_fails() {
        let req = VehicleRequest {
            license_plate: "AB-12".to_string(),
            make: None,
            model: None,
            color: Some("x".repeat(31)),
            vehicle_type: None,
            is_default: false,
        };
        assert!(req.validate().is_err());
    }

    // ── UpdateProfileRequest serde ───────────────────────────────────────────

    #[test]
    fn test_update_profile_request_all_none_is_ok() {
        let req = UpdateProfileRequest {
            name: None,
            email: None,
            phone: None,
            picture: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_profile_request_valid_url() {
        let req = UpdateProfileRequest {
            name: None,
            email: None,
            phone: None,
            picture: Some("https://example.com/avatar.jpg".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_profile_request_name_too_long_fails() {
        let req = UpdateProfileRequest {
            name: Some("x".repeat(101)),
            email: None,
            phone: None,
            picture: None,
        };
        assert!(req.validate().is_err());
    }

    // ── ExtendBookingRequest boundary ────────────────────────────────────────

    #[test]
    fn test_extend_booking_request_min_boundary() {
        let at_min = ExtendBookingRequest {
            additional_minutes: 15,
        };
        assert!(at_min.validate().is_ok());

        let below_min = ExtendBookingRequest {
            additional_minutes: 14,
        };
        assert!(below_min.validate().is_err());
    }

    #[test]
    fn test_extend_booking_request_max_boundary() {
        let at_max = ExtendBookingRequest {
            additional_minutes: 480,
        };
        assert!(at_max.validate().is_ok());

        let above_max = ExtendBookingRequest {
            additional_minutes: 481,
        };
        assert!(above_max.validate().is_err());
    }

    // ── UpdateBookingRequest serde ───────────────────────────────────────────

    #[test]
    fn test_update_booking_request_all_none_from_empty_json() {
        let json = serde_json::json!({});
        let req: UpdateBookingRequest = serde_json::from_value(json).unwrap();
        assert!(req.start_time.is_none());
        assert!(req.duration_minutes.is_none());
        assert!(req.notes.is_none());
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_booking_request_notes_exactly_500_chars() {
        let req = UpdateBookingRequest {
            start_time: None,
            duration_minutes: None,
            notes: Some("x".repeat(500)),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_booking_request_notes_501_chars_fails() {
        let req = UpdateBookingRequest {
            start_time: None,
            duration_minutes: None,
            notes: Some("x".repeat(501)),
        };
        assert!(req.validate().is_err());
    }

    // ── UpdatePreferencesRequest serde ───────────────────────────────────────

    #[test]
    fn test_update_preferences_language_too_short_fails() {
        let req = UpdatePreferencesRequest {
            default_duration_minutes: None,
            notifications_enabled: None,
            email_reminders: None,
            language: Some("x".to_string()), // 1 char, min is 2
            theme: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_preferences_theme_too_long_fails() {
        let req = UpdatePreferencesRequest {
            default_duration_minutes: None,
            notifications_enabled: None,
            email_reminders: None,
            language: None,
            theme: Some("x".repeat(11)), // max is 10
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_preferences_all_none_is_valid() {
        let req = UpdatePreferencesRequest {
            default_duration_minutes: None,
            notifications_enabled: None,
            email_reminders: None,
            language: None,
            theme: None,
        };
        assert!(req.validate().is_ok());
    }

    // ── PaginationParams serde defaults ─────────────────────────────────────

    #[test]
    fn test_pagination_params_serde_defaults() {
        let json = serde_json::json!({});
        let params: PaginationParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_pagination_params_per_page_over_100_fails() {
        let req = PaginationParams {
            page: 1,
            per_page: 101,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_pagination_params_page_zero_fails() {
        let req = PaginationParams {
            page: 0,
            per_page: 20,
        };
        assert!(req.validate().is_err());
    }
}
