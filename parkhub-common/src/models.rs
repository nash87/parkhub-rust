//! Data Models
//!
//! All shared data structures for the `ParkHub` system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════════
// USER & AUTHENTICATION MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub picture: Option<String>,
    pub phone: Option<String>,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub preferences: UserPreferences,
    pub is_active: bool,
    #[serde(default)]
    pub credits_balance: i32,
    #[serde(default = "default_credits_quota")]
    pub credits_monthly_quota: i32,
    #[serde(default)]
    pub credits_last_refilled: Option<DateTime<Utc>>,
    /// Multi-tenant isolation: tenant ID (None = super-admin / global scope)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Accessibility needs: wheelchair, reduced_mobility, visual, hearing, none
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessibility_needs: Option<String>,
    /// Cost center for billing allocation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<String>,
    /// Department for billing allocation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
}

const fn default_credits_quota() -> i32 {
    40
}

/// User role for access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    #[default]
    User,
    Premium,
    Admin,
    SuperAdmin,
}

/// User preferences stored on server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub default_duration_minutes: Option<i32>,
    pub favorite_slots: Vec<String>,
    pub notifications_enabled: bool,
    pub email_reminders: bool,
    pub language: String,
    pub theme: String,
}

/// Authentication tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub token_type: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// PARKING LOT MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Parking lot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingLot {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub total_slots: i32,
    pub available_slots: i32,
    pub floors: Vec<ParkingFloor>,
    pub amenities: Vec<String>,
    pub pricing: PricingInfo,
    pub operating_hours: OperatingHours,
    pub images: Vec<String>,
    pub status: LotStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Multi-tenant isolation: tenant ID (None = global scope)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Parking floor within a lot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingFloor {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub name: String,
    pub floor_number: i32,
    pub total_slots: i32,
    pub available_slots: i32,
    pub slots: Vec<ParkingSlot>,
}

/// Individual parking slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingSlot {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub floor_id: Uuid,
    pub slot_number: i32,
    pub row: i32,
    pub column: i32,
    pub slot_type: SlotType,
    pub status: SlotStatus,
    pub current_booking: Option<SlotBookingInfo>,
    pub features: Vec<SlotFeature>,
    pub position: SlotPosition,
    /// Whether this slot is designated as accessible (wheelchair, reduced mobility)
    #[serde(default)]
    pub is_accessible: bool,
}

/// Slot type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SlotType {
    #[default]
    Standard,
    Compact,
    Large,
    Handicap,
    Electric,
    Motorcycle,
    Reserved,
    Vip,
}

/// Slot availability status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SlotStatus {
    #[default]
    Available,
    Occupied,
    Reserved,
    Maintenance,
    Disabled,
}

/// Brief booking info for slot display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotBookingInfo {
    pub booking_id: Uuid,
    pub user_id: Uuid,
    pub license_plate: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_own_booking: bool,
}

/// Additional slot features
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlotFeature {
    NearExit,
    NearElevator,
    NearStairs,
    Covered,
    SecurityCamera,
    WellLit,
    WideLane,
    ChargingStation,
}

/// Physical position in the lot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotPosition {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
}

/// Lot operational status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LotStatus {
    #[default]
    Open,
    Closed,
    Full,
    Maintenance,
}

// ═══════════════════════════════════════════════════════════════════════════════
// PRICING MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Pricing information for a lot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    pub currency: String,
    pub rates: Vec<PricingRate>,
    pub daily_max: Option<f64>,
    pub monthly_pass: Option<f64>,
}

/// Individual pricing rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRate {
    pub duration_minutes: i32,
    pub price: f64,
    pub label: String,
}

/// Operating hours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingHours {
    pub is_24h: bool,
    pub monday: Option<DayHours>,
    pub tuesday: Option<DayHours>,
    pub wednesday: Option<DayHours>,
    pub thursday: Option<DayHours>,
    pub friday: Option<DayHours>,
    pub saturday: Option<DayHours>,
    pub sunday: Option<DayHours>,
}

