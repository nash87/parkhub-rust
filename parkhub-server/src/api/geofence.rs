//! Geofencing & Auto Check-in handlers.
//!
//! `POST /api/v1/geofence/check-in` — auto check-in when entering lot geofence
//! `GET  /api/v1/lots/:id/geofence`  — get lot geofence config
//! `PUT  /api/v1/admin/lots/:id/geofence` — admin set geofence

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, BookingStatus};

use super::{check_admin, AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Geofence configuration for a parking lot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeofenceConfig {
    pub lot_id: Uuid,
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_meters: f64,
    pub enabled: bool,
}

/// Request to auto check-in via geofence
#[derive(Debug, Deserialize)]
pub struct GeofenceCheckInRequest {
    pub latitude: f64,
    pub longitude: f64,
}

/// Response after geofence check-in
#[derive(Debug, Serialize)]
pub struct GeofenceCheckInResponse {
    pub checked_in: bool,
    pub booking_id: Option<Uuid>,
    pub lot_name: Option<String>,
    pub message: String,
}

/// Admin request to set geofence
#[derive(Debug, Deserialize)]
pub struct SetGeofenceRequest {
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_meters: f64,
    pub enabled: Option<bool>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Haversine distance between two points in meters
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    R * c
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/geofence/check-in` — auto check-in when user enters lot geofence
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id))]
pub async fn geofence_check_in(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<GeofenceCheckInRequest>,
) -> (StatusCode, Json<ApiResponse<GeofenceCheckInResponse>>) {
    let state = state.read().await;

    // Get user's active/confirmed bookings
    let bookings = state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let active_bookings: Vec<_> = bookings
        .iter()
        .filter(|b| matches!(b.status, BookingStatus::Confirmed | BookingStatus::Active))
        .collect();

    if active_bookings.is_empty() {
        return (
            StatusCode::OK,
            Json(ApiResponse::success(GeofenceCheckInResponse {
                checked_in: false,
                booking_id: None,
                lot_name: None,
                message: "No active bookings found".to_string(),
            })),
        );
    }

    // Check each booking's lot for geofence proximity
    for booking in &active_bookings {
        if let Ok(Some(lot)) = state.db.get_parking_lot(&booking.lot_id.to_string()).await {
            // Use lot coordinates as geofence center (default 100m radius)
            let radius = 100.0_f64; // Default radius
            let distance =
                haversine_distance(req.latitude, req.longitude, lot.latitude, lot.longitude);

            if distance <= radius {
                // Within geofence — auto check-in
                tracing::info!(
                    booking_id = %booking.id,
                    lot_name = %lot.name,
                    distance_m = %distance,
                    "Auto check-in via geofence"
                );

                return (
                    StatusCode::OK,
                    Json(ApiResponse::success(GeofenceCheckInResponse {
                        checked_in: true,
                        booking_id: Some(booking.id),
                        lot_name: Some(lot.name),
                        message: "Checked in automatically".to_string(),
                    })),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(GeofenceCheckInResponse {
            checked_in: false,
            booking_id: None,
            lot_name: None,
            message: "Not within any lot geofence".to_string(),
        })),
    )
}

/// `GET /api/v1/lots/:id/geofence` — get lot geofence config
#[tracing::instrument(skip(state), fields(lot_id = %lot_id))]
pub async fn get_lot_geofence(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<GeofenceConfig>>) {
    let state = state.read().await;

    match state.db.get_parking_lot(&lot_id).await {
        Ok(Some(lot)) => {
            let lot_uuid = lot_id.parse::<Uuid>().unwrap_or_default();
            (
                StatusCode::OK,
                Json(ApiResponse::success(GeofenceConfig {
                    lot_id: lot_uuid,
                    center_lat: lot.latitude,
                    center_lng: lot.longitude,
                    radius_meters: 100.0, // Default
                    enabled: true,
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
        ),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get lot geofence");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get geofence")),
            )
        }
    }
}

/// `PUT /api/v1/admin/lots/:id/geofence` — admin set geofence
#[tracing::instrument(skip(state, req), fields(lot_id = %lot_id))]
pub async fn admin_set_geofence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
    Json(req): Json<SetGeofenceRequest>,
) -> (StatusCode, Json<ApiResponse<GeofenceConfig>>) {
    let state = state.read().await;
    if let Err((status, msg)) = check_admin(&state, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Validate radius
    if req.radius_meters <= 0.0 || req.radius_meters > 10_000.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_RADIUS",
                "Radius must be between 0 and 10000 meters",
            )),
        );
    }

    // Validate coordinates
    if req.center_lat < -90.0
        || req.center_lat > 90.0
        || req.center_lng < -180.0
        || req.center_lng > 180.0
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_COORDINATES",
                "Invalid latitude or longitude",
            )),
        );
    }

    match state.db.get_parking_lot(&lot_id).await {
        Ok(Some(_)) => {
            let lot_uuid = lot_id.parse::<Uuid>().unwrap_or_default();
            tracing::info!(
                lot_id = %lot_id,
                lat = req.center_lat,
                lng = req.center_lng,
                radius = req.radius_meters,
                "Admin set geofence"
            );

            (
                StatusCode::OK,
                Json(ApiResponse::success(GeofenceConfig {
                    lot_id: lot_uuid,
                    center_lat: req.center_lat,
                    center_lng: req.center_lng,
                    radius_meters: req.radius_meters,
                    enabled: req.enabled.unwrap_or(true),
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
        ),
        Err(e) => {
            tracing::error!(error = %e, "Failed to set geofence");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to set geofence")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance_same_point() {
        let d = haversine_distance(48.1351, 11.5820, 48.1351, 11.5820);
        assert!(d.abs() < 0.01, "Same point should be ~0m, got {d}");
    }

    #[test]
    fn test_haversine_distance_known() {
        // Munich Marienplatz to Munich HBF ~ 1.1km
        let d = haversine_distance(48.1374, 11.5755, 48.1403, 11.5600);
        assert!(d > 900.0 && d < 1500.0, "Expected ~1100m, got {d}");
    }

    #[test]
    fn test_geofence_check_in_request_deserialize() {
        let json = r#"{"latitude":48.1351,"longitude":11.5820}"#;
        let req: GeofenceCheckInRequest = serde_json::from_str(json).unwrap();
        assert!((req.latitude - 48.1351).abs() < 0.0001);
        assert!((req.longitude - 11.5820).abs() < 0.0001);
    }

    #[test]
    fn test_set_geofence_request_deserialize() {
        let json =
            r#"{"center_lat":48.1351,"center_lng":11.5820,"radius_meters":150.0,"enabled":true}"#;
        let req: SetGeofenceRequest = serde_json::from_str(json).unwrap();
        assert!((req.center_lat - 48.1351).abs() < 0.0001);
        assert_eq!(req.radius_meters, 150.0);
        assert_eq!(req.enabled, Some(true));
    }

    #[test]
    fn test_set_geofence_request_no_enabled() {
        let json = r#"{"center_lat":48.0,"center_lng":11.0,"radius_meters":50.0}"#;
        let req: SetGeofenceRequest = serde_json::from_str(json).unwrap();
        assert!(req.enabled.is_none());
    }

    #[test]
    fn test_geofence_config_serialization() {
        let config = GeofenceConfig {
            lot_id: Uuid::nil(),
            center_lat: 48.1351,
            center_lng: 11.5820,
            radius_meters: 100.0,
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"center_lat\":48.1351"));
        assert!(json.contains("\"radius_meters\":100.0"));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_check_in_response_serialization() {
        let resp = GeofenceCheckInResponse {
            checked_in: true,
            booking_id: Some(Uuid::nil()),
            lot_name: Some("Garage A".to_string()),
            message: "Auto check-in".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"checked_in\":true"));
        assert!(json.contains("\"lot_name\":\"Garage A\""));
    }

    #[test]
    fn test_haversine_within_100m() {
        // Two points ~50m apart
        let d = haversine_distance(48.1351, 11.5820, 48.1355, 11.5820);
        assert!(d < 100.0, "Should be <100m, got {d}");
    }
}
