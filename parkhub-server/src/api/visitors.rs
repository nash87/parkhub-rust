//! Visitor pre-registration handlers.
//!
//! Allows users to pre-register visitors with name, email, vehicle plate,
//! and visit date. Generates QR codes and sends email notifications.

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::ApiResponse;
use parkhub_common::models::{Visitor, VisitorStatus};

use super::settings::read_admin_setting;
use super::{AuthUser, SharedState, check_admin};

/// Request body for pre-registering a visitor
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterVisitorRequest {
    pub name: String,
    pub email: String,
    pub vehicle_plate: Option<String>,
    pub visit_date: DateTime<Utc>,
    pub purpose: Option<String>,
}

/// Query params for admin visitor listing
#[derive(Debug, Deserialize, Default)]
pub struct VisitorQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    #[allow(dead_code)]
    pub from_date: Option<String>,
    #[allow(dead_code)]
    pub to_date: Option<String>,
}

/// Generate a visitor pass QR code URL
fn generate_visitor_pass_url(visitor_id: &Uuid) -> String {
    format!("/visitor-pass/{}", visitor_id)
}

/// Generate QR code data as base64-encoded PNG
fn generate_qr_base64(data: &str) -> String {
    use base64::Engine;
    use image::Luma;
    use qrcode::QrCode;

    let code = match QrCode::new(data.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let img = code.render::<Luma<u8>>().quiet_zone(true).build();
    let mut buf = std::io::Cursor::new(Vec::new());
    if img.write_to(&mut buf, image::ImageFormat::Png).is_err() {
        return String::new();
    }
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    )
}

/// `POST /api/v1/visitors/register` — pre-register a visitor
#[utoipa::path(
    post,
    path = "/api/v1/visitors/register",
    tag = "Visitors",
    summary = "Register a visitor",
    description = "Pre-register a visitor with name, email, vehicle plate, and visit date.",
    security(("bearer_auth" = []))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, visitor_name = %req.name))]
pub async fn register_visitor(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<RegisterVisitorRequest>,
) -> (StatusCode, Json<ApiResponse<Visitor>>) {
    let state_guard = state.read().await;

    // Check feature flag
    let enabled = read_admin_setting(&state_guard.db, "visitors_enabled").await;
    if enabled != "true" {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "VISITORS_DISABLED",
                "Visitor pre-registration is not enabled",
            )),
        );
    }

    // Validate required fields
    if req.name.trim().is_empty() || req.email.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "Name and email are required",
            )),
        );
    }

    let visitor_id = Uuid::new_v4();
    let pass_url = generate_visitor_pass_url(&visitor_id);
    let qr_code = generate_qr_base64(&pass_url);

    let visitor = Visitor {
        id: visitor_id,
        host_user_id: auth_user.user_id,
        name: req.name,
        email: req.email,
        vehicle_plate: req.vehicle_plate,
        visit_date: req.visit_date,
        purpose: req.purpose,
        status: VisitorStatus::Pending,
        qr_code: Some(qr_code),
        pass_url: Some(pass_url),
        checked_in_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_visitor(&visitor).await {
        tracing::error!("Failed to save visitor: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to register visitor",
            )),
        );
    }

    // Log visitor notification (email would be sent in production)
    tracing::info!(
        visitor_id = %visitor.id,
        visitor_email = %visitor.email,
        "Visitor registered — notification would be sent"
    );

    (StatusCode::CREATED, Json(ApiResponse::success(visitor)))
}

/// `GET /api/v1/visitors` — list my registered visitors
#[utoipa::path(
    get,
    path = "/api/v1/visitors",
    tag = "Visitors",
    summary = "List my visitors",
    description = "List all visitors registered by the current user.",
    security(("bearer_auth" = []))
)]
pub async fn list_my_visitors(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Visitor>>>) {
    let state_guard = state.read().await;

    match state_guard
        .db
        .list_visitors_by_host(&auth_user.user_id.to_string())
        .await
    {
        Ok(visitors) => (StatusCode::OK, Json(ApiResponse::success(visitors))),
        Err(e) => {
            tracing::error!("Failed to list visitors: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list visitors",
                )),
            )
        }
    }
}

/// `GET /api/v1/admin/visitors` — admin: list all visitors
#[utoipa::path(
    get,
    path = "/api/v1/admin/visitors",
    tag = "Admin",
    summary = "List all visitors",
    description = "List all visitors across all hosts. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_list_visitors(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<VisitorQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Visitor>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_all_visitors().await {
        Ok(mut visitors) => {
            // Apply filters
            if let Some(ref status_filter) = query.status {
                visitors.retain(|v| {
                    serde_json::to_string(&v.status)
                        .unwrap_or_default()
                        .contains(status_filter)
                });
            }
            if let Some(ref search) = query.search {
                let q = search.to_lowercase();
                visitors.retain(|v| {
                    v.name.to_lowercase().contains(&q) || v.email.to_lowercase().contains(&q)
                });
            }
            (StatusCode::OK, Json(ApiResponse::success(visitors)))
        }
        Err(e) => {
            tracing::error!("Failed to list all visitors: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list visitors",
                )),
            )
        }
    }
}

