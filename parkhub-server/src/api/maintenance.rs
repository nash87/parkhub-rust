//! Maintenance Scheduling — endpoints for maintenance window management
//!
//! - `POST   /api/v1/admin/maintenance` — create maintenance window
//! - `GET    /api/v1/admin/maintenance` — list all scheduled maintenance
//! - `PUT    /api/v1/admin/maintenance/:id` — update maintenance window
//! - `DELETE /api/v1/admin/maintenance/:id` — cancel maintenance window
//! - `GET    /api/v1/maintenance/active` — current active maintenance (public)

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use super::{AuthUser, check_admin};
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// A maintenance window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub lot_name: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub reason: String,
    pub affected_slots: AffectedSlots,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Which slots are affected by maintenance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AffectedSlots {
    All,
    Specific { slot_ids: Vec<String> },
}

/// Request to create a maintenance window
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateMaintenanceRequest {
    pub lot_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub reason: String,
    /// "all" or list of slot IDs
    pub affected_slots: Option<Vec<String>>,
}

/// Request to update a maintenance window
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateMaintenanceRequest {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub reason: Option<String>,
    pub affected_slots: Option<Vec<String>>,
}

/// Active maintenance info for public display
#[derive(Debug, Serialize)]
pub struct ActiveMaintenance {
    pub id: String,
    pub lot_id: String,
    pub lot_name: String,
    pub reason: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub affected_slots: AffectedSlots,
}

// Settings key prefix for maintenance windows
const MAINTENANCE_PREFIX: &str = "maintenance:";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn list_all_maintenance(state: &AppState) -> Vec<MaintenanceWindow> {
    let mut windows = Vec::new();
    // Scan all settings with maintenance: prefix
    // Since we don't have a prefix scan, we store a list of IDs
    let ids_json = state
        .db
        .get_setting("maintenance_ids")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();

    for id in &ids {
        let key = format!("{MAINTENANCE_PREFIX}{id}");
        if let Ok(Some(val)) = state.db.get_setting(&key).await
            && let Ok(window) = serde_json::from_str::<MaintenanceWindow>(&val)
        {
            windows.push(window);
        }
    }

    windows.sort_by_key(|w| w.start_time);
    windows
}

async fn save_maintenance(state: &AppState, window: &MaintenanceWindow) -> anyhow::Result<()> {
    let key = format!("{MAINTENANCE_PREFIX}{}", window.id);
    let val = serde_json::to_string(window)?;
    state.db.set_setting(&key, &val).await?;

    // Update ID list
    let ids_json = state
        .db
        .get_setting("maintenance_ids")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
    let id_str = window.id.to_string();
    if !ids.contains(&id_str) {
        ids.push(id_str);
    }
    state
        .db
        .set_setting("maintenance_ids", &serde_json::to_string(&ids)?)
        .await?;

    Ok(())
}

