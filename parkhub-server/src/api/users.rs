//! User profile handlers: profile CRUD, password change, GDPR export/delete,
//! calendar export, stats, and preferences.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::fmt::Write as _;
use uuid::Uuid;

use parkhub_common::models::UserPreferences;
use parkhub_common::{ApiResponse, User, UserRole};

use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState};

        (status = 200, description = "User profile"),
        (status = 404, description = "User not found")
    )
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
pub async fn get_current_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(mut user)) => {
            user.password_hash = String::new();
            (StatusCode::OK, Json(ApiResponse::success(user)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch current user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

/// Request body for updating the current user's profile
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateCurrentUserRequest {
    name: Option<String>,
    phone: Option<String>,
    picture: Option<String>,
}
        (status = 200, description = "Profile updated"),
        (status = 400, description = "Invalid input"),
        (status = 404, description = "User not found")
    )
)]
pub async fn update_current_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateCurrentUserRequest>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state_guard = state.read().await;

    let mut user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching user for update: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Apply only the fields provided in the request
    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(phone) = req.phone {
        user.phone = Some(phone);
    }
    if let Some(picture) = req.picture {
        // Validate picture URL: must be empty, or a well-formed http(s) URL
        // capped at 2048 characters to prevent abuse.
        if !picture.is_empty() {
            if picture.len() > 2048 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_INPUT",
                        "Picture URL must be at most 2048 characters",
                    )),
                );
            }
            if !picture.starts_with("https://") && !picture.starts_with("http://") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_INPUT",
                        "Picture must be a valid HTTP or HTTPS URL",
                    )),
                );
            }
        }
        user.picture = if picture.is_empty() {
            None
        } else {
            Some(picture)
        };
    }
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save user profile update: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update profile",
            )),
        );
    }

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(user.id, &user.username)
        .log();

    user.password_hash = String::new();
    (StatusCode::OK, Json(ApiResponse::success(user)))
}
)]
pub async fn get_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    // Verify caller is an admin before exposing arbitrary user records.
    let Ok(Some(caller)) = state.db.get_user(&auth_user.user_id.to_string()).await else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    match state.db.get_user(&id).await {
        Ok(Some(mut user)) => {
            user.password_hash = String::new();
            (StatusCode::OK, Json(ApiResponse::success(user)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}
)]
pub async fn gdpr_export_data(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state = state.read().await;
    let user_id = auth_user.user_id.to_string();

    let Ok(Some(user)) = state.db.get_user(&user_id).await else {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(&ApiResponse::<()>::error("NOT_FOUND", "User not found"))
                .unwrap_or_default(),
        );
    };

    let bookings = state
        .db
        .list_bookings_by_user(&user_id)
        .await
        .unwrap_or_default();
    let vehicles = state
        .db
        .list_vehicles_by_user(&user_id)
        .await
        .unwrap_or_default();

    let absences = state
        .db
        .list_absences_by_user(&user_id)
        .await
        .unwrap_or_default();
    let credit_transactions = state
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default();
    let notifications = state
        .db
        .list_notifications_by_user(&user_id)
        .await
        .unwrap_or_default();

    // Note: password_hash is intentionally excluded from GDPR exports.
    // Exporting a password hash would allow offline brute-force attacks
    // against the user's own credential — contrary to the spirit of Art. 15.
    let export = serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "gdpr_basis": "GDPR Art. 15 — Right of Access",
        "profile": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "name": user.name,
            "phone": user.phone,
            "role": user.role,
            "created_at": user.created_at,
            "last_login": user.last_login,
            "preferences": user.preferences,
        },
        "bookings": bookings,
        "vehicles": vehicles,
        "absences": absences,
        "credit_transactions": credit_transactions,
        "notifications": notifications,
    });

    let json_str = serde_json::to_string_pretty(&export).unwrap_or_default();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json_str,
    )
}
)]
pub async fn gdpr_delete_account(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let user_id = auth_user.user_id.to_string();
    let state_guard = state.read().await;

    // Capture username before anonymization scrubs it
    let username = state_guard
        .db
        .get_user(&user_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    match state_guard.db.anonymize_user(&user_id).await {
        Ok(true) => {
            AuditEntry::new(AuditEventType::UserDeleted)
                .user(auth_user.user_id, &username)
                .log();
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("GDPR anonymization failed for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to anonymize account",
                )),
            )
        }
    }
}