/// `PUT /api/v1/visitors/:id/check-in` — mark visitor as checked in
#[utoipa::path(
    put,
    path = "/api/v1/visitors/{id}/check-in",
    tag = "Visitors",
    summary = "Check in a visitor",
    description = "Mark a visitor as checked in.",
    security(("bearer_auth" = []))
)]
pub async fn check_in_visitor(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Visitor>>) {
    let state_guard = state.read().await;

    let mut visitor = match state_guard.db.get_visitor(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Visitor not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Only the host or an admin can check in a visitor
    if visitor.host_user_id != auth_user.user_id {
        if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
            return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
        }
    }

    if visitor.status == VisitorStatus::CheckedIn {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "ALREADY_CHECKED_IN",
                "Visitor is already checked in",
            )),
        );
    }

    visitor.status = VisitorStatus::CheckedIn;
    visitor.checked_in_at = Some(Utc::now());
    visitor.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_visitor(&visitor).await {
        tracing::error!("Failed to update visitor: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to check in visitor",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(visitor)))
}

/// `DELETE /api/v1/visitors/:id` — cancel visitor registration
#[utoipa::path(
    delete,
    path = "/api/v1/visitors/{id}",
    tag = "Visitors",
    summary = "Cancel visitor registration",
    description = "Cancel a visitor registration. Only the host or admin can cancel.",
    security(("bearer_auth" = []))
)]
pub async fn cancel_visitor(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Visitor>>) {
    let state_guard = state.read().await;

    let mut visitor = match state_guard.db.get_visitor(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Visitor not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Only the host or an admin can cancel
    if visitor.host_user_id != auth_user.user_id {
        if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
            return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
        }
    }

    visitor.status = VisitorStatus::Cancelled;
    visitor.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_visitor(&visitor).await {
        tracing::error!("Failed to cancel visitor: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel visitor",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(visitor)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_visitor_request_deserialize() {
        let json = r#"{
            "name":"Alice Smith",
            "email":"alice@example.com",
            "vehicle_plate":"ABC-123",
            "visit_date":"2026-04-15T09:00:00Z",
            "purpose":"Business meeting"
        }"#;
        let req: RegisterVisitorRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Alice Smith");
        assert_eq!(req.email, "alice@example.com");
        assert_eq!(req.vehicle_plate.as_deref(), Some("ABC-123"));
        assert_eq!(req.purpose.as_deref(), Some("Business meeting"));
    }

    #[test]
    fn test_register_visitor_request_minimal() {
        let json = r#"{
            "name":"Bob",
            "email":"bob@example.com",
            "visit_date":"2026-04-15T09:00:00Z"
        }"#;
        let req: RegisterVisitorRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Bob");
        assert!(req.vehicle_plate.is_none());
        assert!(req.purpose.is_none());
    }

    #[test]
    fn test_generate_visitor_pass_url() {
        let id = Uuid::new_v4();
        let url = generate_visitor_pass_url(&id);
        assert!(url.starts_with("/visitor-pass/"));
        assert!(url.contains(&id.to_string()));
    }

    #[test]
    fn test_generate_qr_base64_nonempty() {
        let qr = generate_qr_base64("https://parkhub.example.com/pass/123");
        assert!(qr.starts_with("data:image/png;base64,"));
        assert!(qr.len() > 50);
    }

    #[test]
    fn test_visitor_status_serialization() {
        assert_eq!(
            serde_json::to_string(&VisitorStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&VisitorStatus::CheckedIn).unwrap(),
            "\"checked_in\""
        );
        assert_eq!(
            serde_json::to_string(&VisitorStatus::Expired).unwrap(),
            "\"expired\""
        );
        assert_eq!(
            serde_json::to_string(&VisitorStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn test_visitor_model_roundtrip() {
        let visitor = Visitor {
            id: Uuid::new_v4(),
            host_user_id: Uuid::new_v4(),
            name: "Test Visitor".to_string(),
            email: "test@example.com".to_string(),
            vehicle_plate: Some("XY-999".to_string()),
            visit_date: Utc::now(),
            purpose: Some("Interview".to_string()),
            status: VisitorStatus::Pending,
            qr_code: Some("data:image/png;base64,abc".to_string()),
            pass_url: Some("/visitor-pass/123".to_string()),
            checked_in_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&visitor).unwrap();
        let back: Visitor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Test Visitor");
        assert_eq!(back.email, "test@example.com");
        assert_eq!(back.status, VisitorStatus::Pending);
    }

    #[test]
    fn test_visitor_query_defaults() {
        let q = VisitorQuery::default();
        assert!(q.status.is_none());
        assert!(q.search.is_none());
        assert!(q.from_date.is_none());
        assert!(q.to_date.is_none());
    }

    #[test]
    fn test_visitor_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<VisitorStatus>("\"pending\"").unwrap(),
            VisitorStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<VisitorStatus>("\"checked_in\"").unwrap(),
            VisitorStatus::CheckedIn
        );
        assert_eq!(
            serde_json::from_str::<VisitorStatus>("\"cancelled\"").unwrap(),
            VisitorStatus::Cancelled
        );
    }
}