async fn delete_maintenance_by_id(state: &AppState, id: &str) -> anyhow::Result<()> {
    let key = format!("{MAINTENANCE_PREFIX}{id}");
    state.db.set_setting(&key, "").await?;

    let ids_json = state
        .db
        .get_setting("maintenance_ids")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
    ids.retain(|i| i != id);
    state
        .db
        .set_setting("maintenance_ids", &serde_json::to_string(&ids)?)
        .await?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/v1/admin/maintenance
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/admin/maintenance` — create maintenance window
#[utoipa::path(post, path = "/api/v1/admin/maintenance", tag = "Maintenance",
    summary = "Create maintenance window",
    description = "Schedule a maintenance window for a lot. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Maintenance window created"),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn create_maintenance(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateMaintenanceRequest>,
) -> (StatusCode, Json<ApiResponse<MaintenanceWindow>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if req.end_time <= req.start_time {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_TIME_RANGE",
                "End time must be after start time",
            )),
        );
    }

    if req.reason.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_REASON", "Reason is required")),
        );
    }

    // Verify lot exists
    let lot_name = match state_guard.db.get_parking_lot(&req.lot_id).await {
        Ok(Some(lot)) => lot.name,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Failed to get lot: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get lot")),
            );
        }
    };

    let affected = match &req.affected_slots {
        None => AffectedSlots::All,
        Some(ids) if ids.is_empty() => AffectedSlots::All,
        Some(ids) => AffectedSlots::Specific {
            slot_ids: ids.clone(),
        },
    };

    let now = Utc::now();
    let window = MaintenanceWindow {
        id: Uuid::new_v4(),
        lot_id: Uuid::parse_str(&req.lot_id).unwrap_or_else(|_| Uuid::new_v4()),
        lot_name: Some(lot_name),
        start_time: req.start_time,
        end_time: req.end_time,
        reason: req.reason,
        affected_slots: affected,
        created_by: Some(auth_user.user_id),
        created_at: now,
        updated_at: now,
    };

    if let Err(e) = save_maintenance(&state_guard, &window).await {
        tracing::error!("Failed to save maintenance: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create maintenance",
            )),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(window)))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/maintenance
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/maintenance` — list all maintenance windows
#[utoipa::path(get, path = "/api/v1/admin/maintenance", tag = "Maintenance",
    summary = "List all maintenance",
    description = "List all scheduled maintenance windows. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Maintenance list"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn list_maintenance(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<MaintenanceWindow>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let windows = list_all_maintenance(&state_guard).await;
    (StatusCode::OK, Json(ApiResponse::success(windows)))
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT /api/v1/admin/maintenance/:id
// ─────────────────────────────────────────────────────────────────────────────

/// `PUT /api/v1/admin/maintenance/:id` — update maintenance window
#[utoipa::path(put, path = "/api/v1/admin/maintenance/{id}", tag = "Maintenance",
    summary = "Update maintenance window",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Updated"),
        (status = 404, description = "Not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn update_maintenance(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMaintenanceRequest>,
) -> (StatusCode, Json<ApiResponse<MaintenanceWindow>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let key = format!("{MAINTENANCE_PREFIX}{id}");
    let mut window = match state_guard.db.get_setting(&key).await {
        Ok(Some(val)) if !val.is_empty() => match serde_json::from_str::<MaintenanceWindow>(&val) {
            Ok(w) => w,
            Err(_) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("NOT_FOUND", "Maintenance not found")),
                );
            }
        },
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Maintenance not found")),
            );
        }
    };

    if let Some(start) = req.start_time {
        window.start_time = start;
    }
    if let Some(end) = req.end_time {
        window.end_time = end;
    }
    if let Some(reason) = req.reason {
        window.reason = reason;
    }
    if let Some(slots) = req.affected_slots {
        window.affected_slots = if slots.is_empty() {
            AffectedSlots::All
        } else {
            AffectedSlots::Specific { slot_ids: slots }
        };
    }
    window.updated_at = Utc::now();

    if let Err(e) = save_maintenance(&state_guard, &window).await {
        tracing::error!("Failed to update maintenance: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update maintenance",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(window)))
}

// ─────────────────────────────────────────────────────────────────────────────
// DELETE /api/v1/admin/maintenance/:id
// ─────────────────────────────────────────────────────────────────────────────