/// Request body for password change
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id))]
pub async fn change_password(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Validate new password length
    if req.new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "New password must be at least 8 characters",
            )),
        );
    }

    let state_guard = state.read().await;
    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
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

    // Verify current password
    if !verify_password(&req.current_password, &user.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error(
                "INVALID_PASSWORD",
                "Current password is incorrect",
            )),
        );
    }

    // Hash new password
    let new_hash = match hash_password_simple(&req.new_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Password hashing failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Update user
    let mut updated_user = user;
    updated_user.password_hash = new_hash;
    updated_user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&updated_user).await {
        tracing::error!("Failed to save user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update password",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}
)]
pub async fn user_calendar_ics(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;

    let bookings = match state_guard
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings for iCal: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Failed to generate calendar".to_string(),
            );
        }
    };

    let mut ical = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//ParkHub//EN\r\n");

    for b in &bookings {
        // Resolve lot name (best-effort)
        let lot_name = match state_guard.db.get_parking_lot(&b.lot_id.to_string()).await {
            Ok(Some(l)) => l.name,
            _ => "Unknown Lot".to_string(),
        };

        let dtstart = b.start_time.format("%Y%m%dT%H%M%SZ");
        let dtend = b.end_time.format("%Y%m%dT%H%M%SZ");

        ical.push_str("BEGIN:VEVENT\r\n");
        let _ = write!(ical, "UID:{}@parkhub\r\n", b.id);
        let _ = write!(ical, "DTSTART:{dtstart}\r\n");
        let _ = write!(ical, "DTEND:{dtend}\r\n");
        let _ = write!(
            ical,
            "SUMMARY:Parking - {} - Slot {}\r\n",
            lot_name, b.slot_number
        );
        ical.push_str("END:VEVENT\r\n");
    }

    ical.push_str("END:VCALENDAR\r\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/calendar; charset=utf-8")],
        ical,
    )
}
)]
pub async fn user_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    let uid = auth_user.user_id.to_string();

    let Ok(Some(user)) = state_guard.db.get_user(&uid).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        );
    };

    let bookings = state_guard
        .db
        .list_bookings_by_user(&uid)
        .await
        .unwrap_or_default();

    let total_bookings = bookings.len();
    let active_bookings = bookings
        .iter()
        .filter(|b| {
            matches!(
                b.status,
                BookingStatus::Confirmed | BookingStatus::Active | BookingStatus::Pending
            )
        })
        .count();
    let cancelled_bookings = bookings
        .iter()
        .filter(|b| b.status == BookingStatus::Cancelled)
        .count();

    // Sum credits spent from deduction transactions
    let total_credits_spent = state_guard
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|tx| tx.transaction_type == CreditTransactionType::Deduction)
        .map(|tx| i64::from(tx.amount.abs()))
        .sum::<i64>();

    // Find favorite lot by most bookings
    let favorite_lot = {
        let mut lot_counts: std::collections::HashMap<Uuid, usize> =
            std::collections::HashMap::new();
        for b in &bookings {
            *lot_counts.entry(b.lot_id).or_insert(0) += 1;
        }
        if let Some((&lot_id, _)) = lot_counts.iter().max_by_key(|(_, &c)| c) {
            state_guard
                .db
                .get_parking_lot(&lot_id.to_string())
                .await
                .ok()
                .flatten()
                .map_or_else(|| "Unknown".to_string(), |l| l.name)
        } else {
            "None".to_string()
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "total_bookings": total_bookings,
            "active_bookings": active_bookings,
            "cancelled_bookings": cancelled_bookings,
            "total_credits_spent": total_credits_spent,
            "favorite_lot": favorite_lot,
            "member_since": user.created_at,
        }))),
    )
}
)]
pub async fn get_user_preferences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;

    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "language": user.preferences.language,
                "theme": user.preferences.theme,
                "notifications_enabled": user.preferences.notifications_enabled,
                "email_reminders": user.preferences.email_reminders,
                "default_duration_minutes": user.preferences.default_duration_minutes,
            }))),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
    }
}

/// Request body for updating user preferences
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdatePreferencesRequest {
    language: Option<String>,
    theme: Option<String>,
    notifications_enabled: Option<bool>,
    email_reminders: Option<bool>,
    default_duration_minutes: Option<i32>,
}
)]
pub async fn update_user_preferences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;

    let Ok(Some(mut user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        );
    };

    if let Some(lang) = req.language {
        user.preferences.language = lang;
    }
    if let Some(theme) = req.theme {
        user.preferences.theme = theme;
    }
    if let Some(notif) = req.notifications_enabled {
        user.preferences.notifications_enabled = notif;
    }
    if let Some(email) = req.email_reminders {
        user.preferences.email_reminders = email;
    }
    if let Some(dur) = req.default_duration_minutes {
        user.preferences.default_duration_minutes = Some(dur);
    }
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save preferences: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save preferences",
            )),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "language": user.preferences.language,
            "theme": user.preferences.theme,
            "notifications_enabled": user.preferences.notifications_enabled,
            "email_reminders": user.preferences.email_reminders,
            "default_duration_minutes": user.preferences.default_duration_minutes,
        }))),
    )
}
