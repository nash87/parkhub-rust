//! System information and maintenance endpoints.
//!
//! Provides version info, maintenance mode status, and update-check stub.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use parkhub_common::ApiResponse;

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// Response for `GET /api/v1/system/version`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SystemVersionResponse {
    /// Crate version from Cargo.toml
    pub version: &'static str,
    /// Rust toolchain used to compile
    pub rust_version: &'static str,
    /// Git commit hash (set at build time, may be empty in dev builds)
    pub git_hash: &'static str,
}

/// Response for `GET /api/v1/system/maintenance`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MaintenanceStatusResponse {
    /// Whether the system is in maintenance mode
    pub maintenance_mode: bool,
    /// Optional message shown to users during maintenance
    pub message: Option<String>,
}

/// Response for `GET /api/v1/update/check`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UpdateCheckResponse {
    /// Currently running version
    pub current_version: &'static str,
    /// Whether an update is available (always false for self-hosted)
    pub update_available: bool,
    /// Latest known version (same as current for self-hosted)
    pub latest_version: &'static str,
}

/// `GET /api/v1/system/version` — current server version
#[utoipa::path(
    get,
    path = "/api/v1/system/version",
    tag = "System",
    summary = "Get server version",
    description = "Returns the current server version, Rust toolchain, and build info.",
    responses(
        (status = 200, description = "Version info"),
    )
)]
pub async fn system_version() -> Json<ApiResponse<SystemVersionResponse>> {
    Json(ApiResponse::success(SystemVersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        rust_version: option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("unknown"),
        git_hash: option_env!("GIT_HASH").unwrap_or("dev"),
    }))
}

/// `GET /api/v1/system/maintenance` — maintenance mode status
#[utoipa::path(
    get,
    path = "/api/v1/system/maintenance",
    tag = "System",
    summary = "Get maintenance mode status",
    description = "Returns whether the system is currently in maintenance mode.",
    responses(
        (status = 200, description = "Maintenance status"),
    )
)]
pub async fn system_maintenance(
    State(state): State<SharedState>,
) -> Json<ApiResponse<MaintenanceStatusResponse>> {
    let guard = state.read().await;
    let maintenance_mode = guard
        .db
        .get_setting("maintenance_mode")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    let message = guard
        .db
        .get_setting("maintenance_message")
        .await
        .ok()
        .flatten();
    drop(guard);

    Json(ApiResponse::success(MaintenanceStatusResponse {
        maintenance_mode,
        message,
    }))
}

/// `GET /api/v1/update/check` — check for available updates
#[utoipa::path(
    get,
    path = "/api/v1/update/check",
    tag = "System",
    summary = "Check for updates",
    description = "Returns whether a newer version is available. Self-hosted instances always report up-to-date.",
    responses(
        (status = 200, description = "Update check result"),
    )
)]
pub async fn update_check() -> Json<ApiResponse<UpdateCheckResponse>> {
    Json(ApiResponse::success(UpdateCheckResponse {
        current_version: env!("CARGO_PKG_VERSION"),
        update_available: false,
        latest_version: env!("CARGO_PKG_VERSION"),
    }))
}

/// `POST /api/v1/setup/complete` — mark initial setup as done
///
/// This is an alias / explicit endpoint that simply marks setup completed.
/// It mirrors the PHP `POST /api/v1/setup/complete` contract.
#[utoipa::path(
    post,
    path = "/api/v1/setup/complete",
    tag = "Setup",
    summary = "Mark setup as complete",
    description = "Explicitly marks the initial setup wizard as completed. Only works when setup is still pending.",
    responses(
        (status = 200, description = "Setup marked as complete"),
        (status = 400, description = "Setup already completed"),
    )
)]
pub async fn setup_complete(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let guard = state.write().await;

    if !guard.db.is_fresh().await.unwrap_or(false) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "SETUP_COMPLETED",
                "Setup already completed",
            )),
        );
    }

    if let Err(e) = guard.db.mark_setup_completed().await {
        tracing::error!("Failed to mark setup completed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to complete setup",
            )),
        );
    }
    drop(guard);

    tracing::info!("Setup explicitly marked as completed via /setup/complete");

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "message": "Setup completed successfully"
        }))),
    )
}

/// `POST /api/v1/setup/change-password` — change the admin password during setup
///
/// Works only before setup is fully completed, providing a way to change
/// the initial admin password without full auth being active.
#[utoipa::path(
    post,
    path = "/api/v1/setup/change-password",
    tag = "Setup",
    summary = "Change admin password during setup",
    description = "Change the first admin user's password during the setup wizard, before authentication is fully active.",
    request_body = SetupChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed"),
        (status = 400, description = "Setup already completed or validation error"),
        (status = 404, description = "No admin user found"),
    )
)]
pub async fn setup_change_password(
    State(state): State<SharedState>,
    Json(req): Json<SetupChangePasswordRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let guard = state.write().await;

    // Only allow during setup phase
    if !guard.db.is_fresh().await.unwrap_or(false) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "SETUP_COMPLETED",
                "This endpoint is only available during initial setup",
            )),
        );
    }

    if req.new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "Password must be at least 8 characters",
            )),
        );
    }

    // Find the first admin user
    let users = match guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let mut admin = match users
        .into_iter()
        .find(|u| u.role == parkhub_common::UserRole::Admin || u.role == parkhub_common::UserRole::SuperAdmin)
    {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "NOT_FOUND",
                    "No admin user found. Run setup first.",
                )),
            );
        }
    };

    let password_hash = match crate::hash_password(&req.new_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to hash password")),
            );
        }
    };

    admin.password_hash = password_hash;
    admin.updated_at = chrono::Utc::now();

    if let Err(e) = guard.db.save_user(&admin).await {
        tracing::error!("Failed to update admin password: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update password",
            )),
        );
    }
    drop(guard);

    tracing::info!("Admin password changed during setup");

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "message": "Password changed successfully"
        }))),
    )
}

