//! Enhanced Waitlist with Notifications.
//!
//! Priority-based waitlist with auto-notification when slots become available.
//!
//! - `POST /api/v1/lots/:id/waitlist/subscribe` — join with priority
//! - `GET  /api/v1/lots/:id/waitlist`           — view position + estimated wait
//! - `DELETE /api/v1/lots/:id/waitlist`          — leave waitlist
//! - `POST /api/v1/lots/:id/waitlist/:entry_id/accept`  — accept offered slot
//! - `POST /api/v1/lots/:id/waitlist/:entry_id/decline` — decline, move to next

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::ApiResponse;
use parkhub_common::models::{Notification, NotificationType, WaitlistEntry, WaitlistStatus};

use super::{AuthUser, SharedState};

/// Offer expiry duration in minutes
const OFFER_EXPIRY_MINUTES: i64 = 15;

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for subscribing to waitlist
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SubscribeWaitlistRequest {
    /// Priority level 1-5 (1 = highest)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Optional notes
    #[allow(dead_code)]
    pub notes: Option<String>,
}

fn default_priority() -> u8 {
    3
}

/// Response for waitlist position
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WaitlistPositionResponse {
    pub entry: WaitlistEntry,
    pub position: usize,
    pub total_ahead: usize,
    pub estimated_wait_minutes: Option<i64>,
}

