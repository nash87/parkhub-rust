//! Waitlist handlers: list, join, leave.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::models::{WaitlistEntry, WaitlistStatus};
use parkhub_common::ApiResponse;

use super::settings::read_admin_setting;
use super::{AuthUser, SharedState};

/// `GET /api/v1/waitlist` — list current user's waitlist entries
#[utoipa::path(get, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "List waitlist entries",
    description = "Returns waitlist entries for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<WaitlistEntry>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(entries) => Json(ApiResponse::success(entries)),
        Err(e) => {
            tracing::error!("Failed to list waitlist entries: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list waitlist entries",
            ))
        }
    }
}

/// Request body for joining the waitlist
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct JoinWaitlistRequest {
    lot_id: Uuid,
}

/// `POST /api/v1/waitlist` — join waitlist for a lot
#[utoipa::path(post, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "Join waitlist",
    description = "Adds the user to a lot waitlist.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn join_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<JoinWaitlistRequest>,
) -> (StatusCode, Json<ApiResponse<WaitlistEntry>>) {
    let state_guard = state.read().await;

    // Check waitlist_enabled setting
    let waitlist_enabled = read_admin_setting(&state_guard.db, "waitlist_enabled").await;
    if waitlist_enabled != "true" {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "WAITLIST_DISABLED",
                "Waitlist is not enabled",
            )),
        );
    }

    // First-or-create: check if user already has a waitlist entry for this lot
    let existing = state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();
    if let Some(entry) = existing.iter().find(|e| e.lot_id == req.lot_id) {
        return (StatusCode::OK, Json(ApiResponse::success(entry.clone())));
    }

    let entry = WaitlistEntry {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
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

    (StatusCode::CREATED, Json(ApiResponse::success(entry)))
}

/// `DELETE /api/v1/waitlist/{id}` — leave waitlist (verify ownership)
#[utoipa::path(delete, path = "/api/v1/waitlist/{id}", tag = "Waitlist",
    summary = "Leave waitlist",
    description = "Removes the user from a waitlist entry.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn leave_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership
    match state_guard.db.get_waitlist_entry(&id).await {
        Ok(Some(entry)) => {
            if entry.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
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

    match state_guard.db.delete_waitlist_entry(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete waitlist entry: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to leave waitlist",
                )),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_waitlist_request() {
        let json = r#"{"lot_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: JoinWaitlistRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.lot_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