/// Request body for `POST /api/v1/setup/change-password`.
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct SetupChangePasswordRequest {
    /// New password (min 8 characters)
    pub new_password: String,
}

/// `PATCH /api/v1/admin/slots/{id}` — admin update a slot by ID
///
/// Allows an admin to update a parking slot directly by slot ID, without
/// specifying the parent lot. This mirrors the PHP admin endpoint.
#[utoipa::path(
    patch,
    path = "/api/v1/admin/slots/{id}",
    tag = "Admin",
    summary = "Admin update slot",
    description = "Update a parking slot by ID. Admin only. Supports partial updates to status, slot_type, and slot_number.",
    params(("id" = String, Path, description = "Slot UUID")),
    responses(
        (status = 200, description = "Slot updated"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Slot not found"),
    )
)]
pub async fn admin_update_slot(
    State(state): State<SharedState>,
    axum::extract::Extension(auth_user): axum::extract::Extension<super::AuthUser>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    use parkhub_common::models::{SlotStatus, SlotType};

    let guard = state.read().await;

    // Admin check
    if let Err((_code, msg)) = super::check_admin(&guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", msg)),
        );
    }

    let mut slot = match guard.db.get_parking_slot(&id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Slot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Failed to get slot: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if let Some(status) = req.get("status").and_then(|v| v.as_str()) {
        slot.status = match status {
            "available" => SlotStatus::Available,
            "occupied" => SlotStatus::Occupied,
            "reserved" => SlotStatus::Reserved,
            "maintenance" => SlotStatus::Maintenance,
            "disabled" => SlotStatus::Disabled,
            _ => slot.status,
        };
    }

    if let Some(slot_type) = req.get("slot_type").and_then(|v| v.as_str()) {
        slot.slot_type = match slot_type {
            "compact" => SlotType::Compact,
            "large" => SlotType::Large,
            "handicap" => SlotType::Handicap,
            "electric" => SlotType::Electric,
            "motorcycle" => SlotType::Motorcycle,
            "vip" => SlotType::Vip,
            _ => SlotType::Standard,
        };
    }

    if let Some(number) = req.get("slot_number").and_then(serde_json::Value::as_i64) {
        #[allow(clippy::cast_possible_truncation)]
        let num = number as i32;
        slot.slot_number = num;
    }

    if let Err(e) = guard.db.save_parking_slot(&slot).await {
        tracing::error!("Failed to update slot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update slot")),
        );
    }
    drop(guard);

    tracing::info!("Admin updated slot {}", id);

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "id": slot.id.to_string(),
            "slot_number": slot.slot_number,
            "message": "Slot updated"
        }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_version_response_serialize() {
        let resp = SystemVersionResponse {
            version: "1.0.0",
            rust_version: "1.78.0",
            git_hash: "abc1234",
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["rust_version"], "1.78.0");
        assert_eq!(json["git_hash"], "abc1234");
    }

    #[test]
    fn test_maintenance_status_serialize() {
        let resp = MaintenanceStatusResponse {
            maintenance_mode: true,
            message: Some("Scheduled downtime".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["maintenance_mode"], true);
        assert_eq!(json["message"], "Scheduled downtime");
    }

    #[test]
    fn test_maintenance_status_no_message() {
        let resp = MaintenanceStatusResponse {
            maintenance_mode: false,
            message: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["maintenance_mode"], false);
        assert!(json["message"].is_null());
    }

    #[test]
    fn test_update_check_response_serialize() {
        let resp = UpdateCheckResponse {
            current_version: "1.9.0",
            update_available: false,
            latest_version: "1.9.0",
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["update_available"], false);
        assert_eq!(json["current_version"], "1.9.0");
    }

    #[test]
    fn test_setup_change_password_request_deserialize() {
        let json = r#"{"new_password": "SecurePass123!"}"#;
        let req: SetupChangePasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.new_password, "SecurePass123!");
    }

    #[test]
    fn test_admin_slot_update_partial_json() {
        let json = r#"{"status": "maintenance"}"#;
        let val: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(val["status"], "maintenance");
        assert!(val.get("slot_type").is_none());
    }

    #[test]
    fn test_admin_slot_update_full_json() {
        let json = r#"{"status": "disabled", "slot_type": "electric", "slot_number": 42}"#;
        let val: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(val["status"], "disabled");
        assert_eq!(val["slot_type"], "electric");
        assert_eq!(val["slot_number"], 42);
    }
}
