//! Admin handlers: user management, booking management, stats, reports, audit,
//! settings (auto-release, email, privacy, database reset), user updates.
//!
//! Extracted from mod.rs — Phase 3 API extraction.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{Datelike, TimeDelta, Timelike, Utc};
use parkhub_common::{ApiResponse, BookingStatus, User, UserRole};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audit::{AuditEntry, AuditEventType};
use crate::AppState;

use super::admin::AdminUserResponse;
use super::{check_admin, read_admin_setting, AuthUser};

type SharedState = Arc<RwLock<AppState>>;

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN — USER MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for updating a user's role
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRoleRequest {
    role: String,
}

/// Request body for updating a user's status
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserStatusRequest {
    status: String,
}

/// `GET /api/v1/admin/users` — list all users (admin only)
#[utoipa::path(get, path = "/api/v1/admin/users", tag = "Admin",
    summary = "List all users (admin)", description = "Returns all registered users. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "User list"), (status = 403, description = "Forbidden"))
)]
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_list_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminUserResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_users().await {
        Ok(users) => {
            tracing::debug!(count = users.len(), "Admin listed users");
            let response: Vec<AdminUserResponse> =
                users.iter().map(AdminUserResponse::from).collect();
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list users")),
            )
        }
    }
}

/// `PATCH /api/v1/admin/users/{id}/role` — update a user's role (admin only)
#[utoipa::path(patch, path = "/api/v1/admin/users/{id}/role", tag = "Admin",
    summary = "Update user role (admin)", description = "Changes a user's role. Prevents privilege escalation.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Role updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state, req), fields(admin_id = %auth_user.user_id, target_user_id = %id, new_role = %req.role))]
pub async fn admin_update_user_role(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Fetch the caller to check their role for privilege escalation prevention
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };

    // Only SuperAdmin may promote users to SuperAdmin (prevent privilege escalation)
    if req.role.as_str() == "superadmin" && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "FORBIDDEN",
                "Only a SuperAdmin can assign the SuperAdmin role",
            )),
        );
    }

    let mut user = match state_guard.db.get_user(&id).await {
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

    // Parse role string
    user.role = match req.role.as_str() {
        "admin" => UserRole::Admin,
        "superadmin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    };
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user role: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    let admin_username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    AuditEntry::new(AuditEventType::RoleChanged)
        .user(auth_user.user_id, &admin_username)
        .resource("user", &id)
        .log();

    tracing::info!(
        admin_id = %auth_user.user_id,
        target_user_id = %id,
        new_role = %req.role,
        "Admin updated user role"
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminUserResponse::from(&user))),
    )
}

/// `PATCH /api/v1/admin/users/{id}/status` — enable or disable a user account (admin only)
#[utoipa::path(patch, path = "/api/v1/admin/users/{id}/status", tag = "Admin",
    summary = "Enable or disable a user (admin)", description = "Sets a user's active/inactive status. Admin only.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state, req), fields(admin_id = %auth_user.user_id, target_user_id = %id))]
pub async fn admin_update_user_status(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut user = match state_guard.db.get_user(&id).await {
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

    user.is_active = req.status == "active";
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user status: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    // Revoke all sessions when a user is disabled
    if !user.is_active {
        if let Err(e) = state_guard.db.delete_sessions_by_user(user.id).await {
            tracing::error!("Failed to revoke sessions for disabled user {}: {}", id, e);
        }
    }

    let event_type = if user.is_active {
        AuditEventType::UserActivated
    } else {
        AuditEventType::UserDeactivated
    };
    let audit = AuditEntry::new(event_type)
        .user(auth_user.user_id, "admin")
        .resource("user", &id)
        .details(serde_json::json!({ "new_status": req.status }))
        .log();
    audit.persist(&state_guard.db).await;

    tracing::info!(
        admin_id = %auth_user.user_id,
        target_user_id = %id,
        new_status = %req.status,
        "Admin updated user status"
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminUserResponse::from(&user))),
    )
}

