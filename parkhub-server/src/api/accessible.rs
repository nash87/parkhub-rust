//! Accessible Parking — endpoints for accessible slot management
//!
//! - `GET  /api/v1/lots/:id/slots/accessible` — list accessible slots only
//! - `PUT  /api/v1/admin/lots/:id/slots/:slot_id/accessible` — mark slot as accessible
//! - `GET  /api/v1/bookings/accessible-stats` — admin stats on accessible slot usage
//! - `PUT  /api/v1/users/me/accessibility-needs` — update user accessibility needs

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use parkhub_common::{ApiResponse, BookingStatus};

use super::{AuthUser, check_admin};
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// Allowed accessibility need values
const VALID_NEEDS: &[&str] = &[
    "wheelchair",
    "reduced_mobility",
    "visual",
    "hearing",
    "none",
];

/// Priority booking head-start for accessible users (minutes)
const ACCESSIBLE_PRIORITY_MINUTES: i64 = 30;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Accessible slot info returned by the list endpoint
#[derive(Debug, Serialize)]
pub struct AccessibleSlotInfo {
    pub slot_id: String,
    pub lot_id: String,
    pub slot_number: i32,
    pub status: String,
    pub slot_type: String,
    pub is_accessible: bool,
}

/// Request to toggle accessible status on a slot
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetAccessibleRequest {
    pub is_accessible: bool,
}

/// Request to update user accessibility needs
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateAccessibilityNeedsRequest {
    pub accessibility_needs: String,
}

/// Stats on accessible slot usage
#[derive(Debug, Serialize)]
pub struct AccessibleStats {
    pub total_accessible_slots: usize,
    pub occupied_accessible_slots: usize,
    pub utilization_percent: f64,
    pub total_accessible_bookings: usize,
    pub users_with_accessibility_needs: usize,
    pub priority_booking_active: bool,
    pub priority_minutes: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/lots/:id/slots/accessible
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/:id/slots/accessible` — list accessible slots for a lot
#[utoipa::path(get, path = "/api/v1/lots/{id}/slots/accessible", tag = "Accessible",
    summary = "List accessible slots",
    description = "Returns only slots marked as accessible in the given lot.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Accessible slots list"),
        (status = 404, description = "Lot not found"),
    )
)]
pub async fn list_accessible_slots(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Vec<AccessibleSlotInfo>>>) {
    let state_guard = state.read().await;

    // Verify lot exists
    match state_guard.db.get_parking_lot(&lot_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Failed to get lot {lot_id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get lot")),
            );
        }
    }

    let slots = match state_guard.db.list_slots_by_lot(&lot_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list slots for lot {lot_id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list slots")),
            );
        }
    };

    let accessible: Vec<AccessibleSlotInfo> = slots
        .into_iter()
        .filter(|s| s.is_accessible)
        .map(|s| AccessibleSlotInfo {
            slot_id: s.id.to_string(),
            lot_id: s.lot_id.to_string(),
            slot_number: s.slot_number,
            status: format!("{:?}", s.status).to_lowercase(),
            slot_type: format!("{:?}", s.slot_type).to_lowercase(),
            is_accessible: true,
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(accessible)))
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT /api/v1/admin/lots/:id/slots/:slot_id/accessible
// ─────────────────────────────────────────────────────────────────────────────

/// `PUT /api/v1/admin/lots/:id/slots/:slot_id/accessible` — mark slot as accessible
#[utoipa::path(put, path = "/api/v1/admin/lots/{id}/slots/{slot_id}/accessible", tag = "Accessible",
    summary = "Toggle slot accessibility",
    description = "Mark or unmark a parking slot as accessible. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Slot accessibility updated"),
        (status = 404, description = "Slot not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_set_slot_accessible(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((_lot_id, slot_id)): Path<(String, String)>,
    Json(req): Json<SetAccessibleRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut slot = match state_guard.db.get_parking_slot(&slot_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Slot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Failed to get slot {slot_id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get slot")),
            );
        }
    };

    slot.is_accessible = req.is_accessible;

    if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
        tracing::error!("Failed to save slot {slot_id}: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update slot")),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/bookings/accessible-stats
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/bookings/accessible-stats` — stats on accessible slot usage
#[utoipa::path(get, path = "/api/v1/bookings/accessible-stats", tag = "Accessible",
    summary = "Accessible slot statistics",
    description = "Admin stats: total accessible slots, utilization, bookings on accessible slots.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Accessible stats"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn accessible_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<AccessibleStats>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let users = state_guard.db.list_users().await.unwrap_or_default();

