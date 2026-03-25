//! Smart Notification Center — in-app notification system with real-time badge count.
//!
//! Provides a unified notification hub with typed notifications, read/unread filtering,
//! pagination, and badge count for UI integration.
//!
//! Endpoints:
//! - `GET    /api/v1/notifications/center`          — paginated list with read/unread filter
//! - `PUT    /api/v1/notifications/:id/read`         — mark as read (already in notifications.rs)
//! - `PUT    /api/v1/notifications/read-all`         — mark all as read
//! - `GET    /api/v1/notifications/unread-count`     — badge count
//! - `DELETE /api/v1/notifications/:id`              — delete single notification

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Supported notification types for the notification center.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    BookingConfirmed,
    BookingCancelled,
    BookingReminder,
    WaitlistOffer,
    MaintenanceAlert,
    SystemAnnouncement,
    PaymentReceived,
    VisitorArrived,
}

impl NotificationType {
    /// Human-readable label.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::BookingConfirmed => "Booking Confirmed",
            Self::BookingCancelled => "Booking Cancelled",
            Self::BookingReminder => "Booking Reminder",
            Self::WaitlistOffer => "Waitlist Offer",
            Self::MaintenanceAlert => "Maintenance Alert",
            Self::SystemAnnouncement => "System Announcement",
            Self::PaymentReceived => "Payment Received",
            Self::VisitorArrived => "Visitor Arrived",
        }
    }

    /// Icon hint for the frontend.
    pub const fn icon(self) -> &'static str {
        match self {
            Self::BookingConfirmed => "check-circle",
            Self::BookingCancelled => "x-circle",
            Self::BookingReminder => "clock",
            Self::WaitlistOffer => "queue",
            Self::MaintenanceAlert => "wrench",
            Self::SystemAnnouncement => "megaphone",
            Self::PaymentReceived => "currency-dollar",
            Self::VisitorArrived => "user-plus",
        }
    }

    /// Severity category for styling.
    pub const fn severity(self) -> &'static str {
        match self {
            Self::BookingConfirmed | Self::PaymentReceived => "success",
            Self::BookingCancelled | Self::MaintenanceAlert => "warning",
            Self::BookingReminder | Self::WaitlistOffer | Self::VisitorArrived => "info",
            Self::SystemAnnouncement => "neutral",
        }
    }
}

/// A notification center entry with typed metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CenterNotification {
    pub id: Uuid,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub read: bool,
    /// Optional deep-link target (e.g. `/bookings/abc-123`).
    pub action_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedNotifications {
    pub items: Vec<CenterNotificationResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub unread_count: usize,
}

/// Notification response DTO.
#[derive(Debug, Serialize)]
pub struct CenterNotificationResponse {
    pub id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub action_url: Option<String>,
    pub icon: String,
    pub severity: String,
    pub type_label: String,
    pub created_at: String,
    pub date_group: String,
}

impl CenterNotificationResponse {
    fn from_notification(n: &CenterNotification) -> Self {
        let today = Utc::now().date_naive();
        let notif_date = n.created_at.date_naive();
        let date_group = if notif_date == today {
            "today".to_string()
        } else if notif_date == today.pred_opt().unwrap_or(today) {
            "yesterday".to_string()
        } else {
            notif_date.format("%Y-%m-%d").to_string()
        };

        Self {
            id: n.id.to_string(),
            notification_type: n.notification_type,
            title: n.title.clone(),
            message: n.message.clone(),
            read: n.read,
            action_url: n.action_url.clone(),
            icon: n.notification_type.icon().to_string(),
            severity: n.notification_type.severity().to_string(),
            type_label: n.notification_type.display_name().to_string(),
            created_at: n.created_at.to_rfc3339(),
            date_group,
        }
    }
}

/// Query parameters for paginated notification list.
#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    /// Filter: `read`, `unread`, or `all` (default).
    pub filter: Option<String>,
    /// Filter by notification type.
    pub notification_type: Option<String>,
}