/// `DELETE /api/v1/admin/users/{id}` — delete a user account (admin only, GDPR anonymize)
#[utoipa::path(delete, path = "/api/v1/admin/users/{id}", tag = "Admin",
    summary = "Delete user (admin)", description = "Anonymizes user data per GDPR. Admin only.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id, target_user_id = %id))]
pub async fn admin_delete_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Prevent admin from deleting their own account via admin panel
    if id == auth_user.user_id.to_string() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "CANNOT_DELETE_SELF",
                "You cannot delete your own account",
            )),
        );
    }

    let admin_username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    match state_guard.db.anonymize_user(&id).await {
        Ok(true) => {
            AuditEntry::new(AuditEventType::UserDeleted)
                .user(auth_user.user_id, &admin_username)
                .resource("user", &id)
                .log();
            tracing::info!(
                admin_id = %auth_user.user_id,
                target_user_id = %id,
                "Admin anonymized user"
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to anonymize user {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to delete user")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN — BOOKING MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Response type for admin booking listing (includes user details)
#[derive(Debug, Serialize)]
pub struct AdminBookingResponse {
    id: String,
    user_id: String,
    user_name: String,
    user_email: String,
    lot_id: String,
    lot_name: String,
    slot_id: String,
    slot_number: String,
    vehicle_plate: String,
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

/// `GET /api/v1/admin/bookings` — list all bookings (admin only)
#[utoipa::path(get, path = "/api/v1/admin/bookings", tag = "Admin",
    summary = "List all bookings (admin)", description = "Returns all bookings with enriched details. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All bookings"), (status = 403, description = "Forbidden"))
)]
pub async fn admin_list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminBookingResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list bookings",
                )),
            );
        }
    };

    // Batch-load all users and lots upfront to avoid N+1 queries
    let all_users = state_guard.db.list_users().await.unwrap_or_default();
    let user_map: std::collections::HashMap<String, _> = all_users
        .into_iter()
        .map(|u| (u.id.to_string(), u))
        .collect();

    let all_lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let lot_map: std::collections::HashMap<String, _> = all_lots
        .into_iter()
        .map(|l| (l.id.to_string(), l))
        .collect();

    let mut response = Vec::with_capacity(bookings.len());
    for booking in bookings {
        let (user_name, user_email) = match user_map.get(&booking.user_id.to_string()) {
            Some(u) => (u.name.clone(), u.email.clone()),
            None => (booking.user_id.to_string(), String::new()),
        };

        let lot_name = match lot_map.get(&booking.lot_id.to_string()) {
            Some(l) => l.name.clone(),
            None => booking.lot_id.to_string(),
        };

        response.push(AdminBookingResponse {
            id: booking.id.to_string(),
            user_id: booking.user_id.to_string(),
            user_name,
            user_email,
            lot_id: booking.lot_id.to_string(),
            lot_name,
            slot_id: booking.slot_id.to_string(),
            slot_number: booking.slot_number.to_string(),
            vehicle_plate: booking.vehicle.license_plate.clone(),
            start_time: booking.start_time,
            end_time: booking.end_time,
            status: format!("{:?}", booking.status).to_lowercase(),
            created_at: booking.created_at,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN REPORTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Dashboard stats response
#[derive(Debug, Serialize)]
pub struct AdminStatsResponse {
    total_users: u64,
    total_lots: u64,
    total_slots: u64,
    total_bookings: u64,
    active_bookings: u64,
    occupancy_percent: f64,
}

/// `GET /api/v1/admin/stats` — dashboard stats
#[utoipa::path(get, path = "/api/v1/admin/stats", tag = "Admin",
    summary = "Admin dashboard statistics",
    description = "Returns aggregated system stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<AdminStatsResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db_stats = state_guard
        .db
        .stats()
        .await
        .unwrap_or(crate::db::DatabaseStats {
            users: 0,
            bookings: 0,
            parking_lots: 0,
            slots: 0,
            sessions: 0,
            vehicles: 0,
        });

    // Count active bookings
    let active_bookings = state_guard
        .db
        .list_bookings()
        .await
        .map(|bookings| {
            bookings
                .iter()
                .filter(|b| {
                    b.status == BookingStatus::Confirmed || b.status == BookingStatus::Active
                })
                .count() as u64
        })
        .unwrap_or(0);

    #[allow(clippy::cast_precision_loss)]
    let occupancy = if db_stats.slots > 0 {
        (active_bookings as f64 / db_stats.slots as f64) * 100.0
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminStatsResponse {
            total_users: db_stats.users,
            total_lots: db_stats.parking_lots,
            total_slots: db_stats.slots,
            total_bookings: db_stats.bookings,
            active_bookings,
            occupancy_percent: (occupancy * 100.0).round() / 100.0,
        })),
    )
}

