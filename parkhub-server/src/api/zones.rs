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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateZoneRequest {
    /// Zone name (e.g. "VIP Section", "Level A")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Display color (hex code, e.g. "#FFD700")
    pub color: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/{lot_id}/zones` — list zones for a lot
#[utoipa::path(
    get,
    path = "/api/v1/lots/{lot_id}/zones",
    tag = "Zones",
    summary = "List zones for a lot",
    description = "Returns all zones defined in the specified parking lot.",
    params(("lot_id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "List of zones"),
    )
)]
pub async fn list_zones(
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
#[utoipa::path(
    post,
    path = "/api/v1/lots/{lot_id}/zones",
    tag = "Zones",
    summary = "Create a zone",
    description = "Create a new zone within a parking lot. Admin only.",
    params(("lot_id" = String, Path, description = "Parking lot ID")),
    request_body = CreateZoneRequest,
    responses(
        (status = 201, description = "Zone created"),
        (status = 400, description = "Invalid lot ID"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn create_zone(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
    Json(req): Json<CreateZoneRequest>,
) -> (StatusCode, Json<ApiResponse<Zone>>) {
    let state_guard = state.read().await;

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

    let Ok(lot_uuid) = lot_id.parse::<Uuid>() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("VALIDATION_ERROR", "Invalid lot ID")),
        );
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
    drop(state_guard);

    tracing::info!(
        "Created zone '{}' ({}) in lot {}",
        zone.name,
        zone.id,
        lot_id
    );

    (StatusCode::CREATED, Json(ApiResponse::success(zone)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Update zone
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateZoneRequest {
    /// New zone name (optional)
    pub name: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New display color (optional, hex code e.g. "#FFD700")
    pub color: Option<String>,
}

/// `PUT /api/v1/lots/{lot_id}/zones/{zone_id}` — update a zone (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/lots/{lot_id}/zones/{zone_id}",
    tag = "Zones",
    summary = "Update a zone",
    description = "Partially update a zone's name, description, or color. Admin only.",
    params(
        ("lot_id" = String, Path, description = "Parking lot ID"),
        ("zone_id" = String, Path, description = "Zone ID"),
    ),
    request_body = UpdateZoneRequest,
    responses(
        (status = 200, description = "Zone updated"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Zone not found"),
    )
)]
pub async fn update_zone(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, zone_id)): Path<(String, String)>,
    Json(req): Json<UpdateZoneRequest>,
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

    // Load zones for the lot and find the target zone
    let zones = match state_guard.db.list_zones_by_lot(&lot_id).await {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let Some(mut zone) = zones.into_iter().find(|z| z.id.to_string() == zone_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Zone not found")),
        );
    };

    // Apply partial updates
    if let Some(name) = req.name {
        zone.name = name;
    }
    if let Some(description) = req.description {
        zone.description = Some(description);
    }
    if let Some(color) = req.color {
        zone.color = Some(color);
    }

    if let Err(e) = state_guard.db.save_zone(&zone).await {
        tracing::error!("Failed to update zone: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update zone")),
        );
    }

    tracing::info!(
        "Updated zone '{}' ({}) in lot {}",
        zone.name,
        zone.id,
        lot_id
    );

    (StatusCode::OK, Json(ApiResponse::success(zone)))
}

/// `DELETE /api/v1/lots/{lot_id}/zones/{zone_id}` — delete a zone (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/lots/{lot_id}/zones/{zone_id}",
    tag = "Zones",
    summary = "Delete a zone",
    description = "Remove a zone from a parking lot. Admin only.",
    params(
        ("lot_id" = String, Path, description = "Parking lot ID"),
        ("zone_id" = String, Path, description = "Zone ID"),
    ),
    responses(
        (status = 200, description = "Zone deleted"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Zone not found"),
    )
)]
pub async fn delete_zone(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, zone_id)): Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_zone_request_full() {
        let json = r##"{"name":"VIP Section","description":"Premium spots","color":"#FFD700"}"##;
        let req: CreateZoneRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "VIP Section");
        assert_eq!(req.description.as_deref(), Some("Premium spots"));
        assert_eq!(req.color.as_deref(), Some("#FFD700"));
    }

    #[test]
    fn test_create_zone_request_minimal() {
        let json = r#"{"name":"Level A"}"#;
        let req: CreateZoneRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Level A");
        assert!(req.description.is_none());
        assert!(req.color.is_none());
    }

    #[test]
    fn test_create_zone_request_missing_name() {
        let json = r#"{"description":"No name"}"#;
        let result: Result<CreateZoneRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_zone_serde_roundtrip() {
        let zone = Zone {
            id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            name: "Floor B".to_string(),
            description: Some("Second floor".to_string()),
            color: Some("green".to_string()),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&zone).unwrap();
        let deserialized: Zone = serde_json::from_str(&json).unwrap();
        assert_eq!(zone.id, deserialized.id);
        assert_eq!(zone.name, deserialized.name);
        assert_eq!(zone.description, deserialized.description);
        assert_eq!(zone.color, deserialized.color);
    }

    #[test]
    fn test_update_zone_request_full() {
        let json = r##"{"name":"Updated Name","description":"New desc","color":"#123456"}"##;
        let req: UpdateZoneRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Updated Name"));
        assert_eq!(req.description.as_deref(), Some("New desc"));
        assert_eq!(req.color.as_deref(), Some("#123456"));
    }

    #[test]
    fn test_update_zone_request_empty() {
        let json = r#"{}"#;
        let req: UpdateZoneRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.description.is_none());
        assert!(req.color.is_none());
    }

    #[test]
    fn test_update_zone_request_partial() {
        let json = r#"{"name":"Only Name"}"#;
        let req: UpdateZoneRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Only Name"));
        assert!(req.description.is_none());
        assert!(req.color.is_none());
    }

    #[test]
    fn test_zone_no_optional_fields() {
        let zone = Zone {
            id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            name: "Basic".to_string(),
            description: None,
            color: None,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&zone).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["description"], serde_json::Value::Null);
        assert_eq!(value["color"], serde_json::Value::Null);
    }
}