    let mut total_accessible = 0usize;
    let mut occupied_accessible = 0usize;
    let mut accessible_slot_ids = Vec::new();

    for lot in &lots {
        let slots = state_guard
            .db
            .list_slots_by_lot(&lot.id.to_string())
            .await
            .unwrap_or_default();
        for slot in &slots {
            if slot.is_accessible {
                total_accessible += 1;
                accessible_slot_ids.push(slot.id);
                if slot.current_booking.is_some() {
                    occupied_accessible += 1;
                }
            }
        }
    }

    let total_accessible_bookings = bookings
        .iter()
        .filter(|b| {
            accessible_slot_ids.contains(&b.slot_id)
                && (b.status == BookingStatus::Confirmed || b.status == BookingStatus::Active)
        })
        .count();

    let users_with_needs = users
        .iter()
        .filter(|u| {
            u.accessibility_needs
                .as_deref()
                .is_some_and(|n| n != "none" && !n.is_empty())
        })
        .count();

    let utilization = if total_accessible > 0 {
        occupied_accessible as f64 / total_accessible as f64 * 100.0
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(AccessibleStats {
            total_accessible_slots: total_accessible,
            occupied_accessible_slots: occupied_accessible,
            utilization_percent: utilization,
            total_accessible_bookings,
            users_with_accessibility_needs: users_with_needs,
            priority_booking_active: true,
            priority_minutes: ACCESSIBLE_PRIORITY_MINUTES,
        })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT /api/v1/users/me/accessibility-needs
// ─────────────────────────────────────────────────────────────────────────────

/// `PUT /api/v1/users/me/accessibility-needs` — update accessibility needs
#[utoipa::path(put, path = "/api/v1/users/me/accessibility-needs", tag = "Accessible",
    summary = "Update accessibility needs",
    description = "Set the user's accessibility needs (wheelchair, reduced_mobility, visual, hearing, none).",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Accessibility needs updated"),
        (status = 400, description = "Invalid accessibility need"),
    )
)]
pub async fn update_accessibility_needs(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateAccessibilityNeedsRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    if !VALID_NEEDS.contains(&req.accessibility_needs.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_NEED",
                "Invalid accessibility need. Valid values: wheelchair, reduced_mobility, visual, hearing, none",
            )),
        );
    }

    let state_guard = state.read().await;
    let mut user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Failed to get user: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get user")),
            );
        }
    };

    user.accessibility_needs = if req.accessibility_needs == "none" {
        None
    } else {
        Some(req.accessibility_needs)
    };

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save user: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update accessibility needs",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

/// Check if a user has accessibility needs and should get priority booking
/// on accessible slots. Returns true if they get a 30-min head start.
#[allow(dead_code)]
pub fn user_has_accessible_priority(accessibility_needs: Option<&str>) -> bool {
    accessibility_needs.is_some_and(|n| n != "none" && !n.is_empty())
}