/// Query params for reports
#[derive(Debug, Deserialize)]
pub struct ReportsQuery {
    days: Option<i64>,
}

/// Booking stats by day
#[derive(Debug, Serialize)]
pub struct DailyBookingStat {
    date: String,
    count: usize,
}

/// `GET /api/v1/admin/reports` — booking stats by day for last N days
#[utoipa::path(get, path = "/api/v1/admin/reports", tag = "Admin",
    summary = "Booking reports (admin)",
    description = "Returns daily booking stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_reports(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ReportsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<DailyBookingStat>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let days = query.days.unwrap_or(30);
    let cutoff = Utc::now() - TimeDelta::days(days);

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    // Group by date
    let mut by_date: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for b in &bookings {
        if b.created_at >= cutoff {
            let date = b.created_at.format("%Y-%m-%d").to_string();
            *by_date.entry(date).or_insert(0) += 1;
        }
    }

    let daily_stats: Vec<DailyBookingStat> = by_date
        .into_iter()
        .map(|(date, count)| DailyBookingStat { date, count })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(daily_stats)))
}

/// Heatmap cell: booking count by weekday x hour
#[derive(Debug, Serialize)]
pub struct HeatmapCell {
    weekday: u32,
    hour: u32,
    count: usize,
}

/// `GET /api/v1/admin/heatmap` — booking counts by weekday x hour
#[utoipa::path(get, path = "/api/v1/admin/heatmap", tag = "Admin",
    summary = "Booking heatmap (admin)",
    description = "Returns booking counts by weekday and hour.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_heatmap(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<HeatmapCell>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    // Build 7x24 grid (weekday 0=Mon .. 6=Sun, hour 0..23)
    let mut grid = [[0usize; 24]; 7];
    for b in &bookings {
        let weekday = b.start_time.weekday().num_days_from_monday() as usize;
        let hour = b.start_time.hour() as usize;
        if weekday < 7 && hour < 24 {
            grid[weekday][hour] += 1;
        }
    }

    let cells: Vec<HeatmapCell> = grid
        .iter()
        .enumerate()
        .flat_map(|(wd, hours)| {
            hours
                .iter()
                .enumerate()
                .map(move |(h, &count)| HeatmapCell {
                    weekday: u32::try_from(wd).unwrap_or(0),
                    hour: u32::try_from(h).unwrap_or(0),
                    count,
                })
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(cells)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// AUDIT LOG
// ═══════════════════════════════════════════════════════════════════════════════

/// Paginated audit log response
#[derive(Debug, Serialize)]
pub struct PaginatedAuditLog {
    pub entries: Vec<crate::db::AuditLogEntry>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// `GET /api/v1/admin/audit-log` — paginated, filterable audit log
#[utoipa::path(get, path = "/api/v1/admin/audit-log", tag = "Admin",
    summary = "Audit log (admin)",
    description = "Returns paginated audit log entries. Filterable by action, user, date range.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_audit_log(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<PaginatedAuditLog>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let page = params
        .get("page")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("per_page")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(25)
        .min(100);
    let action_filter = params.get("action").cloned();
    let user_filter = params.get("user").cloned();
    let from_filter = params
        .get("from")
        .and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
    let to_filter = params
        .get("to")
        .and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());

    match state_guard.db.list_all_audit_log().await {
        Ok(mut entries) => {
            // Apply filters
            if let Some(ref action) = action_filter {
                entries.retain(|e| e.event_type.to_lowercase().contains(&action.to_lowercase()));
            }
            if let Some(ref user) = user_filter {
                let q = user.to_lowercase();
                entries.retain(|e| {
                    e.username
                        .as_ref()
                        .is_some_and(|u| u.to_lowercase().contains(&q))
                        || e.user_id.is_some_and(|id| id.to_string().contains(&q))
                });
            }
            if let Some(from) = from_filter {
                entries.retain(|e| e.timestamp.date_naive() >= from);
            }
            if let Some(to) = to_filter {
                entries.retain(|e| e.timestamp.date_naive() <= to);
            }

            let total = entries.len();
            let total_pages = if total == 0 {
                1
            } else {
                (total + per_page - 1) / per_page
            };
            let start = (page - 1) * per_page;
            let page_entries = if start < total {
                entries[start..(start + per_page).min(total)].to_vec()
            } else {
                Vec::new()
            };

            (
                StatusCode::OK,
                Json(ApiResponse::success(PaginatedAuditLog {
                    entries: page_entries,
                    total,
                    page,
                    per_page,
                    total_pages,
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to list audit log: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list audit log",
                )),
            )
        }
    }
}

/// `GET /api/v1/admin/audit-log/export` — CSV export of audit log
#[utoipa::path(get, path = "/api/v1/admin/audit-log/export", tag = "Admin",
    summary = "Export audit log as CSV",
    description = "Download all audit log entries as a CSV file. Supports optional date filtering via from and to query params (YYYY-MM-DD). Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_audit_log_export(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (
            status,
            [
                (axum::http::header::CONTENT_TYPE, "text/plain"),
                (axum::http::header::CONTENT_DISPOSITION, "inline"),
            ],
            msg.to_string(),
        );
    }

    let action_filter = params.get("action").cloned();
    let user_filter = params.get("user").cloned();
    let from_filter = params
        .get("from")
        .and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
    let to_filter = params
        .get("to")
        .and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());

    match state_guard.db.list_all_audit_log().await {
        Ok(mut entries) => {
            if let Some(ref action) = action_filter {
                entries.retain(|e| e.event_type.to_lowercase().contains(&action.to_lowercase()));
            }
            if let Some(ref user) = user_filter {
                let q = user.to_lowercase();
                entries.retain(|e| {
                    e.username
                        .as_ref()
                        .is_some_and(|u| u.to_lowercase().contains(&q))
                });
            }
            if let Some(from) = from_filter {
                entries.retain(|e| e.timestamp.date_naive() >= from);
            }
            if let Some(to) = to_filter {
                entries.retain(|e| e.timestamp.date_naive() <= to);
            }

            let mut csv = String::from(
                "id,timestamp,event_type,user_id,username,target_type,target_id,ip_address,details\n",
            );
            for e in &entries {
                use std::fmt::Write;
                let _ = write!(
                    csv,
                    "{},{},{},{},{},{},{},{},{}\n",
                    e.id,
                    e.timestamp.to_rfc3339(),
                    csv_escape(&e.event_type),
                    e.user_id.map_or_else(String::new, |id| id.to_string()),
                    csv_escape(e.username.as_deref().unwrap_or("")),
                    csv_escape(e.target_type.as_deref().unwrap_or("")),
                    csv_escape(e.target_id.as_deref().unwrap_or("")),
                    csv_escape(e.ip_address.as_deref().unwrap_or("")),
                    csv_escape(e.details.as_deref().unwrap_or("")),
                );
            }

            (
                StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"audit-log.csv\"",
                    ),
                ],
                csv,
            )
        }
        Err(e) => {
            tracing::error!("Failed to export audit log: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [
                    (axum::http::header::CONTENT_TYPE, "text/plain"),
                    (axum::http::header::CONTENT_DISPOSITION, "inline"),
                ],
                "Failed to export audit log".to_string(),
            )
        }
    }
}