/// Unread count response.
#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/notifications/center` — paginated notification list with filtering.
#[utoipa::path(get, path = "/api/v1/notifications/center", tag = "Notification Center",
    summary = "List notifications (paginated)",
    description = "Returns paginated notifications for the authenticated user with optional read/unread filtering.",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<usize>, Query, description = "Page number (1-based)"),
        ("per_page" = Option<usize>, Query, description = "Items per page (default 20, max 100)"),
        ("filter" = Option<String>, Query, description = "Filter: read, unread, or all"),
        ("notification_type" = Option<String>, Query, description = "Filter by notification type"),
    ),
    responses((status = 200, description = "Paginated notifications"))
)]
pub async fn list_center_notifications(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ListNotificationsQuery>,
) -> (StatusCode, Json<ApiResponse<PaginatedNotifications>>) {
    let state_guard = state.read().await;
    let user_id = auth_user.user_id.to_string();

    // Load notifications from the existing notification store
    let all_notifications = match state_guard.db.list_notifications_by_user(&user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list notifications",
                )),
            );
        }
    };

    // Convert to center notifications with typed metadata
    let mut center_notifs: Vec<CenterNotification> = all_notifications
        .iter()
        .map(|n| {
            let notification_type = parse_notification_type(&n.title);
            CenterNotification {
                id: n.id,
                user_id: user_id.clone(),
                notification_type,
                title: n.title.clone(),
                message: n.message.clone(),
                read: n.read,
                action_url: extract_action_url(&n.title, &notification_type),
                created_at: n.created_at,
            }
        })
        .collect();

    // Sort newest first
    center_notifs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Apply read/unread filter
    let filter = query.filter.as_deref().unwrap_or("all");
    let filtered: Vec<&CenterNotification> = match filter {
        "read" => center_notifs.iter().filter(|n| n.read).collect(),
        "unread" => center_notifs.iter().filter(|n| !n.read).collect(),
        _ => center_notifs.iter().collect(),
    };

    // Apply type filter
    let filtered: Vec<&CenterNotification> = if let Some(ref type_filter) = query.notification_type
    {
        let parsed = parse_notification_type(type_filter);
        filtered
            .into_iter()
            .filter(|n| n.notification_type == parsed)
            .collect()
    } else {
        filtered
    };

    let total = filtered.len();
    let unread_count = center_notifs.iter().filter(|n| !n.read).count();

    // Pagination
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let skip = (page - 1) * per_page;

    let items: Vec<CenterNotificationResponse> = filtered
        .into_iter()
        .skip(skip)
        .take(per_page)
        .map(CenterNotificationResponse::from_notification)
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(PaginatedNotifications {
            items,
            total,
            page,
            per_page,
            unread_count,
        })),
    )
}

/// `GET /api/v1/notifications/unread-count` — badge count.
#[utoipa::path(get, path = "/api/v1/notifications/unread-count", tag = "Notification Center",
    summary = "Get unread notification count",
    description = "Returns the number of unread notifications for badge display.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Unread count"))
)]
pub async fn unread_count(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<UnreadCountResponse>>) {
    let state_guard = state.read().await;
    let user_id = auth_user.user_id.to_string();

    match state_guard.db.list_notifications_by_user(&user_id).await {
        Ok(notifications) => {
            let count = notifications.iter().filter(|n| !n.read).count();
            (
                StatusCode::OK,
                Json(ApiResponse::success(UnreadCountResponse { count })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to count unread notifications: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to count notifications",
                )),
            )
        }
    }
}

/// `DELETE /api/v1/notifications/:id` — delete a single notification.
#[utoipa::path(delete, path = "/api/v1/notifications/{id}", tag = "Notification Center",
    summary = "Delete a notification",
    description = "Permanently deletes a notification owned by the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Deleted"))
)]
pub async fn delete_notification(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    let user_id = auth_user.user_id.to_string();

    // Verify ownership
    let notifications = match state_guard.db.list_notifications_by_user(&user_id).await {
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

    match state_guard.db.delete_notification(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Notification not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete notification: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete notification",
                )),
            )
        }
    }
}

