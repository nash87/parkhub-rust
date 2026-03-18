//! Setup wizard — initial deployment configuration via API.
//!
//! These endpoints are only accessible before setup is completed.
//! Once setup is done, they return 400 and are effectively disabled.

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use parkhub_common::models::UserPreferences;
use parkhub_common::{ApiResponse, User, UserRole};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

#[derive(serde::Serialize)]
pub struct SetupStatus {
    setup_completed: bool,
    has_admin: bool,
    has_parking_lots: bool,
    has_users: bool,
}

#[derive(Deserialize)]
pub struct SetupRequest {
    pub company_name: String,
    pub admin_username: String,
    pub admin_password: String,
    pub admin_email: String,
    pub admin_name: String,
    #[serde(default)]
    pub use_case: Option<String>,
    #[serde(default)]
    pub create_sample_data: bool,
}

/// `GET /api/v1/setup/status` — check if initial setup is completed
pub async fn setup_status(State(state): State<SharedState>) -> Json<ApiResponse<SetupStatus>> {
    let state = state.read().await;
    let is_fresh = state.db.is_fresh().await.unwrap_or(true);
    let stats = state.db.stats().await.unwrap_or_default();

    Json(ApiResponse::success(SetupStatus {
        setup_completed: !is_fresh,
        has_admin: stats.users > 0,
        has_parking_lots: stats.parking_lots > 0,
        has_users: stats.users > 0,
    }))
}

/// `POST /api/v1/setup` — initial setup: create admin user and configure system
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
    if req.create_sample_data {
        if let Err(e) = crate::create_sample_parking_lot(&state_guard.db).await {
            tracing::warn!("Failed to create sample data: {}", e);
        }
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