/// Hours for a specific day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayHours {
    pub open: String,
    pub close: String,
    #[serde(default)]
    pub closed: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// DYNAMIC PRICING MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Dynamic pricing rules for occupancy-based surge/discount pricing per lot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPricingRules {
    /// Whether dynamic pricing is enabled for this lot
    pub enabled: bool,
    /// Base hourly rate (before any multiplier)
    pub base_price: f64,
    /// Multiplier when occupancy exceeds `surge_threshold` (e.g. 1.5 = +50%)
    pub surge_multiplier: f64,
    /// Multiplier when occupancy is below `discount_threshold` (e.g. 0.8 = -20%)
    pub discount_multiplier: f64,
    /// Occupancy percentage at which surge pricing activates (0-100)
    pub surge_threshold: f64,
    /// Occupancy percentage below which discount pricing activates (0-100)
    pub discount_threshold: f64,
}

impl Default for DynamicPricingRules {
    fn default() -> Self {
        Self {
            enabled: false,
            base_price: 2.50,
            surge_multiplier: 1.5,
            discount_multiplier: 0.8,
            surge_threshold: 80.0,
            discount_threshold: 20.0,
        }
    }
}

/// Current dynamic price calculation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPriceResult {
    /// The computed hourly price after applying any multiplier
    pub current_price: f64,
    /// The base price before any multiplier
    pub base_price: f64,
    /// The multiplier that was applied (1.0 = normal, >1.0 = surge, <1.0 = discount)
    pub applied_multiplier: f64,
    /// Current occupancy percentage
    pub occupancy_percent: f64,
    /// Whether dynamic pricing is active
    pub dynamic_pricing_active: bool,
    /// Human-readable pricing tier: "surge", "discount", or "normal"
    pub tier: String,
    /// Currency (e.g. "EUR")
    pub currency: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// CREDITS MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Credit transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub booking_id: Option<Uuid>,
    pub amount: i32,
    pub transaction_type: CreditTransactionType,
    pub description: Option<String>,
    pub granted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Credit transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreditTransactionType {
    Grant,
    Deduction,
    Refund,
    MonthlyRefill,
    Adjustment,
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Full booking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Booking {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lot_id: Uuid,
    pub slot_id: Uuid,
    pub slot_number: i32,
    pub floor_name: String,
    pub vehicle: Vehicle,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: BookingStatus,
    pub pricing: BookingPricing,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub check_in_time: Option<DateTime<Utc>>,
    pub check_out_time: Option<DateTime<Utc>>,
    pub qr_code: Option<String>,
    pub notes: Option<String>,
    /// Multi-tenant isolation: tenant ID (None = global scope)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Booking status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BookingStatus {
    #[default]
    Pending,
    Confirmed,
    Active,
    Completed,
    Cancelled,
    Expired,
    NoShow,
}

/// Pricing details for a booking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingPricing {
    pub base_price: f64,
    pub discount: f64,
    pub tax: f64,
    pub total: f64,
    pub currency: String,
    pub payment_status: PaymentStatus,
    pub payment_method: Option<String>,
}

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    #[default]
    Pending,
    Paid,
    Failed,
    Refunded,
    PartialRefund,
}

/// Vehicle information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: Uuid,
    pub user_id: Uuid,
    pub license_plate: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub vehicle_type: VehicleType,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

/// Vehicle type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VehicleType {
    #[default]
    Car,
    Suv,
    Motorcycle,
    Bicycle,
    Truck,
    Van,
    Electric,
}

/// Request to create a booking
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateBookingRequest {
    pub lot_id: Uuid,
    pub slot_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: i32,
    pub vehicle_id: Uuid,
    pub license_plate: String,
    pub notes: Option<String>,
}

/// Request to extend a booking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendBookingRequest {
    pub additional_minutes: i32,
}

/// Booking history filters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookingFilters {
    pub status: Option<BookingStatus>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub lot_id: Option<Uuid>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATION MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// User notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    BookingConfirmed,
    BookingReminder,
    BookingExpiring,
    BookingCancelled,
    PaymentReceived,
    PaymentFailed,
    PromotionAvailable,
    SystemMessage,
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATISTICS MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatistics {
    pub total_bookings: i32,
    pub total_hours_parked: f64,
    pub total_spent: f64,
    pub currency: String,
    pub favorite_lot: Option<String>,
    pub favorite_slot: Option<i32>,
    pub average_duration_minutes: f64,
    pub bookings_this_month: i32,
    pub monthly_breakdown: Vec<MonthlyStats>,
}

/// Monthly statistics breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyStats {
    pub month: String,
    pub bookings: i32,
    pub hours: f64,
    pub spent: f64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// LAYOUT MODELS (for editor)
// ═══════════════════════════════════════════════════════════════════════════════

