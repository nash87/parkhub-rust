//! API Data Models
//!
//! All data structures for communication with the parking backend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// USER & AUTHENTICATION MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub phone: Option<String>,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub preferences: UserPreferences,
}

/// User role for access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
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

/// Login request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub provider: String, // "google", "email", etc.
    pub token: String,    // OAuth token or email token
}

/// Login response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub tokens: AuthTokens,
}

// ═══════════════════════════════════════════════════════════════════════════════
// PARKING LOT MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Parking lot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingLot {
    pub id: String,
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
}

/// Parking floor within a lot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingFloor {
    pub id: String,
    pub name: String,      // e.g., "UG-1", "Level 1"
    pub floor_number: i32, // -1 for underground, 0 for ground, etc.
    pub total_slots: i32,
    pub available_slots: i32,
    pub slots: Vec<ParkingSlot>,
}

/// Individual parking slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingSlot {
    pub id: String,
    pub slot_number: i32,
    pub floor_id: String,
    pub row: i32,
    pub column: i32,
    pub slot_type: SlotType,
    pub status: SlotStatus,
    pub current_booking: Option<SlotBookingInfo>,
    pub features: Vec<SlotFeature>,
    pub position: SlotPosition,
}

/// Slot type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
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
    pub booking_id: String,
    pub user_id: String,
    pub license_plate: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_own_booking: bool,
}

/// Additional slot features
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
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
    pub open: String,  // "08:00"
    pub close: String, // "22:00"
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// Full booking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Booking {
    pub id: String,
    pub user_id: String,
    pub lot_id: String,
    pub slot_id: String,
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
}

/// Booking status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
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
    pub id: Option<String>,
    pub license_plate: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub vehicle_type: VehicleType,
}

/// Vehicle type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum VehicleType {
    #[default]
    Car,
    Suv,
    Motorcycle,
    Truck,
    Van,
    Electric,
}

/// Request to create a booking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookingRequest {
    pub lot_id: String,
    pub slot_id: String,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: i32,
    pub vehicle: Vehicle,
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
    pub lot_id: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATION MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// User notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub month: String, // "2024-01"
    pub bookings: i32,
    pub hours: f64,
    pub spent: f64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE WRAPPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiErrorResponse>,
    pub meta: Option<ResponseMeta>,
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub total: Option<i32>,
    pub total_pages: Option<i32>,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: i32,
    pub per_page: i32,
    pub total: i32,
    pub total_pages: i32,
}

// ═══════════════════════════════════════════════════════════════════════════════
// REAL-TIME UPDATES
// ═══════════════════════════════════════════════════════════════════════════════

/// Real-time slot update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotUpdateEvent {
    pub lot_id: String,
    pub slot_id: String,
    pub slot_number: i32,
    pub floor_id: String,
    pub previous_status: SlotStatus,
    pub new_status: SlotStatus,
    pub booking_info: Option<SlotBookingInfo>,
    pub timestamp: DateTime<Utc>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    #[serde(rename = "slot_update")]
    SlotUpdate(SlotUpdateEvent),
    #[serde(rename = "booking_update")]
    BookingUpdate(Booking),
    #[serde(rename = "notification")]
    Notification(Notification),
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}
