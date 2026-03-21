//! Notification handlers: list, mark read, mark all read.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};

use parkhub_common::models::Notification;
use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

/// `GET /api/v1/notifications` — list current user's notifications (most recent 50)
#[utoipa::path(get, path = "/api/v1/notifications", tag = "Notifications",
    summary = "List user notifications",
    description = "Returns recent notifications for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_notifications(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Notification>>>) {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_notifications_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(mut notifications) => {
            // Sort by created_at descending (most recent first)
            notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            notifications.truncate(50);
            (StatusCode::OK, Json(ApiResponse::success(notifications)))
        }
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list notifications",
                )),
            )
        }
    }
}

/// `PUT /api/v1/notifications/{id}/read` — mark notification as read (verify ownership)
#[utoipa::path(put, path = "/api/v1/notifications/{id}/read", tag = "Notifications",
    summary = "Mark notification as read",
    description = "Marks a notification as read.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn mark_notification_read(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership by listing user's notifications
    let notifications = match state_guard
        .db
        .list_notifications_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let owns = notifications.iter().any(|n| n.id.to_string() == id);
    if !owns {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Notification not found")),
        );
    }

    match state_guard.db.mark_notification_read(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Notification not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to mark notification read: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to mark notification read",
                )),
            )
        }
    }
}

/// `POST /api/v1/notifications/read-all` — mark all user's notifications as read
#[utoipa::path(post, path = "/api/v1/notifications/read-all", tag = "Notifications",
    summary = "Mark all notifications as read",
    description = "Marks all notifications as read.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn mark_all_notifications_read(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<u32>>) {
    let state_guard = state.read().await;
    let notifications = match state_guard
        .db
        .list_notifications_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to mark notifications read",
                )),
            );
        }
    };

    let mut count = 0u32;
    for notif in &notifications {
        if !notif.read
            && matches!(
                state_guard
                    .db
                    .mark_notification_read(&notif.id.to_string())
                    .await,
                Ok(true)
            )
        {
            count += 1;
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(count)))
}
