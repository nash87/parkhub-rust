//! Fleet / Vehicle Management — admin endpoints
//!
//! - `GET  /api/v1/admin/fleet` — all vehicles across all users with stats
//! - `GET  /api/v1/admin/fleet/stats` — fleet overview (types, electric ratio)
//! - `PUT  /api/v1/admin/fleet/:id/flag` — flag a vehicle

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use parkhub_common::{ApiResponse, VehicleType};

use super::{check_admin, AuthUser};
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Vehicle entry with usage stats for admin fleet view
#[derive(Debug, Serialize)]
pub struct FleetVehicle {
    pub id: String,
    pub user_id: String,
    pub username: Option<String>,
    pub license_plate: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub vehicle_type: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub bookings_count: usize,
    pub last_used: Option<DateTime<Utc>>,
    pub flagged: bool,
    pub flag_reason: Option<String>,
}

/// Fleet-level statistics
#[derive(Debug, Serialize)]
pub struct FleetStats {
    pub total_vehicles: usize,
    pub types_distribution: HashMap<String, usize>,
    pub electric_count: usize,
    pub electric_ratio: f64,
    pub flagged_count: usize,
}

/// Request to flag a vehicle
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct FlagVehicleRequest {
    pub flagged: bool,
    pub reason: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/fleet
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/fleet` — list all vehicles with stats
#[utoipa::path(get, path = "/api/v1/admin/fleet", tag = "Admin",
    summary = "Fleet overview",
    description = "List all vehicles across all users with booking stats. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Vehicle list"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_fleet_list(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<Vec<FleetVehicle>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let vehicles = match state_guard.db.list_all_vehicles().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to list fleet vehicles: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list vehicles",
                )),
            );
        }
    };

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let users = state_guard.db.list_users().await.unwrap_or_default();

    let search = params.get("search").cloned();
    let type_filter = params.get("type").cloned();

    // Build user lookup
    let user_map: HashMap<Uuid, String> =
        users.iter().map(|u| (u.id, u.username.clone())).collect();

    // Read flags from settings (stored as "vehicle_flag:{id}" => reason)
    let mut fleet: Vec<FleetVehicle> = Vec::new();

    for v in &vehicles {
        let vtype = format!("{:?}", v.vehicle_type).to_lowercase();

        if let Some(ref tf) = type_filter {
            if vtype != tf.to_lowercase() {
                continue;
            }
        }

        let username = user_map.get(&v.user_id).cloned();

        if let Some(ref q) = search {
            let q = q.to_lowercase();
            let matches = v.license_plate.to_lowercase().contains(&q)
                || v.make
                    .as_ref()
                    .is_some_and(|m| m.to_lowercase().contains(&q))
                || v.model
                    .as_ref()
                    .is_some_and(|m| m.to_lowercase().contains(&q))
                || username
                    .as_ref()
                    .is_some_and(|u| u.to_lowercase().contains(&q));
            if !matches {
                continue;
            }
        }

        // Count bookings for this vehicle
        let vehicle_bookings: Vec<_> = bookings
            .iter()
            .filter(|b| b.vehicle.license_plate == v.license_plate && b.user_id == v.user_id)
            .collect();
        let bookings_count = vehicle_bookings.len();
        let last_used = vehicle_bookings.iter().map(|b| b.end_time).max();

        // Check flag
        let flag_key = format!("vehicle_flag:{}", v.id);
        let flag_reason = state_guard.db.get_setting(&flag_key).await.ok().flatten();
        let flagged = flag_reason.is_some();

        fleet.push(FleetVehicle {
            id: v.id.to_string(),
            user_id: v.user_id.to_string(),
            username,
            license_plate: v.license_plate.clone(),
            make: v.make.clone(),
            model: v.model.clone(),
            color: v.color.clone(),
            vehicle_type: vtype,
            is_default: v.is_default,
            created_at: v.created_at,
            bookings_count,
            last_used,
            flagged,
            flag_reason,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(fleet)))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/fleet/stats
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/fleet/stats` — fleet statistics
#[utoipa::path(get, path = "/api/v1/admin/fleet/stats", tag = "Admin",
    summary = "Fleet statistics",
    description = "Fleet overview: total, types distribution, electric ratio. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fleet stats"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_fleet_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<FleetStats>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let vehicles = match state_guard.db.list_all_vehicles().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to list fleet for stats: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to get fleet stats",
                )),
            );
        }
    };

    let total = vehicles.len();
    let mut types_distribution: HashMap<String, usize> = HashMap::new();
    let mut electric_count = 0usize;
    let mut flagged_count = 0usize;

    for v in &vehicles {
        let vtype = format!("{:?}", v.vehicle_type).to_lowercase();
        *types_distribution.entry(vtype).or_insert(0) += 1;
        if v.vehicle_type == VehicleType::Electric {
            electric_count += 1;
        }
        let flag_key = format!("vehicle_flag:{}", v.id);
        if state_guard
            .db
            .get_setting(&flag_key)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            flagged_count += 1;
        }
    }

    let electric_ratio = if total > 0 {
        electric_count as f64 / total as f64
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(FleetStats {
            total_vehicles: total,
            types_distribution,
            electric_count,
            electric_ratio,
            flagged_count,
        })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT /api/v1/admin/fleet/:id/flag
// ─────────────────────────────────────────────────────────────────────────────

/// `PUT /api/v1/admin/fleet/:id/flag` — flag/unflag a vehicle
#[utoipa::path(put, path = "/api/v1/admin/fleet/{id}/flag", tag = "Admin",
    summary = "Flag vehicle",
    description = "Flag or unflag a vehicle (stolen, expired registration, etc.). Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Vehicle flagged"),
        (status = 404, description = "Vehicle not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_fleet_flag(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<FlagVehicleRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Verify vehicle exists
    match state_guard.db.get_vehicle(&id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            )
        }
        Err(e) => {
            tracing::error!("Failed to get vehicle {id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to look up vehicle",
                )),
            );
        }
    }

    let flag_key = format!("vehicle_flag:{id}");
    if req.flagged {
        let reason = req.reason.unwrap_or_else(|| "flagged".to_string());
        if let Err(e) = state_guard.db.set_setting(&flag_key, &reason).await {
            tracing::error!("Failed to flag vehicle {id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to flag vehicle")),
            );
        }
    } else if let Err(e) = state_guard.db.set_setting(&flag_key, "").await {
        tracing::error!("Failed to unflag vehicle {id}: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to unflag vehicle",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fleet_vehicle_serialization() {
        let v = FleetVehicle {
            id: "v1".to_string(),
            user_id: "u1".to_string(),
            username: Some("alice".to_string()),
            license_plate: "AB-CD-123".to_string(),
            make: Some("Tesla".to_string()),
            model: Some("Model 3".to_string()),
            color: Some("white".to_string()),
            vehicle_type: "electric".to_string(),
            is_default: true,
            created_at: Utc::now(),
            bookings_count: 5,
            last_used: Some(Utc::now()),
            flagged: false,
            flag_reason: None,
        };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["license_plate"], "AB-CD-123");
        assert_eq!(json["vehicle_type"], "electric");
        assert_eq!(json["bookings_count"], 5);
        assert!(!json["flagged"].as_bool().unwrap());
    }

    #[test]
    fn test_fleet_stats_serialization() {
        let mut dist = HashMap::new();
        dist.insert("car".to_string(), 10);
        dist.insert("electric".to_string(), 5);
        let stats = FleetStats {
            total_vehicles: 15,
            types_distribution: dist,
            electric_count: 5,
            electric_ratio: 5.0 / 15.0,
            flagged_count: 1,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_vehicles"], 15);
        assert_eq!(json["electric_count"], 5);
        assert_eq!(json["flagged_count"], 1);
    }

    #[test]
    fn test_fleet_stats_empty() {
        let stats = FleetStats {
            total_vehicles: 0,
            types_distribution: HashMap::new(),
            electric_count: 0,
            electric_ratio: 0.0,
            flagged_count: 0,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_vehicles"], 0);
        assert_eq!(json["electric_ratio"], 0.0);
    }

    #[test]
    fn test_flag_request_deserialization() {
        let json = r#"{"flagged":true,"reason":"stolen vehicle"}"#;
        let req: FlagVehicleRequest = serde_json::from_str(json).unwrap();
        assert!(req.flagged);
        assert_eq!(req.reason.as_deref(), Some("stolen vehicle"));
    }

    #[test]
    fn test_flag_request_no_reason() {
        let json = r#"{"flagged":false}"#;
        let req: FlagVehicleRequest = serde_json::from_str(json).unwrap();
        assert!(!req.flagged);
        assert!(req.reason.is_none());
    }

    #[test]
    fn test_vehicle_type_bicycle() {
        let json = r#""bicycle""#;
        let vt: VehicleType = serde_json::from_str(json).unwrap();
        assert_eq!(vt, VehicleType::Bicycle);
    }
}
