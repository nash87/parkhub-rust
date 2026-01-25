//! API Endpoints
//!
//! Defines all API endpoints for the parking system.

/// API version prefix
pub const API_VERSION: &str = "v1";

/// Base API paths
pub mod paths {
    use super::API_VERSION;

    // Authentication endpoints
    pub fn auth_login() -> String {
        format!("/api/{}/auth/login", API_VERSION)
    }

    pub fn auth_logout() -> String {
        format!("/api/{}/auth/logout", API_VERSION)
    }

    pub fn auth_refresh() -> String {
        format!("/api/{}/auth/refresh", API_VERSION)
    }

    pub fn auth_me() -> String {
        format!("/api/{}/auth/me", API_VERSION)
    }

    // User endpoints
    pub fn user_profile() -> String {
        format!("/api/{}/user/profile", API_VERSION)
    }

    pub fn user_preferences() -> String {
        format!("/api/{}/user/preferences", API_VERSION)
    }

    pub fn user_vehicles() -> String {
        format!("/api/{}/user/vehicles", API_VERSION)
    }

    pub fn user_vehicle(id: &str) -> String {
        format!("/api/{}/user/vehicles/{}", API_VERSION, id)
    }

    pub fn user_statistics() -> String {
        format!("/api/{}/user/statistics", API_VERSION)
    }

    // Parking lot endpoints
    pub fn lots() -> String {
        format!("/api/{}/lots", API_VERSION)
    }

    pub fn lot(id: &str) -> String {
        format!("/api/{}/lots/{}", API_VERSION, id)
    }

    pub fn lot_floors(lot_id: &str) -> String {
        format!("/api/{}/lots/{}/floors", API_VERSION, lot_id)
    }

    pub fn lot_slots(lot_id: &str) -> String {
        format!("/api/{}/lots/{}/slots", API_VERSION, lot_id)
    }

    pub fn lot_slots_by_floor(lot_id: &str, floor_id: &str) -> String {
        format!(
            "/api/{}/lots/{}/floors/{}/slots",
            API_VERSION, lot_id, floor_id
        )
    }

    pub fn lot_availability(lot_id: &str) -> String {
        format!("/api/{}/lots/{}/availability", API_VERSION, lot_id)
    }

    pub fn lot_pricing(lot_id: &str) -> String {
        format!("/api/{}/lots/{}/pricing", API_VERSION, lot_id)
    }

    // Booking endpoints
    pub fn bookings() -> String {
        format!("/api/{}/bookings", API_VERSION)
    }

    pub fn booking(id: &str) -> String {
        format!("/api/{}/bookings/{}", API_VERSION, id)
    }

    pub fn booking_extend(id: &str) -> String {
        format!("/api/{}/bookings/{}/extend", API_VERSION, id)
    }

    pub fn booking_cancel(id: &str) -> String {
        format!("/api/{}/bookings/{}/cancel", API_VERSION, id)
    }

    pub fn booking_checkin(id: &str) -> String {
        format!("/api/{}/bookings/{}/checkin", API_VERSION, id)
    }

    pub fn booking_checkout(id: &str) -> String {
        format!("/api/{}/bookings/{}/checkout", API_VERSION, id)
    }

    pub fn booking_qrcode(id: &str) -> String {
        format!("/api/{}/bookings/{}/qrcode", API_VERSION, id)
    }

    pub fn active_bookings() -> String {
        format!("/api/{}/bookings/active", API_VERSION)
    }

    pub fn booking_history() -> String {
        format!("/api/{}/bookings/history", API_VERSION)
    }

    // Payment endpoints
    pub fn payments() -> String {
        format!("/api/{}/payments", API_VERSION)
    }

    pub fn payment(id: &str) -> String {
        format!("/api/{}/payments/{}", API_VERSION, id)
    }

    pub fn payment_methods() -> String {
        format!("/api/{}/payments/methods", API_VERSION)
    }

    pub fn payment_intent() -> String {
        format!("/api/{}/payments/intent", API_VERSION)
    }

    // Notification endpoints
    pub fn notifications() -> String {
        format!("/api/{}/notifications", API_VERSION)
    }

    pub fn notification_read(id: &str) -> String {
        format!("/api/{}/notifications/{}/read", API_VERSION, id)
    }

    pub fn notifications_read_all() -> String {
        format!("/api/{}/notifications/read-all", API_VERSION)
    }

    // WebSocket endpoint
    pub fn websocket() -> String {
        format!("/ws/{}", API_VERSION)
    }

    // Health check
    pub fn health() -> String {
        "/health".to_string()
    }
}

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}