/// `DELETE /api/v1/admin/maintenance/:id` — cancel maintenance
#[utoipa::path(delete, path = "/api/v1/admin/maintenance/{id}", tag = "Maintenance",
    summary = "Cancel maintenance",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Cancelled"),
        (status = 404, description = "Not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn delete_maintenance(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Verify exists
    let key = format!("{MAINTENANCE_PREFIX}{id}");
    match state_guard.db.get_setting(&key).await {
        Ok(Some(val)) if !val.is_empty() => {}
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Maintenance not found")),
            );
        }
    }

    if let Err(e) = delete_maintenance_by_id(&state_guard, &id).await {
        tracing::error!("Failed to delete maintenance: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel maintenance",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/maintenance/active
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/maintenance/active` — current active maintenance (public)
#[utoipa::path(get, path = "/api/v1/maintenance/active", tag = "Maintenance",
    summary = "Active maintenance",
    description = "List currently active maintenance windows. Public.",
    responses(
        (status = 200, description = "Active maintenance list"),
    )
)]
pub async fn active_maintenance(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<ActiveMaintenance>>>) {
    let state_guard = state.read().await;
    let now = Utc::now();

    let windows = list_all_maintenance(&state_guard).await;
    let active: Vec<ActiveMaintenance> = windows
        .into_iter()
        .filter(|w| w.start_time <= now && w.end_time > now)
        .map(|w| ActiveMaintenance {
            id: w.id.to_string(),
            lot_id: w.lot_id.to_string(),
            lot_name: w.lot_name.unwrap_or_default(),
            reason: w.reason,
            start_time: w.start_time,
            end_time: w.end_time,
            affected_slots: w.affected_slots,
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(active)))
}

/// Check if a booking overlaps with any maintenance window
#[allow(dead_code)]
pub fn booking_overlaps_maintenance(
    windows: &[MaintenanceWindow],
    lot_id: &Uuid,
    slot_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Option<String> {
    for w in windows {
        if &w.lot_id != lot_id {
            continue;
        }
        // Check time overlap
        if start < w.end_time && end > w.start_time {
            // Check if slot is affected
            let affected = match &w.affected_slots {
                AffectedSlots::All => true,
                AffectedSlots::Specific { slot_ids } => slot_ids.contains(&slot_id.to_string()),
            };
            if affected {
                return Some(w.reason.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintenance_window_serialization() {
        let w = MaintenanceWindow {
            id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            lot_name: Some("Lot A".to_string()),
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::hours(4),
            reason: "Elevator repair".to_string(),
            affected_slots: AffectedSlots::All,
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_value(&w).unwrap();
        assert_eq!(json["reason"], "Elevator repair");
        assert_eq!(json["affected_slots"]["type"], "all");
    }

    #[test]
    fn test_affected_slots_specific() {
        let affected = AffectedSlots::Specific {
            slot_ids: vec!["s1".to_string(), "s2".to_string()],
        };
        let json = serde_json::to_value(&affected).unwrap();
        assert_eq!(json["type"], "specific");
        assert_eq!(json["slot_ids"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_create_request_deserialization() {
        let json = r#"{"lot_id":"abc","start_time":"2026-04-01T08:00:00Z","end_time":"2026-04-01T12:00:00Z","reason":"Painting"}"#;
        let req: CreateMaintenanceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.lot_id, "abc");
        assert_eq!(req.reason, "Painting");
        assert!(req.affected_slots.is_none());
    }

    #[test]
    fn test_update_request_partial() {
        let json = r#"{"reason":"Updated reason"}"#;
        let req: UpdateMaintenanceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.reason.as_deref(), Some("Updated reason"));
        assert!(req.start_time.is_none());
        assert!(req.end_time.is_none());
        assert!(req.affected_slots.is_none());
    }

    #[test]
    fn test_booking_overlaps_no_overlap() {
        let lot_id = Uuid::new_v4();
        let windows = vec![MaintenanceWindow {
            id: Uuid::new_v4(),
            lot_id,
            lot_name: None,
            start_time: Utc::now() + chrono::Duration::hours(10),
            end_time: Utc::now() + chrono::Duration::hours(14),
            reason: "Repair".to_string(),
            affected_slots: AffectedSlots::All,
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        // Booking before maintenance
        let result = booking_overlaps_maintenance(
            &windows,
            &lot_id,
            "s1",
            Utc::now(),
            Utc::now() + chrono::Duration::hours(2),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_booking_overlaps_with_maintenance() {
        let lot_id = Uuid::new_v4();
        let start = Utc::now();
        let end = start + chrono::Duration::hours(4);
        let windows = vec![MaintenanceWindow {
            id: Uuid::new_v4(),
            lot_id,
            lot_name: None,
            start_time: start + chrono::Duration::hours(1),
            end_time: start + chrono::Duration::hours(3),
            reason: "Painting".to_string(),
            affected_slots: AffectedSlots::All,
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        let result = booking_overlaps_maintenance(&windows, &lot_id, "s1", start, end);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Painting");
    }

    #[test]
    fn test_booking_overlaps_specific_slots() {
        let lot_id = Uuid::new_v4();
        let start = Utc::now();
        let end = start + chrono::Duration::hours(4);
        let windows = vec![MaintenanceWindow {
            id: Uuid::new_v4(),
            lot_id,
            lot_name: None,
            start_time: start,
            end_time: end,
            reason: "Repair".to_string(),
            affected_slots: AffectedSlots::Specific {
                slot_ids: vec!["s1".to_string()],
            },
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        // s1 is affected
        assert!(booking_overlaps_maintenance(&windows, &lot_id, "s1", start, end).is_some());
        // s2 is not affected
        assert!(booking_overlaps_maintenance(&windows, &lot_id, "s2", start, end).is_none());
    }

    #[test]
    fn test_booking_overlaps_different_lot() {
        let lot_id = Uuid::new_v4();
        let other_lot = Uuid::new_v4();
        let start = Utc::now();
        let end = start + chrono::Duration::hours(4);
        let windows = vec![MaintenanceWindow {
            id: Uuid::new_v4(),
            lot_id,
            lot_name: None,
            start_time: start,
            end_time: end,
            reason: "Repair".to_string(),
            affected_slots: AffectedSlots::All,
            created_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        // Different lot — no overlap
        assert!(booking_overlaps_maintenance(&windows, &other_lot, "s1", start, end).is_none());
    }
}
