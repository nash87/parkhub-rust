//! Lobby Display / Kiosk Mode — public endpoints for digital signage.
//!
//! `GET /api/v1/lots/:id/display` returns structured JSON for lobby monitors.
//! No authentication required. Rate-limited to 10 requests per minute per IP.
//! Feature flag: `mod-lobby-display`.

use axum::{extract::Path, extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use parkhub_common::ApiResponse;

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// Color status for occupancy indicator
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OccupancyColor {
    Green,
    Yellow,
    Red,
}

/// Per-floor availability for lobby display
#[derive(Debug, Serialize, Clone)]
pub struct FloorDisplay {
    pub floor_name: String,
    pub floor_number: i32,
    pub total_slots: i32,
    pub available_slots: i32,
    pub occupancy_percent: f64,
}

/// Response payload for the lobby display endpoint
#[derive(Debug, Serialize, Clone)]
pub struct LotDisplayData {
    pub lot_id: String,
    pub lot_name: String,
    pub total_slots: i32,
    pub available_slots: i32,
    pub occupancy_percent: f64,
    pub color_status: OccupancyColor,
    pub floors: Vec<FloorDisplay>,
    pub timestamp: String,
}

/// Determine the occupancy color: green <50%, yellow 50-80%, red >80%.
fn occupancy_color(occupancy_percent: f64) -> OccupancyColor {
    if occupancy_percent > 80.0 {
        OccupancyColor::Red
    } else if occupancy_percent >= 50.0 {
        OccupancyColor::Yellow
    } else {
        OccupancyColor::Green
    }
}

/// `GET /api/v1/lots/{id}/display` — public lobby display data for a single lot.
///
/// Returns lot name, total/available slots, occupancy percentage, color status,
/// and per-floor breakdown. No authentication required.
#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}/display",
    tag = "Public",
    summary = "Lobby display data for a parking lot",
    description = "Returns structured display data for digital signage / kiosk monitors. \
        No auth required. Rate-limited to 10 req/min per IP.",
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Lobby display data"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn lot_display(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<LotDisplayData>>) {
    let state_guard = state.read().await;

    let lot = match state_guard.db.get_parking_lot(&id).await {
        Ok(Some(lot)) => lot,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!(lot_id = %id, error = %e, "Failed to load lot for display");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Count active bookings per lot
    let now = Utc::now();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let active_bookings: Vec<_> = bookings
        .iter()
        .filter(|b| {
            b.lot_id == lot.id
                && b.start_time <= now
                && b.end_time >= now
                && matches!(
                    b.status,
                    parkhub_common::BookingStatus::Confirmed
                        | parkhub_common::BookingStatus::Active
                )
        })
        .collect();

    let occupied = i32::try_from(active_bookings.len()).unwrap_or(i32::MAX);
    let available = (lot.total_slots - occupied).max(0);
    let occupancy_pct = if lot.total_slots > 0 {
        (f64::from(occupied) / f64::from(lot.total_slots)) * 100.0
    } else {
        0.0
    };

    // Per-floor breakdown
    let floors: Vec<FloorDisplay> = lot
        .floors
        .iter()
        .map(|floor| {
            let floor_occupied = i32::try_from(
                active_bookings
                    .iter()
                    .filter(|b| {
                        // Match bookings to floor via slot -> floor mapping
                        floor
                            .slots
                            .iter()
                            .any(|s| s.id.to_string() == b.slot_id.to_string())
                    })
                    .count(),
            )
            .unwrap_or(i32::MAX);

            let floor_available = (floor.total_slots - floor_occupied).max(0);
            let floor_occ = if floor.total_slots > 0 {
                (f64::from(floor_occupied) / f64::from(floor.total_slots)) * 100.0
            } else {
                0.0
            };

            FloorDisplay {
                floor_name: floor.name.clone(),
                floor_number: floor.floor_number,
                total_slots: floor.total_slots,
                available_slots: floor_available,
                occupancy_percent: floor_occ,
            }
        })
        .collect();

    let data = LotDisplayData {
        lot_id: lot.id.to_string(),
        lot_name: lot.name.clone(),
        total_slots: lot.total_slots,
        available_slots: available,
        occupancy_percent: occupancy_pct,
        color_status: occupancy_color(occupancy_pct),
        floors,
        timestamp: now.to_rfc3339(),
    };

    (StatusCode::OK, Json(ApiResponse::success(data)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupancy_color_green() {
        assert_eq!(occupancy_color(0.0), OccupancyColor::Green);
        assert_eq!(occupancy_color(25.0), OccupancyColor::Green);
        assert_eq!(occupancy_color(49.9), OccupancyColor::Green);
    }

    #[test]
    fn test_occupancy_color_yellow() {
        assert_eq!(occupancy_color(50.0), OccupancyColor::Yellow);
        assert_eq!(occupancy_color(65.0), OccupancyColor::Yellow);
        assert_eq!(occupancy_color(80.0), OccupancyColor::Yellow);
    }

    #[test]
    fn test_occupancy_color_red() {
        assert_eq!(occupancy_color(80.1), OccupancyColor::Red);
        assert_eq!(occupancy_color(95.0), OccupancyColor::Red);
        assert_eq!(occupancy_color(100.0), OccupancyColor::Red);
    }

    #[test]
    fn test_lot_display_data_serialization() {
        let data = LotDisplayData {
            lot_id: "lot-1".to_string(),
            lot_name: "Test Garage".to_string(),
            total_slots: 100,
            available_slots: 30,
            occupancy_percent: 70.0,
            color_status: OccupancyColor::Yellow,
            floors: vec![FloorDisplay {
                floor_name: "Floor 1".to_string(),
                floor_number: 1,
                total_slots: 50,
                available_slots: 15,
                occupancy_percent: 70.0,
            }],
            timestamp: "2026-03-22T12:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["lot_name"], "Test Garage");
        assert_eq!(json["total_slots"], 100);
        assert_eq!(json["available_slots"], 30);
        assert_eq!(json["color_status"], "yellow");
        assert_eq!(json["floors"].as_array().unwrap().len(), 1);
        assert_eq!(json["floors"][0]["floor_name"], "Floor 1");
    }

    #[test]
    fn test_floor_display_serialization() {
        let floor = FloorDisplay {
            floor_name: "Basement".to_string(),
            floor_number: -1,
            total_slots: 20,
            available_slots: 5,
            occupancy_percent: 75.0,
        };

        let json = serde_json::to_value(&floor).unwrap();
        assert_eq!(json["floor_name"], "Basement");
        assert_eq!(json["floor_number"], -1);
        assert_eq!(json["total_slots"], 20);
        assert_eq!(json["available_slots"], 5);
    }

    #[test]
    fn test_occupancy_color_boundary_values() {
        // Exact boundaries
        assert_eq!(occupancy_color(0.0), OccupancyColor::Green);
        assert_eq!(occupancy_color(50.0), OccupancyColor::Yellow);
        assert_eq!(occupancy_color(80.0), OccupancyColor::Yellow);
        assert_eq!(occupancy_color(80.01), OccupancyColor::Red);
    }
}