/// Parking lot layout for visual editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingLayout {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub floor_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub width: f32,
    pub height: f32,
    pub elements: Vec<LayoutElement>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Layout element (slot, road, obstacle, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutElement {
    pub id: Uuid,
    pub element_type: LayoutElementType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub slot_number: Option<i32>,
    pub slot_id: Option<Uuid>,
    pub label: Option<String>,
}

/// Type of layout element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutElementType {
    ParkingSlot,
    Road,
    Entrance,
    Exit,
    Elevator,
    Stairs,
    Wall,
    Pillar,
    Obstacle,
    ChargingStation,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ABSENCE & HOMEOFFICE MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Absence type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AbsenceType {
    Homeoffice,
    Vacation,
    Sick,
    Training,
    Other,
}

/// Absence record (homeoffice, vacation, sick, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Absence {
    pub id: Uuid,
    pub user_id: Uuid,
    pub absence_type: AbsenceType,
    pub start_date: String,
    pub end_date: String,
    pub note: Option<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

/// Recurring absence pattern (e.g. homeoffice every Monday)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AbsencePattern {
    pub absence_type: AbsenceType,
    pub weekdays: Vec<u8>,
}

/// Waitlist entry for a parking lot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lot_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub notified_at: Option<DateTime<Utc>>,
}

/// Guest booking (visitor parking)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GuestBooking {
    pub id: Uuid,
    pub created_by: Uuid,
    pub lot_id: Uuid,
    pub slot_id: Uuid,
    pub guest_name: String,
    pub guest_email: Option<String>,
    pub guest_code: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub vehicle_plate: Option<String>,
    pub status: BookingStatus,
    pub created_at: DateTime<Utc>,
}

