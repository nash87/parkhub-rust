//! Map view handlers: lot markers with coordinates and live availability.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use parkhub_common::{ApiResponse, LotStatus, UserRole};

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Response types
// ─────────────────────────────────────────────────────────────────────────────

/// Color-coded availability status for map markers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarkerColor {
    /// > 50% available
    Green,
    /// 10–50% available
    Yellow,
    /// < 10% available (or closed/full)
    Red,
    /// Lot is closed or under maintenance
    Gray,
}

/// A parking lot marker for the map view.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LotMarker {
    pub id: String,
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub available_slots: i32,
    pub total_slots: i32,
    /// Lot status as string (open, closed, full, maintenance)
    pub status: String,
    pub color: MarkerColor,
}

/// Request body for setting a lot's geographic coordinates.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetLocationRequest {
    pub latitude: f64,
    pub longitude: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Derive marker color from availability percentage and lot status.
pub fn marker_color(status: &LotStatus, available: i32, total: i32) -> MarkerColor {
    if matches!(
        status,
        LotStatus::Closed | LotStatus::Maintenance | LotStatus::Full
    ) {
        if matches!(status, LotStatus::Full) {
            return MarkerColor::Red;
        }
        return MarkerColor::Gray;
    }
    if total == 0 {
        return MarkerColor::Red;
    }
    let pct = f64::from(available) / f64::from(total) * 100.0;
    if pct > 50.0 {
        MarkerColor::Green
    } else if pct >= 10.0 {
        MarkerColor::Yellow
    } else {
        MarkerColor::Red
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/map` — list all lots with coordinates for the map view.
///
/// Returns lots that have non-zero coordinates set. Includes live availability
/// and a color-coded marker status (green/yellow/red/gray).
#[utoipa::path(
    get,
    path = "/api/v1/lots/map",
    tag = "Map",
    summary = "List lots for map view",
    description = "Returns all parking lots with coordinates, availability, and color-coded markers.",
    responses(
        (status = 200, description = "List of lot markers"),
    )
)]
pub async fn list_lot_markers(
    State(state): State<SharedState>,
) -> Json<ApiResponse<Vec<LotMarker>>> {
    let state_guard = state.read().await;

    match state_guard.db.list_parking_lots().await {
        Ok(lots) => {
            let markers: Vec<LotMarker> = lots
                .iter()
                .filter(|lot| lot.latitude != 0.0 || lot.longitude != 0.0)
                .map(|lot| {
                    let color = marker_color(&lot.status, lot.available_slots, lot.total_slots);
                    LotMarker {
                        id: lot.id.to_string(),
                        name: lot.name.clone(),
                        address: lot.address.clone(),
                        latitude: lot.latitude,
                        longitude: lot.longitude,
                        available_slots: lot.available_slots,
                        total_slots: lot.total_slots,
                        status: serde_json::to_value(&lot.status)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_else(|| "open".to_string()),
                        color,
                    }
                })
                .collect();
            Json(ApiResponse::success(markers))
        }
        Err(e) => {
            tracing::error!("Failed to list lots for map: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to load map data",
            ))
        }
    }
}

/// `PUT /api/v1/admin/lots/{id}/location` — set a lot's geographic coordinates.
///
/// Admin-only. Sets the latitude/longitude for a parking lot so it appears on the map.
#[utoipa::path(
    put,
    path = "/api/v1/admin/lots/{id}/location",
    tag = "Map",
    summary = "Set lot coordinates",
    description = "Admin endpoint to set latitude/longitude for a parking lot.",
    request_body = SetLocationRequest,
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Lot UUID")),
    responses(
        (status = 200, description = "Location updated"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Lot not found"),
    )
)]
pub async fn set_lot_location(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<SetLocationRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Check admin
    let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };
    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    // Validate coordinates
    if !(-90.0..=90.0).contains(&req.latitude) || !(-180.0..=180.0).contains(&req.longitude) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "Latitude must be -90..90, longitude must be -180..180",
            )),
        );
    }

    // Fetch lot, update coordinates, save
    match state_guard.db.get_parking_lot(&id).await {
        Ok(Some(mut lot)) => {
            lot.latitude = req.latitude;
            lot.longitude = req.longitude;
            match state_guard.db.save_parking_lot(&lot).await {
                Ok(()) => (StatusCode::OK, Json(ApiResponse::success(()))),
                Err(e) => {
                    tracing::error!("Failed to update lot location: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
                    )
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_color_green() {
        assert_eq!(marker_color(&LotStatus::Open, 60, 100), MarkerColor::Green);
    }

    #[test]
    fn test_marker_color_yellow() {
        assert_eq!(marker_color(&LotStatus::Open, 30, 100), MarkerColor::Yellow);
    }

    #[test]
    fn test_marker_color_red() {
        assert_eq!(marker_color(&LotStatus::Open, 5, 100), MarkerColor::Red);
    }

    #[test]
    fn test_marker_color_gray_closed() {
        assert_eq!(marker_color(&LotStatus::Closed, 50, 100), MarkerColor::Gray);
    }

    #[test]
    fn test_marker_color_gray_maintenance() {
        assert_eq!(
            marker_color(&LotStatus::Maintenance, 50, 100),
            MarkerColor::Gray
        );
    }

    #[test]
    fn test_marker_color_full() {
        assert_eq!(marker_color(&LotStatus::Full, 0, 100), MarkerColor::Red);
    }

    #[test]
    fn test_marker_color_zero_total() {
        assert_eq!(marker_color(&LotStatus::Open, 0, 0), MarkerColor::Red);
    }

    #[test]
    fn test_marker_color_boundary_50_percent() {
        // Exactly 50% = yellow (not green)
        assert_eq!(marker_color(&LotStatus::Open, 50, 100), MarkerColor::Yellow);
    }

    #[test]
    fn test_marker_color_boundary_10_percent() {
        // Exactly 10% = yellow (not red)
        assert_eq!(marker_color(&LotStatus::Open, 10, 100), MarkerColor::Yellow);
    }

    #[test]
    fn test_set_location_request_deserialize() {
        let json = r#"{"latitude": 48.1351, "longitude": 11.5820}"#;
        let req: SetLocationRequest = serde_json::from_str(json).unwrap();
        assert!((req.latitude - 48.1351).abs() < 0.0001);
        assert!((req.longitude - 11.582).abs() < 0.0001);
    }

    #[test]
    fn test_lot_marker_serialize() {
        let marker = LotMarker {
            id: "lot-1".to_string(),
            name: "Central Parking".to_string(),
            address: "123 Main St".to_string(),
            latitude: 48.1351,
            longitude: 11.582,
            available_slots: 42,
            total_slots: 100,
            status: "open".to_string(),
            color: MarkerColor::Yellow,
        };
        let json = serde_json::to_string(&marker).unwrap();
        assert!(json.contains("Central Parking"));
        assert!(json.contains("48.1351"));
        assert!(json.contains("\"yellow\""));
    }

    #[test]
    fn test_marker_color_serde() {
        assert_eq!(
            serde_json::to_string(&MarkerColor::Green).unwrap(),
            "\"green\""
        );
        assert_eq!(
            serde_json::to_string(&MarkerColor::Yellow).unwrap(),
            "\"yellow\""
        );
        assert_eq!(serde_json::to_string(&MarkerColor::Red).unwrap(), "\"red\"");
        assert_eq!(
            serde_json::to_string(&MarkerColor::Gray).unwrap(),
            "\"gray\""
        );
    }
}
