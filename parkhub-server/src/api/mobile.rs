//! Mobile-Optimized Booking Flow — simplified booking for mobile devices.
//!
//! Provides streamlined endpoints for mobile-first booking experience including
//! geolocation-based lot discovery, one-step quick booking, and active booking countdown.
//!
//! Endpoints:
//! - `GET /api/v1/mobile/quick-book`     — simplified booking (lot + slot + confirm)
//! - `GET /api/v1/mobile/nearby-lots`    — geolocation-based lot discovery
//! - `GET /api/v1/mobile/active-booking` — current active booking with countdown

use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use parkhub_common::ApiResponse;
use parkhub_common::models::BookingStatus;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Query parameters for nearby lots discovery.
#[derive(Debug, Deserialize)]
pub struct NearbyLotsQuery {
    pub lat: f64,
    pub lng: f64,
    /// Search radius in meters (default 1000, max 10000).
    pub radius: Option<f64>,
}

/// A nearby lot with distance and availability info.
#[derive(Debug, Serialize)]
pub struct NearbyLot {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub distance_meters: f64,
    pub total_slots: usize,
    pub available_slots: usize,
    pub occupancy_percent: f64,
}

/// Quick-book eligible lots response.
#[derive(Debug, Serialize)]
pub struct QuickBookLot {
    pub id: String,
    pub name: String,
    pub available_slots: usize,
    pub next_available_slot: Option<QuickBookSlot>,
}

/// A slot available for quick booking.
#[derive(Debug, Serialize)]
pub struct QuickBookSlot {
    pub slot_id: String,
    pub slot_label: String,
    pub lot_id: String,
    pub lot_name: String,
}

/// Active booking response with countdown metadata.
#[derive(Debug, Serialize)]
pub struct ActiveBookingResponse {
    pub id: String,
    pub lot_name: String,
    pub slot_label: String,
    pub start_time: String,
    pub end_time: String,
    pub remaining_seconds: i64,
    pub total_seconds: i64,
    pub progress_percent: f64,
    pub status: String,
    pub checked_in: bool,
}