/// Check if an accessible booking should be allowed (priority window check).
/// Non-accessible users can only book accessible slots after the priority window.
#[allow(dead_code)]
pub fn check_accessible_priority(
    slot_is_accessible: bool,
    user_needs: Option<&str>,
    booking_start: chrono::DateTime<Utc>,
) -> bool {
    if !slot_is_accessible {
        return true; // Non-accessible slots have no restriction
    }

    if user_has_accessible_priority(user_needs) {
        return true; // Users with needs always allowed
    }

    // Non-accessible users can book accessible slots only if the booking
    // starts more than PRIORITY_MINUTES from now
    let priority_cutoff = Utc::now() + Duration::minutes(ACCESSIBLE_PRIORITY_MINUTES);
    booking_start > priority_cutoff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_accessibility_needs() {
        assert!(VALID_NEEDS.contains(&"wheelchair"));
        assert!(VALID_NEEDS.contains(&"reduced_mobility"));
        assert!(VALID_NEEDS.contains(&"visual"));
        assert!(VALID_NEEDS.contains(&"hearing"));
        assert!(VALID_NEEDS.contains(&"none"));
        assert!(!VALID_NEEDS.contains(&"invalid"));
    }

    #[test]
    fn test_user_has_accessible_priority() {
        assert!(user_has_accessible_priority(Some("wheelchair")));
        assert!(user_has_accessible_priority(Some("reduced_mobility")));
        assert!(user_has_accessible_priority(Some("visual")));
        assert!(user_has_accessible_priority(Some("hearing")));
        assert!(!user_has_accessible_priority(Some("none")));
        assert!(!user_has_accessible_priority(None));
        assert!(!user_has_accessible_priority(Some("")));
    }

    #[test]
    fn test_check_accessible_priority_non_accessible_slot() {
        // Non-accessible slot — always allowed
        assert!(check_accessible_priority(false, None, Utc::now()));
        assert!(check_accessible_priority(
            false,
            Some("wheelchair"),
            Utc::now()
        ));
    }

    #[test]
    fn test_check_accessible_priority_user_with_needs() {
        // Accessible slot + user with needs — always allowed
        assert!(check_accessible_priority(
            true,
            Some("wheelchair"),
            Utc::now()
        ));
        assert!(check_accessible_priority(
            true,
            Some("reduced_mobility"),
            Utc::now()
        ));
    }

    #[test]
    fn test_check_accessible_priority_user_without_needs() {
        // Accessible slot + user without needs — depends on timing
        let future = Utc::now() + Duration::hours(2);
        assert!(check_accessible_priority(true, None, future));

        let soon = Utc::now() + Duration::minutes(10);
        assert!(!check_accessible_priority(true, None, soon));
    }

    #[test]
    fn test_accessible_slot_info_serialization() {
        let info = AccessibleSlotInfo {
            slot_id: "s1".to_string(),
            lot_id: "l1".to_string(),
            slot_number: 1,
            status: "available".to_string(),
            slot_type: "handicap".to_string(),
            is_accessible: true,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["slot_number"], 1);
        assert_eq!(json["is_accessible"], true);
        assert_eq!(json["status"], "available");
    }

    #[test]
    fn test_set_accessible_request_deserialization() {
        let json = r#"{"is_accessible":true}"#;
        let req: SetAccessibleRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_accessible);

        let json = r#"{"is_accessible":false}"#;
        let req: SetAccessibleRequest = serde_json::from_str(json).unwrap();
        assert!(!req.is_accessible);
    }

    #[test]
    fn test_update_needs_request_deserialization() {
        let json = r#"{"accessibility_needs":"wheelchair"}"#;
        let req: UpdateAccessibilityNeedsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.accessibility_needs, "wheelchair");
    }

    #[test]
    fn test_accessible_stats_serialization() {
        let stats = AccessibleStats {
            total_accessible_slots: 10,
            occupied_accessible_slots: 3,
            utilization_percent: 30.0,
            total_accessible_bookings: 15,
            users_with_accessibility_needs: 5,
            priority_booking_active: true,
            priority_minutes: 30,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_accessible_slots"], 10);
        assert_eq!(json["utilization_percent"], 30.0);
        assert_eq!(json["priority_minutes"], 30);
        assert_eq!(json["users_with_accessibility_needs"], 5);
    }
}
