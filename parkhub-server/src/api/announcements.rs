//! Announcement handlers: public active list, admin CRUD.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::ApiResponse;
use parkhub_common::models::{Announcement, AnnouncementSeverity};

use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState, check_admin};

/// `GET /api/v1/announcements/active` — public, return active non-expired announcements
#[utoipa::path(get, path = "/api/v1/announcements/active", tag = "Public",
    summary = "Get active announcements",
    description = "Returns active non-expired announcements. No auth required.",
    responses((status = 200, description = "Success"))
)]
pub async fn get_active_announcements(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<Vec<Announcement>>>) {
    let state_guard = state.read().await;
    match state_guard.db.list_announcements().await {
        Ok(announcements) => {
            let now = Utc::now();
            let active: Vec<Announcement> = announcements
                .into_iter()
                .filter(|a| a.active && a.expires_at.is_none_or(|exp| exp > now))
                .collect();
            (StatusCode::OK, Json(ApiResponse::success(active)))
        }
        Err(e) => {
            tracing::error!("Failed to list announcements: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list announcements",
                )),
            )
        }
    }
}

/// `GET /api/v1/admin/announcements` — admin: list all announcements
#[utoipa::path(get, path = "/api/v1/admin/announcements", tag = "Admin",
    summary = "List all announcements (admin)",
    description = "Returns all announcements. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_list_announcements(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Announcement>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_announcements().await {
        Ok(announcements) => (StatusCode::OK, Json(ApiResponse::success(announcements))),
        Err(e) => {
            tracing::error!("Failed to list announcements: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list announcements",
                )),
            )
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAnnouncementRequest {
    title: String,
    message: String,
    severity: AnnouncementSeverity,
    active: Option<bool>,
    expires_at: Option<DateTime<Utc>>,
}

/// `POST /api/v1/admin/announcements` — admin: create announcement
#[utoipa::path(
    post,
    path = "/api/v1/admin/announcements",
    tag = "Admin",
    summary = "Create announcement",
    description = "Create a new system announcement. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_create_announcement(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateAnnouncementRequest>,
) -> (StatusCode, Json<ApiResponse<Announcement>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let announcement = Announcement {
        id: Uuid::new_v4(),
        title: req.title,
        message: req.message,
        severity: req.severity,
        active: req.active.unwrap_or(true),
        created_by: Some(auth_user.user_id),
        expires_at: req.expires_at,
        created_at: Utc::now(),
    };

    match state_guard.db.save_announcement(&announcement).await {
        Ok(()) => {
            let audit = AuditEntry::new(AuditEventType::ConfigChanged)
                .user(auth_user.user_id, "admin")
                .resource("announcement", &announcement.id.to_string())
                .details(serde_json::json!({ "action": "create", "title": &announcement.title }))
                .log();
            audit.persist(&state_guard.db).await;
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(announcement)),
            )
        }
        Err(e) => {
            tracing::error!("Failed to save announcement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to create announcement",
                )),
            )
        }
    }
}

/// Represents a field that can be absent, explicitly null, or a value.
/// This avoids `Option<Option<T>>` which clippy flags.
#[derive(Default)]
pub enum NullableField<T> {
    /// Field was not present in the request
    #[default]
    Absent,
    /// Field was explicitly set to null
    Null,
    /// Field was set to a value
    Value(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for NullableField<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|opt| opt.map_or(Self::Null, Self::Value))
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateAnnouncementRequest {
    title: Option<String>,
    message: Option<String>,
    severity: Option<AnnouncementSeverity>,
    active: Option<bool>,
    #[schema(value_type = Option<String>)]
    #[serde(default)]
    expires_at: NullableField<DateTime<Utc>>,
}

/// `PUT /api/v1/admin/announcements/{id}` — admin: update announcement
#[utoipa::path(
    put,
    path = "/api/v1/admin/announcements/{id}",
    tag = "Admin",
    summary = "Update announcement",
    description = "Update an existing announcement by ID. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_announcement(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> (StatusCode, Json<ApiResponse<Announcement>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Fetch all announcements and find by ID
    let announcements = match state_guard.db.list_announcements().await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to list announcements: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let Some(mut announcement) = announcements.into_iter().find(|a| a.id.to_string() == id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Announcement not found")),
        );
    };

    if let Some(title) = req.title {
        announcement.title = title;
    }
    if let Some(message) = req.message {
        announcement.message = message;
    }
    if let Some(severity) = req.severity {
        announcement.severity = severity;
    }
    if let Some(active) = req.active {
        announcement.active = active;
    }
    match req.expires_at {
        NullableField::Value(v) => announcement.expires_at = Some(v),
        NullableField::Null => announcement.expires_at = None,
        NullableField::Absent => {}
    }

    match state_guard.db.save_announcement(&announcement).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::success(announcement))),
        Err(e) => {
            tracing::error!("Failed to update announcement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to update announcement",
                )),
            )
        }
    }
}