/// Escape a cell value for CSV (protection against CSV injection).
fn csv_escape(value: &str) -> String {
    let needs_prefix = value.starts_with('=')
        || value.starts_with('+')
        || value.starts_with('-')
        || value.starts_with('@');

    let val = if needs_prefix {
        format!("'{value}")
    } else {
        value.to_string()
    };

    if val.contains(',') || val.contains('"') || val.contains('\n') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: DATABASE RESET
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for database reset confirmation
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminResetRequest {
    confirm: String,
}

/// `POST /api/v1/admin/reset` — wipe all data (admin only)
#[utoipa::path(post, path = "/api/v1/admin/reset", tag = "Admin",
    summary = "Reset database (admin)",
    description = "Wipes all data. Destructive. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_reset(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AdminResetRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if req.confirm != "RESET" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "CONFIRMATION_REQUIRED",
                "Body must contain {\"confirm\": \"RESET\"}",
            )),
        );
    }

    // Capture admin info before wipe
    let Ok(Some(admin)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to read admin user before reset",
            )),
        );
    };

    if let Err(e) = state_guard.db.clear_all_data().await {
        tracing::error!("Database reset failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to reset database",
            )),
        );
    }

    // Re-create the admin user who triggered the reset
    let admin_user = User {
        id: admin.id,
        username: admin.username.clone(),
        email: admin.email.clone(),
        name: admin.name.clone(),
        password_hash: admin.password_hash,
        role: admin.role,
        is_active: true,
        phone: admin.phone,
        picture: admin.picture,
        preferences: admin.preferences,
        credits_balance: 0,
        credits_monthly_quota: 0,
        credits_last_refilled: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        tenant_id: admin.tenant_id,
        accessibility_needs: None,
        cost_center: None,
        department: None,
    };

    if let Err(e) = state_guard.db.save_user(&admin_user).await {
        tracing::error!("Failed to re-create admin after reset: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Database reset succeeded but admin re-creation failed",
            )),
        );
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, &admin_user.username)
        .details(serde_json::json!({"action": "database_reset"}))
        .log();

    tracing::warn!(
        admin = %admin_user.username,
        "Database reset completed"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: AUTO-RELEASE SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/settings/auto-release` — return auto-release config
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/auto-release",
    tag = "Admin",
    summary = "Get auto-release settings",
    description = "Return the auto-release timing configuration. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_auto_release(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let enabled = read_admin_setting(&state_guard.db, "auto_release_enabled").await;
    let minutes = read_admin_setting(&state_guard.db, "auto_release_minutes").await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "auto_release_enabled": enabled.parse::<bool>().unwrap_or(false),
            "auto_release_minutes": minutes.parse::<i32>().unwrap_or(30),
        }))),
    )
}

/// Request body for auto-release settings update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AutoReleaseSettingsRequest {
    auto_release_enabled: Option<bool>,
    auto_release_minutes: Option<i32>,
}

/// `PUT /api/v1/admin/settings/auto-release` — update auto-release timing
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/auto-release",
    tag = "Admin",
    summary = "Update auto-release settings",
    description = "Update auto-release timing for unclaimed bookings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_auto_release(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AutoReleaseSettingsRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if let Some(enabled) = req.auto_release_enabled {
        if let Err(e) = state_guard
            .db
            .set_setting("auto_release_enabled", &enabled.to_string())
            .await
        {
            tracing::error!("Failed to save auto_release_enabled: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save setting")),
            );
        }
    }

    if let Some(minutes) = req.auto_release_minutes {
        if minutes < 1 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "auto_release_minutes must be >= 1",
                )),
            );
        }
        if let Err(e) = state_guard
            .db
            .set_setting("auto_release_minutes", &minutes.to_string())
            .await
        {
            tracing::error!("Failed to save auto_release_minutes: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save setting")),
            );
        }
    }

    // Return updated values
    let enabled = read_admin_setting(&state_guard.db, "auto_release_enabled").await;
    let minutes = read_admin_setting(&state_guard.db, "auto_release_minutes").await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "auto_release_enabled": enabled.parse::<bool>().unwrap_or(false),
            "auto_release_minutes": minutes.parse::<i32>().unwrap_or(30),
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: EMAIL SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/settings/email` — return SMTP config (password masked)
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/email",
    tag = "Admin",
    summary = "Get email settings",
    description = "Return SMTP configuration (password masked). Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_email_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let host = state_guard
        .db
        .get_setting("smtp_host")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let port = state_guard
        .db
        .get_setting("smtp_port")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "587".to_string());
    let username = state_guard
        .db
        .get_setting("smtp_username")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let has_password = state_guard
        .db
        .get_setting("smtp_password")
        .await
        .ok()
        .flatten()
        .is_some_and(|p| !p.is_empty());
    let from = state_guard
        .db
        .get_setting("smtp_from")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let enabled = state_guard
        .db
        .get_setting("smtp_enabled")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "false".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "smtp_host": host,
            "smtp_port": port.parse::<i32>().unwrap_or(587),
            "smtp_username": username,
            "smtp_password": if has_password { "********" } else { "" },
            "smtp_from": from,
            "smtp_enabled": enabled.parse::<bool>().unwrap_or(false),
        }))),
    )
}