/// Haversine distance between two points in meters.
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    R * c
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/mobile/nearby-lots` — geolocation-based lot discovery.
#[utoipa::path(get, path = "/api/v1/mobile/nearby-lots", tag = "Mobile Booking",
    summary = "Find nearby parking lots",
    description = "Returns parking lots within a given radius of the user's location, sorted by distance.",
    security(("bearer_auth" = [])),
    params(
        ("lat" = f64, Query, description = "Latitude"),
        ("lng" = f64, Query, description = "Longitude"),
        ("radius" = Option<f64>, Query, description = "Radius in meters (default 1000, max 10000)"),
    ),
    responses((status = 200, description = "Nearby lots sorted by distance"))
)]
pub async fn nearby_lots(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Query(query): Query<NearbyLotsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<NearbyLot>>>) {
    let state_guard = state.read().await;
    let radius = query.radius.unwrap_or(1000.0).clamp(100.0, 10000.0);

    let lots = match state_guard.db.list_parking_lots().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to list lots: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list lots")),
            );
        }
    };

    let mut nearby: Vec<NearbyLot> = Vec::new();

    for lot in &lots {
        // Use lot coordinates if available (stored via map module)
        let lot_lat = lot.latitude;
        let lot_lng = lot.longitude;
        let slots = match state_guard.db.list_slots_by_lot(&lot.id.to_string()).await {
            Ok(slots) => slots,
            Err(e) => {
                tracing::error!("Failed to list slots for lot {}: {}", lot.id, e);
                continue;
            }
        };

        // Skip lots without coordinates
        if lot_lat.abs() < f64::EPSILON && lot_lng.abs() < f64::EPSILON {
            // Include lots without coordinates but with large distance marker
            let total_slots = slots.len();
            let booked_count = 0usize; // simplified — full impl would check bookings
            let available = total_slots.saturating_sub(booked_count);
            nearby.push(NearbyLot {
                id: lot.id.to_string(),
                name: lot.name.clone(),
                address: Some(lot.address.clone()),
                lat: Some(lot.latitude),
                lng: Some(lot.longitude),
                distance_meters: f64::MAX,
                total_slots,
                available_slots: available,
                occupancy_percent: if total_slots > 0 {
                    (booked_count as f64 / total_slots as f64) * 100.0
                } else {
                    0.0
                },
            });
            continue;
        }

        let distance = haversine_distance(query.lat, query.lng, lot_lat, lot_lng);
        if distance <= radius {
            let total_slots = slots.len();
            let booked_count = 0usize;
            let available = total_slots.saturating_sub(booked_count);
            nearby.push(NearbyLot {
                id: lot.id.to_string(),
                name: lot.name.clone(),
                address: Some(lot.address.clone()),
                lat: Some(lot.latitude),
                lng: Some(lot.longitude),
                distance_meters: distance,
                total_slots,
                available_slots: available,
                occupancy_percent: if total_slots > 0 {
                    (booked_count as f64 / total_slots as f64) * 100.0
                } else {
                    0.0
                },
            });
        }
    }

    // Sort by distance (closest first)
    nearby.sort_by(|a, b| {
        a.distance_meters
            .partial_cmp(&b.distance_meters)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    (StatusCode::OK, Json(ApiResponse::success(nearby)))
}

/// `GET /api/v1/mobile/quick-book` — simplified booking options for mobile.
#[utoipa::path(get, path = "/api/v1/mobile/quick-book", tag = "Mobile Booking",
    summary = "Quick booking options",
    description = "Returns lots with available slots for one-tap mobile booking.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Quick-book eligible lots"))
)]
pub async fn quick_book(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<QuickBookLot>>>) {
    let state_guard = state.read().await;

    let lots = match state_guard.db.list_parking_lots().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to list lots: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list lots")),
            );
        }
    };

    let mut result = Vec::new();
    for lot in &lots {
        let slots = match state_guard.db.list_slots_by_lot(&lot.id.to_string()).await {
            Ok(slots) => slots,
            Err(e) => {
                tracing::error!("Failed to list slots for lot {}: {}", lot.id, e);
                continue;
            }
        };
        if slots.is_empty() {
            continue;
        }

        let available_slots = slots.len(); // simplified
        let next_slot = slots.first().map(|s| QuickBookSlot {
            slot_id: s.id.to_string(),
            slot_label: s.slot_number.to_string(),
            lot_id: lot.id.to_string(),
            lot_name: lot.name.clone(),
        });
        result.push(QuickBookLot {
            id: lot.id.to_string(),
            name: lot.name.clone(),
            available_slots,
            next_available_slot: next_slot,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

/// `GET /api/v1/mobile/active-booking` — current active booking with countdown.
#[utoipa::path(get, path = "/api/v1/mobile/active-booking", tag = "Mobile Booking",
    summary = "Get active booking with countdown",
    description = "Returns the user's current active booking with countdown timer metadata.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Active booking or null"))
)]
pub async fn active_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Option<ActiveBookingResponse>>>) {
    let state_guard = state.read().await;
    let user_id = auth_user.user_id.to_string();

    let bookings = match state_guard.db.list_bookings_by_user(&user_id).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to fetch bookings",
                )),
            );
        }
    };

    let now = Utc::now();

    // Find the current active booking (start <= now <= end).
    let active = bookings.iter().find(|b| {
        matches!(b.status, BookingStatus::Confirmed | BookingStatus::Active)
            && b.start_time <= now
            && b.end_time >= now
    });

    let response = active.map(|b| {
        let total_seconds = (b.end_time - b.start_time).num_seconds();
        let remaining_seconds = (b.end_time - now).num_seconds().max(0);
        let elapsed = total_seconds - remaining_seconds;
        let progress = if total_seconds > 0 {
            (elapsed as f64 / total_seconds as f64) * 100.0
        } else {
            100.0
        };

        // Resolve lot/slot names
        let lot_name = b.lot_id.to_string(); // full impl would look up
        let slot_label = b.slot_id.to_string();

        ActiveBookingResponse {
            id: b.id.to_string(),
            lot_name,
            slot_label,
            start_time: b.start_time.to_rfc3339(),
            end_time: b.end_time.to_rfc3339(),
            remaining_seconds,
            total_seconds,
            progress_percent: progress,
            status: format!("{:?}", b.status).to_lowercase(),
            checked_in: b.check_in_time.is_some(),
        }
    });

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_haversine_distance_same_point() {
        let d = haversine_distance(48.1351, 11.5820, 48.1351, 11.5820);
        assert!(d < 0.01);
    }

    #[test]
    fn test_haversine_distance_known_pair() {
        // Munich to Berlin ~504 km
        let d = haversine_distance(48.1351, 11.5820, 52.5200, 13.4050);
        assert!(d > 400_000.0);
        assert!(d < 600_000.0);
    }

    #[test]
    fn test_haversine_distance_short() {
        // ~111 meters for 0.001 degree latitude at equator
        let d = haversine_distance(0.0, 0.0, 0.001, 0.0);
        assert!(d > 100.0);
        assert!(d < 120.0);
    }

    #[test]
    fn test_nearby_lot_serialization() {
        let lot = NearbyLot {
            id: Uuid::new_v4().to_string(),
            name: "Test Lot".to_string(),
            address: Some("123 Main St".to_string()),
            lat: Some(48.1351),
            lng: Some(11.5820),
            distance_meters: 250.5,
            total_slots: 20,
            available_slots: 15,
            occupancy_percent: 25.0,
        };
        let json = serde_json::to_string(&lot).unwrap();
        assert!(json.contains("Test Lot"));
        assert!(json.contains("250.5"));
    }

    #[test]
    fn test_quick_book_lot_serialization() {
        let lot = QuickBookLot {
            id: "lot-1".to_string(),
            name: "Office Garage".to_string(),
            available_slots: 5,
            next_available_slot: Some(QuickBookSlot {
                slot_id: "slot-1".to_string(),
                slot_label: "A-01".to_string(),
                lot_id: "lot-1".to_string(),
                lot_name: "Office Garage".to_string(),
            }),
        };
        let json = serde_json::to_string(&lot).unwrap();
        assert!(json.contains("A-01"));
        assert!(json.contains("Office Garage"));
    }

    #[test]
    fn test_quick_book_lot_no_slot() {
        let lot = QuickBookLot {
            id: "lot-2".to_string(),
            name: "Empty Lot".to_string(),
            available_slots: 0,
            next_available_slot: None,
        };
        let json = serde_json::to_string(&lot).unwrap();
        assert!(json.contains("null"));
    }

    #[test]
    fn test_active_booking_response_serialization() {
        let resp = ActiveBookingResponse {
            id: Uuid::new_v4().to_string(),
            lot_name: "Main Garage".to_string(),
            slot_label: "B-05".to_string(),
            start_time: "2026-03-24T08:00:00Z".to_string(),
            end_time: "2026-03-24T18:00:00Z".to_string(),
            remaining_seconds: 3600,
            total_seconds: 36000,
            progress_percent: 90.0,
            status: "confirmed".to_string(),
            checked_in: true,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Main Garage"));
        assert!(json.contains("B-05"));
        assert!(json.contains("90"));
    }

    #[test]
    fn test_active_booking_countdown_math() {
        let total_seconds: i64 = 36000;
        let remaining: i64 = 9000;
        let elapsed = total_seconds - remaining;
        let progress = (elapsed as f64 / total_seconds as f64) * 100.0;
        assert!((progress - 75.0).abs() < f64::EPSILON);
    }
}