/// `DELETE /api/v1/admin/announcements/{id}` — admin: delete announcement
#[utoipa::path(
    delete,
    path = "/api/v1/admin/announcements/{id}",
    tag = "Admin",
    summary = "Delete announcement",
    description = "Delete an announcement by ID. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_delete_announcement(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.delete_announcement(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Announcement not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete announcement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete announcement",
                )),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_announcement_request_full() {
        let json = r#"{
            "title":"Maintenance",
            "message":"Lot A closed on Monday",
            "severity":"warning",
            "active":true,
            "expires_at":"2026-04-01T00:00:00Z"
        }"#;
        let req: CreateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "Maintenance");
        assert_eq!(req.message, "Lot A closed on Monday");
        assert_eq!(req.active, Some(true));
        assert!(req.expires_at.is_some());
    }

    #[test]
    fn test_create_announcement_request_minimal() {
        let json = r#"{"title":"Info","message":"Welcome!","severity":"info"}"#;
        let req: CreateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert!(req.active.is_none());
        assert!(req.expires_at.is_none());
    }

    // ── NullableField deserialization ────────────────────────────────────────

    #[test]
    fn nullable_field_absent_when_key_missing() {
        let json = r#"{"title":"T","message":"M"}"#;
        let req: UpdateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req.expires_at, NullableField::Absent));
    }

    #[test]
    fn nullable_field_null_when_explicit_null() {
        let json = r#"{"title":"T","message":"M","expires_at":null}"#;
        let req: UpdateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req.expires_at, NullableField::Null));
    }

    #[test]
    fn nullable_field_value_when_present() {
        let json = r#"{"title":"T","message":"M","expires_at":"2026-05-01T12:00:00Z"}"#;
        let req: UpdateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req.expires_at, NullableField::Value(_)));
    }

    // ── Severity serde ──────────────────────────────────────────────────────

    #[test]
    fn all_severities_deserialize() {
        for sev in ["info", "warning", "error", "success"] {
            let json = format!(r#"{{"title":"T","message":"M","severity":"{sev}"}}"#);
            let req: CreateAnnouncementRequest = serde_json::from_str(&json).unwrap();
            match sev {
                "info" => assert_eq!(req.severity, AnnouncementSeverity::Info),
                "warning" => assert_eq!(req.severity, AnnouncementSeverity::Warning),
                "error" => assert_eq!(req.severity, AnnouncementSeverity::Error),
                "success" => assert_eq!(req.severity, AnnouncementSeverity::Success),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn invalid_severity_rejected() {
        let json = r#"{"title":"T","message":"M","severity":"critical"}"#;
        assert!(serde_json::from_str::<CreateAnnouncementRequest>(json).is_err());
    }

    // ── UpdateAnnouncementRequest partial updates ───────────────────────────

    #[test]
    fn update_request_all_fields() {
        let json = r#"{
            "title":"New Title",
            "message":"New Message",
            "severity":"error",
            "active":false,
            "expires_at":"2026-06-01T00:00:00Z"
        }"#;
        let req: UpdateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title.unwrap(), "New Title");
        assert_eq!(req.message.unwrap(), "New Message");
        assert_eq!(req.severity.unwrap(), AnnouncementSeverity::Error);
        assert_eq!(req.active, Some(false));
    }

    #[test]
    fn update_request_empty_body() {
        let json = r#"{}"#;
        let req: UpdateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert!(req.title.is_none());
        assert!(req.message.is_none());
        assert!(req.severity.is_none());
        assert!(req.active.is_none());
        assert!(matches!(req.expires_at, NullableField::Absent));
    }

    #[test]
    fn nullable_field_default_is_absent() {
        let field: NullableField<String> = NullableField::default();
        assert!(matches!(field, NullableField::Absent));
    }
}
