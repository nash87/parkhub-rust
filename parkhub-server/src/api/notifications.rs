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

#[cfg(test)]
mod tests {
    use parkhub_common::models::{Notification, NotificationType};
    use uuid::Uuid;

    // ── Notification model unit tests ──

    #[test]
    fn notification_created_at_sort_order() {
        let now = chrono::Utc::now();
        let older = Notification {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Old".into(),
            message: "Old message".into(),
            notification_type: NotificationType::BookingConfirmed,
            data: None,
            read: false,
            created_at: now - chrono::Duration::hours(2),
        };
        let newer = Notification {
            id: Uuid::new_v4(),
            user_id: older.user_id,
            title: "New".into(),
            message: "New message".into(),
            notification_type: NotificationType::BookingReminder,
            data: None,
            read: false,
            created_at: now,
        };

        let mut notifications = vec![older.clone(), newer.clone()];
        // Sort descending — same logic as list_notifications handler
        notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        assert_eq!(notifications[0].id, newer.id, "newest should come first");
        assert_eq!(notifications[1].id, older.id, "oldest should come last");
    }

    #[test]
    fn notification_truncate_to_50() {
        let user_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let mut notifications: Vec<Notification> = (0..75)
            .map(|i| Notification {
                id: Uuid::new_v4(),
                user_id,
                title: format!("Notification {i}"),
                message: format!("Message {i}"),
                notification_type: NotificationType::BookingConfirmed,
                data: None,
                read: false,
                created_at: now + chrono::Duration::minutes(i),
            })
            .collect();

        notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        notifications.truncate(50);

        assert_eq!(notifications.len(), 50, "should be truncated to 50");
        // First entry should have the latest timestamp (index 74)
        assert!(notifications[0].title.contains("74"));
    }

    #[test]
    fn notification_ownership_check_matches_by_id() {
        let user_id = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let notifications = vec![Notification {
            id: notif_id,
            user_id,
            title: "Test".into(),
            message: "Test msg".into(),
            notification_type: NotificationType::BookingConfirmed,
            data: None,
            read: false,
            created_at: now,
        }];

        let target_id = notif_id.to_string();
        let owns = notifications.iter().any(|n| n.id.to_string() == target_id);
        assert!(owns, "should find notification by id");

        let fake_id = Uuid::new_v4().to_string();
        let owns_fake = notifications.iter().any(|n| n.id.to_string() == fake_id);
        assert!(!owns_fake, "should not find notification with wrong id");
    }

    #[test]
    fn notification_ownership_rejects_other_users_notification() {
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let notif_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // User A's notifications
        let user_a_notifications = vec![Notification {
            id: Uuid::new_v4(),
            user_id: user_a,
            title: "A's notif".into(),
            message: "Message".into(),
            notification_type: NotificationType::BookingConfirmed,
            data: None,
            read: false,
            created_at: now,
        }];

        // notif_id belongs to user B, so user A should not find it
        let owns = user_a_notifications
            .iter()
            .any(|n| n.id.to_string() == notif_id.to_string());
        assert!(!owns, "user A should not own user B's notification");
    }

    #[test]
    fn mark_all_skips_already_read_notifications() {
        let user_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let notifications = vec![
            Notification {
                id: Uuid::new_v4(),
                user_id,
                title: "Read".into(),
                message: "Already read".into(),
                notification_type: NotificationType::BookingConfirmed,
                data: None,
                read: true,
                created_at: now,
            },
            Notification {
                id: Uuid::new_v4(),
                user_id,
                title: "Unread".into(),
                message: "Not yet read".into(),
                notification_type: NotificationType::BookingReminder,
                data: None,
                read: false,
                created_at: now,
            },
        ];

        // Simulate the mark-all logic — count only unread
        let unread_count = notifications.iter().filter(|n| !n.read).count();
        assert_eq!(unread_count, 1, "should only count unread notifications");
    }

    #[test]
    fn empty_notification_list_returns_zero_count() {
        let notifications: Vec<Notification> = vec![];
        let count = notifications.iter().filter(|n| !n.read).count();
        assert_eq!(count, 0);
    }
}