/// `PUT /api/v1/notifications/read-all` — mark all as read (enhanced version).
#[utoipa::path(put, path = "/api/v1/notifications/read-all", tag = "Notification Center",
    summary = "Mark all notifications as read",
    description = "Marks all unread notifications as read for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Count of notifications marked as read"))
)]
pub async fn mark_all_read(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<UnreadCountResponse>>) {
    let state_guard = state.read().await;
    let user_id = auth_user.user_id.to_string();

    let notifications = match state_guard.db.list_notifications_by_user(&user_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to mark all read",
                )),
            );
        }
    };

    let mut marked = 0usize;
    for notif in &notifications {
        if !notif.read {
            if let Ok(true) = state_guard
                .db
                .mark_notification_read(&notif.id.to_string())
                .await
            {
                marked += 1;
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(UnreadCountResponse { count: marked })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse notification type from title/category heuristic.
fn parse_notification_type(title: &str) -> NotificationType {
    let lower = title.to_lowercase();
    if lower.contains("confirmed") || lower.contains("booked") {
        NotificationType::BookingConfirmed
    } else if lower.contains("cancel") {
        NotificationType::BookingCancelled
    } else if lower.contains("reminder") {
        NotificationType::BookingReminder
    } else if lower.contains("waitlist") {
        NotificationType::WaitlistOffer
    } else if lower.contains("maintenance") {
        NotificationType::MaintenanceAlert
    } else if lower.contains("payment") || lower.contains("paid") {
        NotificationType::PaymentReceived
    } else if lower.contains("visitor") || lower.contains("arrived") {
        NotificationType::VisitorArrived
    } else {
        NotificationType::SystemAnnouncement
    }
}

/// Extract action URL based on notification type.
fn extract_action_url(_title: &str, notification_type: &NotificationType) -> Option<String> {
    match notification_type {
        NotificationType::BookingConfirmed
        | NotificationType::BookingCancelled
        | NotificationType::BookingReminder => Some("/bookings".to_string()),
        NotificationType::WaitlistOffer => Some("/waitlist".to_string()),
        NotificationType::MaintenanceAlert => Some("/admin/maintenance".to_string()),
        NotificationType::PaymentReceived => Some("/credits".to_string()),
        NotificationType::VisitorArrived => Some("/visitors".to_string()),
        NotificationType::SystemAnnouncement => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_display_names() {
        assert_eq!(
            NotificationType::BookingConfirmed.display_name(),
            "Booking Confirmed"
        );
        assert_eq!(
            NotificationType::BookingCancelled.display_name(),
            "Booking Cancelled"
        );
        assert_eq!(
            NotificationType::BookingReminder.display_name(),
            "Booking Reminder"
        );
        assert_eq!(
            NotificationType::WaitlistOffer.display_name(),
            "Waitlist Offer"
        );
        assert_eq!(
            NotificationType::MaintenanceAlert.display_name(),
            "Maintenance Alert"
        );
        assert_eq!(
            NotificationType::SystemAnnouncement.display_name(),
            "System Announcement"
        );
        assert_eq!(
            NotificationType::PaymentReceived.display_name(),
            "Payment Received"
        );
        assert_eq!(
            NotificationType::VisitorArrived.display_name(),
            "Visitor Arrived"
        );
    }

    #[test]
    fn test_notification_type_icons() {
        assert_eq!(NotificationType::BookingConfirmed.icon(), "check-circle");
        assert_eq!(NotificationType::MaintenanceAlert.icon(), "wrench");
        assert_eq!(NotificationType::VisitorArrived.icon(), "user-plus");
    }

    #[test]
    fn test_notification_type_severity() {
        assert_eq!(NotificationType::BookingConfirmed.severity(), "success");
        assert_eq!(NotificationType::BookingCancelled.severity(), "warning");
        assert_eq!(NotificationType::BookingReminder.severity(), "info");
        assert_eq!(NotificationType::SystemAnnouncement.severity(), "neutral");
    }

    #[test]
    fn test_notification_type_serialization() {
        let json = serde_json::to_string(&NotificationType::BookingConfirmed).unwrap();
        assert_eq!(json, "\"booking_confirmed\"");
        let de: NotificationType = serde_json::from_str("\"waitlist_offer\"").unwrap();
        assert_eq!(de, NotificationType::WaitlistOffer);
    }

    #[test]
    fn test_all_types_deserialize() {
        let types = [
            "booking_confirmed",
            "booking_cancelled",
            "booking_reminder",
            "waitlist_offer",
            "maintenance_alert",
            "system_announcement",
            "payment_received",
            "visitor_arrived",
        ];
        for t in types {
            let json = format!("\"{}\"", t);
            let _: NotificationType = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_parse_notification_type_booking() {
        assert_eq!(
            parse_notification_type("Booking Confirmed"),
            NotificationType::BookingConfirmed
        );
        assert_eq!(
            parse_notification_type("Your booking was cancelled"),
            NotificationType::BookingCancelled
        );
        assert_eq!(
            parse_notification_type("Reminder: parking tomorrow"),
            NotificationType::BookingReminder
        );
    }

    #[test]
    fn test_parse_notification_type_other() {
        assert_eq!(
            parse_notification_type("Waitlist offer available"),
            NotificationType::WaitlistOffer
        );
        assert_eq!(
            parse_notification_type("Maintenance scheduled"),
            NotificationType::MaintenanceAlert
        );
        assert_eq!(
            parse_notification_type("Payment received"),
            NotificationType::PaymentReceived
        );
        assert_eq!(
            parse_notification_type("Visitor arrived at lot"),
            NotificationType::VisitorArrived
        );
        assert_eq!(
            parse_notification_type("General update"),
            NotificationType::SystemAnnouncement
        );
    }

    #[test]
    fn test_extract_action_url() {
        assert_eq!(
            extract_action_url("test", &NotificationType::BookingConfirmed),
            Some("/bookings".to_string())
        );
        assert_eq!(
            extract_action_url("test", &NotificationType::WaitlistOffer),
            Some("/waitlist".to_string())
        );
        assert_eq!(
            extract_action_url("test", &NotificationType::SystemAnnouncement),
            None
        );
    }

    #[test]
    fn test_center_notification_response() {
        let notif = CenterNotification {
            id: Uuid::new_v4(),
            user_id: "user-1".to_string(),
            notification_type: NotificationType::BookingConfirmed,
            title: "Booking Confirmed".to_string(),
            message: "Your booking for Lot A is confirmed.".to_string(),
            read: false,
            action_url: Some("/bookings".to_string()),
            created_at: Utc::now(),
        };
        let resp = CenterNotificationResponse::from_notification(&notif);
        assert_eq!(resp.severity, "success");
        assert_eq!(resp.icon, "check-circle");
        assert!(!resp.read);
        assert_eq!(resp.date_group, "today");
    }

    #[test]
    fn test_center_notification_serialization() {
        let notif = CenterNotification {
            id: Uuid::new_v4(),
            user_id: "user-1".to_string(),
            notification_type: NotificationType::MaintenanceAlert,
            title: "Maintenance Alert".to_string(),
            message: "Lot B closed for maintenance.".to_string(),
            read: true,
            action_url: Some("/admin/maintenance".to_string()),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&notif).unwrap();
        let de: CenterNotification = serde_json::from_str(&json).unwrap();
        assert_eq!(de.notification_type, NotificationType::MaintenanceAlert);
        assert!(de.read);
    }

    #[test]
    fn test_unread_count_response_serialization() {
        let resp = UnreadCountResponse { count: 42 };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("42"));
    }

    #[test]
    fn test_paginated_notifications_serialization() {
        let paginated = PaginatedNotifications {
            items: vec![],
            total: 0,
            page: 1,
            per_page: 20,
            unread_count: 0,
        };
        let json = serde_json::to_string(&paginated).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"page\":1"));
    }
}
