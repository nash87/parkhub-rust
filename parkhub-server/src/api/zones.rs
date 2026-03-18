//! Zone handlers: CRUD operations for parking lot zones.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::{ApiResponse, UserRole};

use crate::db::Zone;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Request DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateZoneRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/{lot_id}/zones` — list zones for a lot
pub(crate) async fn list_zones(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
) -> Json<ApiResponse<Vec<Zone>>> {
    let state = state.read().await;

    match state.db.list_zones_by_lot(&lot_id).await {
        Ok(zones) => Json(ApiResponse::success(zones)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error("SERVER_ERROR", "Failed to list zones"))
        }
    }
}

/// `POST /api/v1/lots/{lot_id}/zones` — create a zone (admin only)
pub(crate) async fn create_zone(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
    Json(req): Json<CreateZoneRequest>,
) -> (StatusCode, Json<ApiResponse<Zone>>) {
    let state_guard = state.write().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    // Verify lot exists
    match state_guard.db.get_parking_lot(&lot_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    }

    let lot_uuid = match lot_id.parse::<Uuid>() {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("VALIDATION_ERROR", "Invalid lot ID")),
            );
        }
    };

    let zone = Zone {
        id: Uuid::new_v4(),
        lot_id: lot_uuid,
        name: req.name,
        description: req.description,
        color: req.color,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_zone(&zone).await {
        tracing::error!("Failed to save zone: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create zone")),
        );
    }

    tracing::info!("Created zone '{}' ({}) in lot {}", zone.name, zone.id, lot_id);

    (StatusCode::CREATED, Json(ApiResponse::success(zone)))
}

/// `DELETE /api/v1/lots/{lot_id}/zones/{zone_id}` — delete a zone (admin only)
pub(crate) async fn delete_zone(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, zone_id)): Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    match state_guard.db.delete_zone(&lot_id, &zone_id).await {
        Ok(true) => {
            tracing::info!("Deleted zone {} from lot {}", zone_id, lot_id);
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Zone not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete zone: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to delete zone")),
            )
        }
    }
}