/// Request body for email settings update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EmailSettingsRequest {
    #[serde(alias = "smtp_host")]
    host: Option<String>,
    #[serde(alias = "smtp_port")]
    port: Option<i32>,
    #[serde(alias = "smtp_username")]
    username: Option<String>,
    #[serde(alias = "smtp_password")]
    password: Option<String>,
    #[serde(alias = "smtp_from")]
    from: Option<String>,
    #[serde(alias = "smtp_enabled")]
    enabled: Option<bool>,
}

/// `PUT /api/v1/admin/settings/email` — update SMTP settings
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/email",
    tag = "Admin",
    summary = "Update email settings",
    description = "Update SMTP settings for outgoing emails. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_email_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<EmailSettingsRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db = &state_guard.db;

    if let Some(host) = &req.host {
        let _ = db.set_setting("smtp_host", host).await;
    }
    if let Some(port) = req.port {
        let _ = db.set_setting("smtp_port", &port.to_string()).await;
    }
    if let Some(username) = &req.username {
        let _ = db.set_setting("smtp_username", username).await;
    }
    if let Some(password) = &req.password {
        // Don't overwrite with the masked placeholder
        if password != "********" {
            let _ = db.set_setting("smtp_password", password).await;
        }
    }
    if let Some(from) = &req.from {
        let _ = db.set_setting("smtp_from", from).await;
    }
    if let Some(enabled) = req.enabled {
        let _ = db.set_setting("smtp_enabled", &enabled.to_string()).await;
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .resource("settings", "email")
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            serde_json::json!({"message": "Email settings updated"}),
        )),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: PRIVACY SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/privacy` — return privacy/GDPR settings
#[utoipa::path(
    get,
    path = "/api/v1/admin/privacy",
    tag = "Admin",
    summary = "Get privacy settings",
    description = "Return privacy and GDPR settings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_privacy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db = &state_guard.db;

    let privacy_policy_url = db
        .get_setting("privacy_policy_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let data_retention_days = db
        .get_setting("data_retention_days")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "365".to_string());
    let require_consent = db
        .get_setting("require_consent")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());
    let anonymize_on_delete = db
        .get_setting("anonymize_on_delete")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "privacy_policy_url": privacy_policy_url,
            "data_retention_days": data_retention_days.parse::<i32>().unwrap_or(365),
            "require_consent": require_consent.parse::<bool>().unwrap_or(true),
            "anonymize_on_delete": anonymize_on_delete.parse::<bool>().unwrap_or(true),
        }))),
    )
}

/// Request body for privacy settings update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PrivacySettingsRequest {
    privacy_policy_url: Option<String>,
    data_retention_days: Option<i32>,
    require_consent: Option<bool>,
    anonymize_on_delete: Option<bool>,
}

/// `PUT /api/v1/admin/privacy` — update privacy settings
#[utoipa::path(
    put,
    path = "/api/v1/admin/privacy",
    tag = "Admin",
    summary = "Update privacy settings",
    description = "Update privacy and GDPR settings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_privacy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<PrivacySettingsRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db = &state_guard.db;

    if let Some(url) = &req.privacy_policy_url {
        let _ = db.set_setting("privacy_policy_url", url).await;
    }
    if let Some(days) = req.data_retention_days {
        if days < 1 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "data_retention_days must be >= 1",
                )),
            );
        }
        let _ = db
            .set_setting("data_retention_days", &days.to_string())
            .await;
    }
    if let Some(consent) = req.require_consent {
        let _ = db
            .set_setting("require_consent", &consent.to_string())
            .await;
    }
    if let Some(anonymize) = req.anonymize_on_delete {
        let _ = db
            .set_setting("anonymize_on_delete", &anonymize.to_string())
            .await;
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .resource("settings", "privacy")
        .log();

    // Return current state
    let privacy_policy_url = db
        .get_setting("privacy_policy_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let data_retention_days = db
        .get_setting("data_retention_days")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "365".to_string());
    let require_consent = db
        .get_setting("require_consent")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());
    let anonymize_on_delete = db
        .get_setting("anonymize_on_delete")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "privacy_policy_url": privacy_policy_url,
            "data_retention_days": data_retention_days.parse::<i32>().unwrap_or(365),
            "require_consent": require_consent.parse::<bool>().unwrap_or(true),
            "anonymize_on_delete": anonymize_on_delete.parse::<bool>().unwrap_or(true),
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: UPDATE USER
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for admin user update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminUpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    role: Option<String>,
    is_active: Option<bool>,
}

/// `PUT /api/v1/admin/users/{id}/update` — admin can update user details
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}/update",
    tag = "Admin",
    summary = "Update user details",
    description = "Admin can update any user's details (name, email, department, etc.).",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<AdminUpdateUserRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut user = match state_guard.db.get_user(&id).await {
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

    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(email) = req.email {
        // Basic email validation
        if !email.contains('@') || email.len() < 5 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_INPUT", "Invalid email address")),
            );
        }
        user.email = email;
    }
    if let Some(role_str) = req.role {
        let new_role = match role_str.to_lowercase().as_str() {
            "user" => UserRole::User,
            "premium" => UserRole::Premium,
            "admin" => UserRole::Admin,
            "superadmin" => {
                // Only SuperAdmin can assign SuperAdmin
                let caller = state_guard
                    .db
                    .get_user(&auth_user.user_id.to_string())
                    .await
                    .ok()
                    .flatten();
                if caller.map(|c| c.role) != Some(UserRole::SuperAdmin) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(ApiResponse::error(
                            "FORBIDDEN",
                            "Only SuperAdmin can assign SuperAdmin role",
                        )),
                    );
                }
                UserRole::SuperAdmin
            }
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_INPUT",
                        "Role must be user, premium, admin, or superadmin",
                    )),
                );
            }
        };
        user.role = new_role;
    }
    if let Some(active) = req.is_active {
        user.is_active = active;
    }
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(auth_user.user_id, "admin")
        .resource("user", &id)
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "id": user.id.to_string(),
            "username": user.username,
            "email": user.email,
            "name": user.name,
            "role": format!("{:?}", user.role).to_lowercase(),
            "is_active": user.is_active,
        }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_user_role_request() {
        let json = r#"{"role":"admin"}"#;
        let req: UpdateUserRoleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, "admin");
    }

    #[test]
    fn test_update_user_status_request() {
        let json = r#"{"status":"active"}"#;
        let req: UpdateUserStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, "active");
    }

    #[test]
    fn test_admin_reset_request() {
        let json = r#"{"confirm":"RESET"}"#;
        let req: AdminResetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.confirm, "RESET");
    }

    #[test]
    fn test_auto_release_settings_request() {
        let json = r#"{"auto_release_enabled":true,"auto_release_minutes":15}"#;
        let req: AutoReleaseSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.auto_release_enabled, Some(true));
        assert_eq!(req.auto_release_minutes, Some(15));
    }

    #[test]
    fn test_auto_release_settings_request_partial() {
        let json = r#"{"auto_release_minutes":45}"#;
        let req: AutoReleaseSettingsRequest = serde_json::from_str(json).unwrap();
        assert!(req.auto_release_enabled.is_none());
        assert_eq!(req.auto_release_minutes, Some(45));
    }

    #[test]
    fn test_email_settings_request_full() {
        let json = r#"{
            "smtp_host":"smtp.example.com",
            "smtp_port":587,
            "smtp_username":"user@example.com",
            "smtp_password":"secret",
            "smtp_from":"noreply@example.com",
            "smtp_enabled":true
        }"#;
        let req: EmailSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.host.as_deref(), Some("smtp.example.com"));
        assert_eq!(req.port, Some(587));
        assert_eq!(req.enabled, Some(true));
    }

    #[test]
    fn test_email_settings_request_empty() {
        let json = r#"{}"#;
        let req: EmailSettingsRequest = serde_json::from_str(json).unwrap();
        assert!(req.host.is_none());
        assert!(req.port.is_none());
        assert!(req.enabled.is_none());
    }

    #[test]
    fn test_privacy_settings_request() {
        let json = r#"{
            "privacy_policy_url":"https://example.com/privacy",
            "data_retention_days":365,
            "require_consent":true,
            "anonymize_on_delete":true
        }"#;
        let req: PrivacySettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.privacy_policy_url.as_deref(),
            Some("https://example.com/privacy")
        );
        assert_eq!(req.data_retention_days, Some(365));
        assert_eq!(req.require_consent, Some(true));
        assert_eq!(req.anonymize_on_delete, Some(true));
    }

    #[test]
    fn test_admin_update_user_request_full() {
        let json =
            r#"{"name":"Updated","email":"new@example.com","role":"admin","is_active":false}"#;
        let req: AdminUpdateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Updated"));
        assert_eq!(req.email.as_deref(), Some("new@example.com"));
        assert_eq!(req.role.as_deref(), Some("admin"));
        assert_eq!(req.is_active, Some(false));
    }

    #[test]
    fn test_admin_update_user_request_partial() {
        let json = r#"{"is_active":true}"#;
        let req: AdminUpdateUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.email.is_none());
        assert!(req.role.is_none());
        assert_eq!(req.is_active, Some(true));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AUDIT LOG TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_paginated_audit_log_serialization() {
        let entry = crate::db::AuditLogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: "LoginSuccess".to_string(),
            user_id: Some(uuid::Uuid::new_v4()),
            username: Some("admin".to_string()),
            details: Some(r#"{"ip":"127.0.0.1"}"#.to_string()),
            target_type: Some("user".to_string()),
            target_id: Some("abc-123".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
        };
        let paginated = PaginatedAuditLog {
            entries: vec![entry],
            total: 1,
            page: 1,
            per_page: 25,
            total_pages: 1,
        };
        let json = serde_json::to_value(&paginated).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["page"], 1);
        assert_eq!(json["per_page"], 25);
        assert_eq!(json["total_pages"], 1);
        assert_eq!(json["entries"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_paginated_audit_log_empty() {
        let paginated = PaginatedAuditLog {
            entries: vec![],
            total: 0,
            page: 1,
            per_page: 25,
            total_pages: 1,
        };
        let json = serde_json::to_value(&paginated).unwrap();
        assert_eq!(json["total"], 0);
        assert!(json["entries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_paginated_audit_log_pagination_math() {
        // 55 entries, 25 per page = 3 pages
        let paginated = PaginatedAuditLog {
            entries: vec![],
            total: 55,
            page: 3,
            per_page: 25,
            total_pages: 3,
        };
        let json = serde_json::to_value(&paginated).unwrap();
        assert_eq!(json["total_pages"], 3);
        assert_eq!(json["page"], 3);
    }

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("John Doe"), "John Doe");
    }

    #[test]
    fn test_csv_escape_injection() {
        assert_eq!(csv_escape("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(csv_escape("+cmd"), "'+cmd");
        assert_eq!(csv_escape("-evil"), "'-evil");
        assert_eq!(csv_escape("@import"), "'@import");
    }

    #[test]
    fn test_csv_escape_special_chars() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_audit_log_entry_new_fields_default() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2026-03-22T10:00:00Z",
            "event_type": "LoginSuccess",
            "user_id": null,
            "username": null,
            "details": null
        }"#;
        let entry: crate::db::AuditLogEntry = serde_json::from_str(json).unwrap();
        assert!(entry.target_type.is_none());
        assert!(entry.target_id.is_none());
        assert!(entry.ip_address.is_none());
    }
}