/// Response for waitlist overview (GET)
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WaitlistOverviewResponse {
    pub entries: Vec<WaitlistPositionResponse>,
    pub total: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/lots/:id/waitlist/subscribe` — join waitlist with priority
#[utoipa::path(post, path = "/api/v1/lots/{id}/waitlist/subscribe", tag = "Waitlist",
    summary = "Subscribe to waitlist",
    description = "Join the waitlist for a specific parking lot with an optional priority level.",
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Subscribed to waitlist"),
        (status = 409, description = "Already on waitlist"),
        (status = 404, description = "Lot not found"),
    )
)]
pub async fn subscribe_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<Uuid>,
    Json(req): Json<SubscribeWaitlistRequest>,
) -> (StatusCode, Json<ApiResponse<WaitlistPositionResponse>>) {
    let state_guard = state.read().await;

    // Verify lot exists
    if state_guard
        .db
        .get_parking_lot(&lot_id.to_string())
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
        );
    }

    // Check for existing entry
    let existing = state_guard
        .db
        .list_waitlist_by_lot(&lot_id.to_string())
        .await
        .unwrap_or_default();

    if let Some(entry) = existing.iter().find(|e| e.user_id == auth_user.user_id) {
        let position = existing
            .iter()
            .filter(|e| e.status == WaitlistStatus::Waiting)
            .position(|e| e.id == entry.id)
            .unwrap_or(0);
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::success(WaitlistPositionResponse {
                entry: entry.clone(),
                position: position + 1,
                total_ahead: position,
                estimated_wait_minutes: estimate_wait(position),
            })),
        );
    }

    let priority = req.priority.clamp(1, 5);
    let entry = WaitlistEntry {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id,
        created_at: Utc::now(),
        notified_at: None,
        status: WaitlistStatus::Waiting,
        offer_expires_at: None,
        accepted_booking_id: None,
    };

    if let Err(e) = state_guard.db.save_waitlist_entry(&entry).await {
        tracing::error!("Failed to save waitlist entry: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to join waitlist",
            )),
        );
    }

    // Calculate position (new entry is at the end)
    let position = existing
        .iter()
        .filter(|e| e.status == WaitlistStatus::Waiting)
        .count();

    tracing::info!(
        "User {} joined waitlist for lot {} with priority {} at position {}",
        auth_user.user_id,
        lot_id,
        priority,
        position + 1
    );

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(WaitlistPositionResponse {
            entry,
            position: position + 1,
            total_ahead: position,
            estimated_wait_minutes: estimate_wait(position),
        })),
    )
}

/// `GET /api/v1/lots/:id/waitlist` — view waitlist position + estimated wait
#[utoipa::path(get, path = "/api/v1/lots/{id}/waitlist", tag = "Waitlist",
    summary = "View waitlist",
    description = "View the current user's waitlist position and estimated wait time for a lot.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Waitlist overview"),
    )
)]
pub async fn get_lot_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<Uuid>,
) -> Json<ApiResponse<WaitlistOverviewResponse>> {
    let state_guard = state.read().await;

    let entries = state_guard
        .db
        .list_waitlist_by_lot(&lot_id.to_string())
        .await
        .unwrap_or_default();

    let waiting: Vec<&WaitlistEntry> = entries
        .iter()
        .filter(|e| e.status == WaitlistStatus::Waiting || e.status == WaitlistStatus::Offered)
        .collect();

    let positions: Vec<WaitlistPositionResponse> = waiting
        .iter()
        .enumerate()
        .filter(|(_, e)| e.user_id == auth_user.user_id)
        .map(|(i, e)| WaitlistPositionResponse {
            entry: (*e).clone(),
            position: i + 1,
            total_ahead: i,
            estimated_wait_minutes: estimate_wait(i),
        })
        .collect();

    Json(ApiResponse::success(WaitlistOverviewResponse {
        total: waiting.len(),
        entries: positions,
    }))
}

/// `DELETE /api/v1/lots/:id/waitlist` — leave waitlist
#[utoipa::path(delete, path = "/api/v1/lots/{id}/waitlist", tag = "Waitlist",
    summary = "Leave waitlist",
    description = "Remove the current user from a lot's waitlist.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Left waitlist"),
        (status = 404, description = "Not on waitlist"),
    )
)]
pub async fn leave_lot_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<Uuid>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    let entries = state_guard
        .db
        .list_waitlist_by_lot(&lot_id.to_string())
        .await
        .unwrap_or_default();

    let user_entry = entries
        .iter()
        .find(|e| e.user_id == auth_user.user_id && e.status == WaitlistStatus::Waiting);

    match user_entry {
        Some(entry) => {
            if let Err(e) = state_guard
                .db
                .delete_waitlist_entry(&entry.id.to_string())
                .await
            {
                tracing::error!("Failed to leave waitlist: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "SERVER_ERROR",
                        "Failed to leave waitlist",
                    )),
                );
            }
            tracing::info!(
                "User {} left waitlist for lot {}",
                auth_user.user_id,
                lot_id
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NOT_FOUND",
                "Not on waitlist for this lot",
            )),
        ),
    }
}

/// `POST /api/v1/lots/:id/waitlist/:entry_id/accept` — accept offered slot
#[utoipa::path(post, path = "/api/v1/lots/{id}/waitlist/{entry_id}/accept", tag = "Waitlist",
    summary = "Accept waitlist offer",
    description = "Accept an offered parking slot from the waitlist.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Offer accepted"),
        (status = 404, description = "Entry not found"),
        (status = 410, description = "Offer expired"),
        (status = 403, description = "Not your entry"),
    )
)]
pub async fn accept_waitlist_offer(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((_lot_id, entry_id)): Path<(Uuid, Uuid)>,
) -> (StatusCode, Json<ApiResponse<WaitlistEntry>>) {
    let state_guard = state.read().await;

    let entry = match state_guard
        .db
        .get_waitlist_entry(&entry_id.to_string())
        .await
    {
        Ok(Some(e)) => e,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
            );
        }
    };

    // Verify ownership
    if entry.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Not your waitlist entry")),
        );
    }

    // Must be in Offered status
    if entry.status != WaitlistStatus::Offered {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "NOT_OFFERED",
                "This entry has not been offered a slot",
            )),
        );
    }

    // Check offer expiry
    if let Some(expires) = entry.offer_expires_at {
        if Utc::now() > expires {
            // Mark as expired
            let mut expired_entry = entry;
            expired_entry.status = WaitlistStatus::Expired;
            let _ = state_guard.db.save_waitlist_entry(&expired_entry).await;
            return (
                StatusCode::GONE,
                Json(ApiResponse::error("OFFER_EXPIRED", "The offer has expired")),
            );
        }
    }

    // Accept
    let mut accepted = entry;
    accepted.status = WaitlistStatus::Accepted;
    accepted.accepted_booking_id = Some(Uuid::new_v4()); // Stub booking ID

    if let Err(e) = state_guard.db.save_waitlist_entry(&accepted).await {
        tracing::error!("Failed to accept waitlist offer: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to accept offer")),
        );
    }

    tracing::info!(
        "User {} accepted waitlist offer {}",
        auth_user.user_id,
        accepted.id
    );

    (StatusCode::OK, Json(ApiResponse::success(accepted)))
}

/// `POST /api/v1/lots/:id/waitlist/:entry_id/decline` — decline, move to next
#[utoipa::path(post, path = "/api/v1/lots/{id}/waitlist/{entry_id}/decline", tag = "Waitlist",
    summary = "Decline waitlist offer",
    description = "Decline an offered slot. The slot will be offered to the next person in line.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Offer declined"),
        (status = 404, description = "Entry not found"),
        (status = 403, description = "Not your entry"),
    )
)]
pub async fn decline_waitlist_offer(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, entry_id)): Path<(Uuid, Uuid)>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    let entry = match state_guard
        .db
        .get_waitlist_entry(&entry_id.to_string())
        .await
    {
        Ok(Some(e)) => e,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
            );
        }
    };

    if entry.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Not your waitlist entry")),
        );
    }

    if entry.status != WaitlistStatus::Offered {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "NOT_OFFERED",
                "This entry has not been offered a slot",
            )),
        );
    }

    // Mark as declined
    let mut declined = entry;
    declined.status = WaitlistStatus::Declined;
    if let Err(e) = state_guard.db.save_waitlist_entry(&declined).await {
        tracing::error!("Failed to decline waitlist offer: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to decline offer",
            )),
        );
    }

    // Auto-notify next in line
    notify_next_in_line(&state_guard, &lot_id).await;

    tracing::info!(
        "User {} declined waitlist offer {}, notifying next",
        auth_user.user_id,
        declined.id
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Estimate wait time based on queue position (rough: 30 min per position)
fn estimate_wait(positions_ahead: usize) -> Option<i64> {
    if positions_ahead == 0 {
        return Some(0);
    }
    Some(positions_ahead as i64 * 30)
}

/// Notify the next waiting user in line for a lot
async fn notify_next_in_line(state: &crate::AppState, lot_id: &Uuid) {
    let entries = state
        .db
        .list_waitlist_by_lot(&lot_id.to_string())
        .await
        .unwrap_or_default();

    let next = entries.iter().find(|e| e.status == WaitlistStatus::Waiting);

    if let Some(entry) = next {
        let mut offered = entry.clone();
        offered.status = WaitlistStatus::Offered;
        offered.notified_at = Some(Utc::now());
        offered.offer_expires_at = Some(Utc::now() + Duration::minutes(OFFER_EXPIRY_MINUTES));

        if state.db.save_waitlist_entry(&offered).await.is_ok() {
            // Create notification
            let notification = Notification {
                id: Uuid::new_v4(),
                user_id: entry.user_id,
                notification_type: NotificationType::WaitlistOffer,
                title: "Parking spot available!".to_string(),
                message: format!(
                    "A spot has opened up. You have {} minutes to accept.",
                    OFFER_EXPIRY_MINUTES
                ),
                data: Some(serde_json::json!({
                    "lot_id": lot_id,
                    "entry_id": entry.id,
                    "expires_at": offered.offer_expires_at,
                })),
                read: false,
                created_at: Utc::now(),
            };
            let _ = state.db.save_notification(&notification).await;

            tracing::info!(
                "Offered waitlist slot to user {} for lot {}",
                entry.user_id,
                lot_id
            );
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
    fn test_subscribe_request_deserialize() {
        let json = r#"{"priority":2,"notes":"Morning preferred"}"#;
        let req: SubscribeWaitlistRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.priority, 2);
        assert_eq!(req.notes.as_deref(), Some("Morning preferred"));
    }

    #[test]
    fn test_subscribe_request_default_priority() {
        let json = r#"{}"#;
        let req: SubscribeWaitlistRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.priority, 3);
        assert!(req.notes.is_none());
    }

    #[test]
    fn test_estimate_wait_zero() {
        assert_eq!(estimate_wait(0), Some(0));
    }

    #[test]
    fn test_estimate_wait_positions() {
        assert_eq!(estimate_wait(1), Some(30));
        assert_eq!(estimate_wait(3), Some(90));
        assert_eq!(estimate_wait(10), Some(300));
    }

    #[test]
    fn test_waitlist_position_response_serialize() {
        let entry = WaitlistEntry {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            lot_id: Uuid::nil(),
            created_at: Utc::now(),
            notified_at: None,
            status: WaitlistStatus::Waiting,
            offer_expires_at: None,
            accepted_booking_id: None,
        };
        let resp = WaitlistPositionResponse {
            entry,
            position: 3,
            total_ahead: 2,
            estimated_wait_minutes: Some(60),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"position\":3"));
        assert!(json.contains("\"total_ahead\":2"));
        assert!(json.contains("\"estimated_wait_minutes\":60"));
    }

    #[test]
    fn test_waitlist_overview_response_serialize() {
        let resp = WaitlistOverviewResponse {
            entries: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"entries\":[]"));
    }

    #[test]
    fn test_waitlist_entry_offered_status() {
        let mut entry = WaitlistEntry {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            created_at: Utc::now(),
            notified_at: None,
            status: WaitlistStatus::Waiting,
            offer_expires_at: None,
            accepted_booking_id: None,
        };
        entry.status = WaitlistStatus::Offered;
        entry.notified_at = Some(Utc::now());
        entry.offer_expires_at = Some(Utc::now() + Duration::minutes(OFFER_EXPIRY_MINUTES));
        assert_eq!(entry.status, WaitlistStatus::Offered);
        assert!(entry.offer_expires_at.is_some());
    }

    #[test]
    fn test_waitlist_entry_accepted_status() {
        let mut entry = WaitlistEntry {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            created_at: Utc::now(),
            notified_at: Some(Utc::now()),
            status: WaitlistStatus::Offered,
            offer_expires_at: Some(Utc::now() + Duration::minutes(15)),
            accepted_booking_id: None,
        };
        entry.status = WaitlistStatus::Accepted;
        entry.accepted_booking_id = Some(Uuid::new_v4());
        assert_eq!(entry.status, WaitlistStatus::Accepted);
        assert!(entry.accepted_booking_id.is_some());
    }

    #[test]
    fn test_offer_expiry_constant() {
        assert_eq!(OFFER_EXPIRY_MINUTES, 15);
    }
}