/// Swap request between two bookings
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SwapRequest {
    pub id: Uuid,
    pub requester_booking_id: Uuid,
    pub target_booking_id: Uuid,
    pub requester_id: Uuid,
    pub target_id: Uuid,
    pub status: SwapRequestStatus,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Swap request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapRequestStatus {
    Pending,
    Accepted,
    Declined,
    Cancelled,
}

/// Recurring booking pattern
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecurringBooking {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lot_id: Uuid,
    pub slot_id: Option<Uuid>,
    pub days_of_week: Vec<u8>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub vehicle_plate: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

/// System announcement
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Announcement {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub severity: AnnouncementSeverity,
    pub active: bool,
    pub created_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Announcement severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementSeverity {
    Info,
    Warning,
    Error,
    Success,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TRANSLATION MANAGEMENT MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Status of a translation proposal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
}

/// A community-submitted translation proposal
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TranslationProposal {
    pub id: Uuid,
    pub language: String,
    pub key: String,
    pub current_value: String,
    pub proposed_value: String,
    pub context: Option<String>,
    pub proposed_by: Uuid,
    pub proposed_by_name: String,
    pub status: ProposalStatus,
    pub votes_for: i32,
    pub votes_against: i32,
    pub reviewer_id: Option<Uuid>,
    pub reviewer_name: Option<String>,
    pub review_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A vote on a translation proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationVote {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub user_id: Uuid,
    pub vote: String, // "up" or "down"
    pub created_at: DateTime<Utc>,
}

/// An approved translation override (applied at runtime)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TranslationOverride {
    pub language: String,
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VISITOR PRE-REGISTRATION MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Visitor registration status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VisitorStatus {
    Pending,
    CheckedIn,
    Expired,
    Cancelled,
}

/// Pre-registered visitor
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Visitor {
    pub id: Uuid,
    pub host_user_id: Uuid,
    pub name: String,
    pub email: String,
    pub vehicle_plate: Option<String>,
    pub visit_date: DateTime<Utc>,
    pub purpose: Option<String>,
    pub status: VisitorStatus,
    pub qr_code: Option<String>,
    pub pass_url: Option<String>,
    pub checked_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_default() {
        let role = UserRole::default();
        assert_eq!(role, UserRole::User);
    }

    #[test]
    fn test_user_role_serialization() {
        assert_eq!(serde_json::to_string(&UserRole::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&UserRole::Premium).unwrap(),
            "\"premium\""
        );
        assert_eq!(
            serde_json::to_string(&UserRole::Admin).unwrap(),
            "\"admin\""
        );
        assert_eq!(
            serde_json::to_string(&UserRole::SuperAdmin).unwrap(),
            "\"superadmin\""
        );
    }

    #[test]
    fn test_user_role_deserialization() {
        assert_eq!(
            serde_json::from_str::<UserRole>("\"user\"").unwrap(),
            UserRole::User
        );
        assert_eq!(
            serde_json::from_str::<UserRole>("\"admin\"").unwrap(),
            UserRole::Admin
        );
    }

    #[test]
    fn test_slot_status_default() {
        let status = SlotStatus::default();
        assert_eq!(status, SlotStatus::Available);
    }

    #[test]
    fn test_slot_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SlotStatus::Available).unwrap(),
            "\"available\""
        );
        assert_eq!(
            serde_json::to_string(&SlotStatus::Occupied).unwrap(),
            "\"occupied\""
        );
        assert_eq!(
            serde_json::to_string(&SlotStatus::Maintenance).unwrap(),
            "\"maintenance\""
        );
    }

    #[test]
    fn test_slot_type_default() {
        let slot_type = SlotType::default();
        assert_eq!(slot_type, SlotType::Standard);
    }

    #[test]
    fn test_slot_type_serialization() {
        assert_eq!(
            serde_json::to_string(&SlotType::Standard).unwrap(),
            "\"standard\""
        );
        assert_eq!(
            serde_json::to_string(&SlotType::Handicap).unwrap(),
            "\"handicap\""
        );
        assert_eq!(
            serde_json::to_string(&SlotType::Electric).unwrap(),
            "\"electric\""
        );
    }

    #[test]
    fn test_booking_status_default() {
        let status = BookingStatus::default();
        assert_eq!(status, BookingStatus::Pending);
    }

    #[test]
    fn test_booking_status_serialization() {
        assert_eq!(
            serde_json::to_string(&BookingStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&BookingStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&BookingStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&BookingStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn test_payment_status_default() {
        let status = PaymentStatus::default();
        assert_eq!(status, PaymentStatus::Pending);
    }

    #[test]
    fn test_vehicle_type_default() {
        let vehicle_type = VehicleType::default();
        assert_eq!(vehicle_type, VehicleType::Car);
    }

    #[test]
    fn test_lot_status_default() {
        let status = LotStatus::default();
        assert_eq!(status, LotStatus::Open);
    }

    #[test]
    fn test_user_preferences_default() {
        let prefs = UserPreferences::default();
        assert!(!prefs.notifications_enabled);
        assert!(!prefs.email_reminders);
        assert!(prefs.favorite_slots.is_empty());
        assert_eq!(prefs.language, "");
        assert_eq!(prefs.theme, "");
    }

    #[test]
    fn test_slot_feature_serialization() {
        assert_eq!(
            serde_json::to_string(&SlotFeature::NearExit).unwrap(),
            "\"near_exit\""
        );
        assert_eq!(
            serde_json::to_string(&SlotFeature::ChargingStation).unwrap(),
            "\"charging_station\""
        );
    }

    #[test]
    fn test_layout_element_type_serialization() {
        assert_eq!(
            serde_json::to_string(&LayoutElementType::ParkingSlot).unwrap(),
            "\"parking_slot\""
        );
        assert_eq!(
            serde_json::to_string(&LayoutElementType::Entrance).unwrap(),
            "\"entrance\""
        );
        assert_eq!(
            serde_json::to_string(&LayoutElementType::ChargingStation).unwrap(),
            "\"charging_station\""
        );
    }

    #[test]
    fn test_booking_filters_default() {
        let filters = BookingFilters::default();
        assert!(filters.status.is_none());
        assert!(filters.from_date.is_none());
        assert!(filters.to_date.is_none());
        assert!(filters.lot_id.is_none());
        assert!(filters.page.is_none());
        assert!(filters.per_page.is_none());
    }

    #[test]
    fn test_create_booking_request_serialization() {
        let request = CreateBookingRequest {
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            start_time: Utc::now(),
            duration_minutes: 60,
            vehicle_id: Uuid::new_v4(),
            license_plate: "ABC-123".to_string(),
            notes: Some("Test booking".to_string()),
        };

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        let deserialized: CreateBookingRequest =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(request.lot_id, deserialized.lot_id);
        assert_eq!(request.duration_minutes, deserialized.duration_minutes);
        assert_eq!(request.license_plate, deserialized.license_plate);
        assert_eq!(request.notes, deserialized.notes);
    }
}
