//! Setup wizard — initial deployment configuration via API.
//!
//! These endpoints are only accessible before setup is completed.
//! Once setup is done, they return 400 and are effectively disabled.

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use parkhub_common::models::UserPreferences;
use parkhub_common::{ApiResponse, User, UserRole};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

#[allow(clippy::struct_excessive_bools)]
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SetupStatus {
    /// Whether initial setup has been completed
    setup_completed: bool,
    /// Whether at least one admin user exists
    has_admin: bool,
    /// Whether at least one parking lot exists
    has_parking_lots: bool,
    /// Whether any users exist
    has_users: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetupRequest {
    /// Organization / company name
    pub company_name: String,
    /// Admin username (min 3 characters)
    pub admin_username: String,
    /// Admin password (min 8 characters)
    pub admin_password: String,
    /// Admin email address
    pub admin_email: String,
    /// Admin full name
    pub admin_name: String,
    /// Use case (e.g. "corporate", "residential")
    #[serde(default)]
    pub use_case: Option<String>,
    /// Whether to create sample parking lot data
    #[serde(default)]
    pub create_sample_data: bool,
}

/// `GET /api/v1/setup/status` — check if initial setup is completed
#[utoipa::path(
    get,
    path = "/api/v1/setup/status",
    tag = "Setup",
    summary = "Get setup status",
    description = "Check whether initial setup has been completed. Public endpoint.",
    responses(
        (status = 200, description = "Setup status"),
    )
)]
pub async fn setup_status(State(state): State<SharedState>) -> Json<ApiResponse<SetupStatus>> {
    let guard = state.read().await;
    let is_fresh = guard.db.is_fresh().await.unwrap_or(true);
    let db_stats = guard.db.stats().await.unwrap_or_default();
    drop(guard);

    Json(ApiResponse::success(SetupStatus {
        setup_completed: !is_fresh,
        has_admin: db_stats.users > 0,
        has_parking_lots: db_stats.parking_lots > 0,
        has_users: db_stats.users > 0,
    }))
}

/// `POST /api/v1/setup` — initial setup: create admin user and configure system
#[utoipa::path(
    post,
    path = "/api/v1/setup",
    tag = "Setup",
    summary = "Run initial setup",
    description = "Create the first admin user and configure the system. Only works before setup is completed.",
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Setup completed successfully"),
        (status = 400, description = "Setup already completed or validation error"),
    )
)]
#[allow(clippy::too_many_lines)]
pub async fn setup_init(
    State(state): State<SharedState>,
    Json(req): Json<SetupRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.write().await;

    // Guard: only allow setup once
    if !state_guard.db.is_fresh().await.unwrap_or(false) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "SETUP_COMPLETED",
                "Setup already completed",
            )),
        );
    }

    // Validate inputs
    if req.admin_username.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "Username must be at least 3 characters",
            )),
        );
    }
    if req.admin_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "Password must be at least 8 characters",
            )),
        );
    }

    // Hash the password
    let password_hash = match crate::hash_password(&req.admin_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to hash password",
                )),
            );
        }
    };

    // Create admin user
    let admin = User {
        id: Uuid::new_v4(),
        username: req.admin_username.clone(),
        email: req.admin_email.clone(),
        name: req.admin_name.clone(),
        password_hash,
        role: UserRole::Admin,
        is_active: true,
        phone: None,
        picture: None,
        preferences: UserPreferences {
            language: "en".to_string(),
            theme: "system".to_string(),
            notifications_enabled: true,
            email_reminders: false,
            default_duration_minutes: None,
            favorite_slots: Vec::new(),
        },
        credits_balance: 0,
        credits_monthly_quota: 0,
        credits_last_refilled: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_login: None,
        tenant_id: None,
        accessibility_needs: None,
        cost_center: None,
        department: None,
    };

    if let Err(e) = state_guard.db.save_user(&admin).await {
        tracing::error!("Failed to create admin user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create admin user",
            )),
        );
    }

    // Save settings
    let _ = state_guard
        .db
        .set_setting("company_name", &req.company_name)
        .await;
    let _ = state_guard
        .db
        .set_setting("use_case", req.use_case.as_deref().unwrap_or("corporate"))
        .await;
    let _ = state_guard.db.set_setting("credits_enabled", "true").await;
    let _ = state_guard.db.set_setting("credits_per_booking", "1").await;

    // Create sample data if requested
    if req.create_sample_data
        && let Err(e) = crate::create_sample_parking_lot(&state_guard.db).await {
            tracing::warn!("Failed to create sample data: {}", e);
        }

    // Mark setup as completed
    if let Err(e) = state_guard.db.mark_setup_completed().await {
        tracing::error!("Failed to mark setup completed: {}", e);
    }

    // Create a session token for the new admin
    let session = crate::db::Session::new(
        admin.id,
        24, // 24-hour session
        &admin.username,
        &format!("{:?}", admin.role).to_lowercase(),
    );
    let token = session.refresh_token.clone();
    if let Err(e) = state_guard.db.save_session(&token, &session).await {
        tracing::error!("Failed to create admin session: {}", e);
    }
    drop(state_guard);

    tracing::info!(
        "Setup completed: admin user '{}' created",
        req.admin_username
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "message": "Setup completed successfully",
            "tokens": {
                "access_token": token,
                "token_type": "Bearer",
            },
        }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_request_deserialize_full() {
        let json = r#"{
            "company_name": "ParkCorp",
            "admin_username": "admin",
            "admin_password": "Secure123!",
            "admin_email": "admin@parkcorp.de",
            "admin_name": "Admin User",
            "use_case": "corporate",
            "create_sample_data": true
        }"#;
        let req: SetupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.company_name, "ParkCorp");
        assert_eq!(req.admin_username, "admin");
        assert_eq!(req.admin_password, "Secure123!");
        assert_eq!(req.admin_email, "admin@parkcorp.de");
        assert_eq!(req.admin_name, "Admin User");
        assert_eq!(req.use_case.as_deref(), Some("corporate"));
        assert!(req.create_sample_data);
    }

    #[test]
    fn test_setup_request_deserialize_minimal() {
        let json = r#"{
            "company_name": "Mini",
            "admin_username": "root",
            "admin_password": "P@ssw0rd!",
            "admin_email": "root@mini.com",
            "admin_name": "Root"
        }"#;
        let req: SetupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.company_name, "Mini");
        assert!(req.use_case.is_none());
        assert!(!req.create_sample_data);
    }

    #[test]
    fn test_setup_request_missing_required_field() {
        let json = r#"{"company_name": "Test"}"#;
        let result: Result<SetupRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_setup_status_serialize() {
        let status = SetupStatus {
            setup_completed: false,
            has_admin: false,
            has_parking_lots: false,
            has_users: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["setup_completed"], false);
        assert_eq!(value["has_admin"], false);
        assert_eq!(value["has_parking_lots"], false);
        assert_eq!(value["has_users"], false);
    }

    #[test]
    fn test_setup_status_serialize_completed() {
        let status = SetupStatus {
            setup_completed: true,
            has_admin: true,
            has_parking_lots: true,
            has_users: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["setup_completed"], true);
        assert_eq!(value["has_admin"], true);
    }

    #[test]
    fn test_setup_request_create_sample_data_default() {
        let json = r#"{
            "company_name": "C",
            "admin_username": "usr",
            "admin_password": "pass1234",
            "admin_email": "a@b.com",
            "admin_name": "A"
        }"#;
        let req: SetupRequest = serde_json::from_str(json).unwrap();
        assert!(
            !req.create_sample_data,
            "create_sample_data should default to false"
        );
    }
}
