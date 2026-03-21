//! Admin handlers: user management, booking management, settings, features,
//! impressum, announcements, guest bookings, stats, reports, heatmap,
//! auto-release, email settings, privacy, dashboard, reset, and audit log.
//!
//! Shared types used by other modules (`AdminUserResponse`, `AdminBookingResponse`)
//! are defined here and re-exported.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use uuid::Uuid;

use parkhub_common::models::{
    Announcement, AnnouncementSeverity, GuestBooking, Notification,
};
use parkhub_common::{
    ApiResponse, Booking, BookingStatus, User, UserRole,
};

use crate::audit::{AuditEntry, AuditEventType};
use crate::metrics;
use crate::utils::html_escape;

use super::{check_admin, AuthUser, SharedState};


#[derive(Debug, Serialize)]
struct BookingsByDay {
    date: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct BookingsByLot {
    lot_name: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct OccupancyByHour {
    hour: u32,
    avg_occupancy: f64,
}

#[derive(Debug, Serialize)]
struct TopUser {
    username: String,
    booking_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DashboardCharts {
    bookings_by_day: Vec<BookingsByDay>,
    bookings_by_lot: Vec<BookingsByLot>,
    occupancy_by_hour: Vec<OccupancyByHour>,
    top_users: Vec<TopUser>,
}
)]
pub async fn admin_dashboard_charts(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<DashboardCharts>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let users = state_guard.db.list_users().await.unwrap_or_default();
    let now = Utc::now();
    let cutoff = now - TimeDelta::days(30);

    // ── bookings_by_day (last 30 days) ──────────────────────────────────────
    let mut by_day: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    // Pre-fill all 30 days so the chart has continuous x-axis
    for d in 0..30 {
        let date = (now - TimeDelta::days(d)).format("%Y-%m-%d").to_string();
        by_day.entry(date).or_insert(0);
    }
    for b in &bookings {
        if b.created_at >= cutoff {
            let date = b.created_at.format("%Y-%m-%d").to_string();
            *by_day.entry(date).or_insert(0) += 1;
        }
    }
    let bookings_by_day: Vec<BookingsByDay> = by_day
        .into_iter()
        .map(|(date, count)| BookingsByDay { date, count })
        .collect();

    // ── bookings_by_lot ─────────────────────────────────────────────────────
    let lot_name_map: std::collections::HashMap<Uuid, String> =
        lots.iter().map(|l| (l.id, l.name.clone())).collect();
    let mut by_lot: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for b in &bookings {
        let name = lot_name_map
            .get(&b.lot_id)
            .cloned()
            .unwrap_or_else(|| b.lot_id.to_string());
        *by_lot.entry(name).or_insert(0) += 1;
    }
    let mut bookings_by_lot: Vec<BookingsByLot> = by_lot
        .into_iter()
        .map(|(lot_name, count)| BookingsByLot { lot_name, count })
        .collect();
    bookings_by_lot.sort_by(|a, b| b.count.cmp(&a.count));

    // ── occupancy_by_hour (average across all lots) ─────────────────────────
    // For each hour of the day, count how many bookings are active during that
    // hour within the last 30 days, then divide by number of days with data.
    let total_slots: i32 = lots.iter().map(|l| l.total_slots).sum();
    let mut hour_totals = [0usize; 24];
    let mut hour_days = [0usize; 24];

    // Count distinct days per hour that had at least one booking
    let mut hour_day_set: [std::collections::HashSet<String>; 24] =
        std::array::from_fn(|_| std::collections::HashSet::new());

    for b in &bookings {
        if b.start_time >= cutoff || b.end_time >= cutoff {
            // Walk through each hour the booking spans
            let mut t = b.start_time;
            while t < b.end_time && t < now {
                let h = t.hour() as usize;
                if h < 24 {
                    hour_totals[h] += 1;
                    hour_day_set[h].insert(t.format("%Y-%m-%d").to_string());
                }
                t += TimeDelta::hours(1);
            }
        }
    }

    for (h, day_set) in hour_day_set.iter().enumerate() {
        hour_days[h] = day_set.len().max(1);
    }

    let occupancy_by_hour: Vec<OccupancyByHour> = (0..24)
        .map(|h| {
            #[allow(clippy::cast_precision_loss)]
            let avg_count = hour_totals[h] as f64 / hour_days[h] as f64;
            let avg_occ = if total_slots > 0 {
                (avg_count / f64::from(total_slots)).min(1.0)
            } else {
                0.0
            };
            OccupancyByHour {
                hour: u32::try_from(h).unwrap_or(0),
                avg_occupancy: (avg_occ * 100.0).round() / 100.0,
            }
        })
        .collect();

    // ── top_users (top 10 by booking count) ─────────────────────────────────
    let user_name_map: std::collections::HashMap<Uuid, String> =
        users.iter().map(|u| (u.id, u.username.clone())).collect();
    let mut by_user: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for b in &bookings {
        let name = user_name_map
            .get(&b.user_id)
            .cloned()
            .unwrap_or_else(|| b.user_id.to_string());
        *by_user.entry(name).or_insert(0) += 1;
    }
    let mut top_users: Vec<TopUser> = by_user
        .into_iter()
        .map(|(username, booking_count)| TopUser {
            username,
            booking_count,
        })
        .collect();
    top_users.sort_by(|a, b| b.booking_count.cmp(&a.booking_count));
    top_users.truncate(10);

    (
        StatusCode::OK,
        Json(ApiResponse::success(DashboardCharts {
            bookings_by_day,
            bookings_by_lot,
            occupancy_by_hour,
            top_users,
        })),
    )
}

/// DDG § 5 Impressum fields stored as settings keys with "impressum_" prefix
#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct ImpressumData {
    pub provider_name: String,
    pub provider_legal_form: String,
    pub street: String,
    pub zip_city: String,
    pub country: String,
    pub email: String,
    pub phone: String,
    pub register_court: String,
    pub register_number: String,
    pub vat_id: String,
    pub responsible_person: String,
    pub custom_text: String,
}
)]
pub async fn get_impressum(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let mut data = serde_json::json!({});
    {
        let state = state.read().await;
        for field in IMPRESSUM_FIELDS {
            let key = format!("impressum_{field}");
            let value = state
                .db
                .get_setting(&key)
                .await
                .unwrap_or(None)
                .unwrap_or_default();
            data[field] = serde_json::Value::String(value);
        }
    }

    Json(data)
}
)]
pub async fn get_impressum_admin(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let state_guard = state.read().await;

    // Verify admin role.
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
        );
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
        );
    }

    let mut data = serde_json::json!({});
    for field in IMPRESSUM_FIELDS {
        let key = format!("impressum_{field}");
        let value = state_guard
            .db
            .get_setting(&key)
            .await
            .unwrap_or(None)
            .unwrap_or_default();
        data[field] = serde_json::Value::String(value);
    }

    (StatusCode::OK, Json(data))
}
)]
pub async fn update_impressum(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Verify admin role
    let user_id_str = auth_user.user_id.to_string();
    let state_guard = state.read().await;
    let Ok(Some(user)) = state_guard.db.get_user(&user_id_str).await else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin required")),
        );
    };
    drop(state_guard);

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin required")),
        );
    }

    let state_guard = state.read().await;
    for field in IMPRESSUM_FIELDS {
        if let Some(serde_json::Value::String(value)) = payload.get(*field) {
            let key = format!("impressum_{field}");
            if let Err(e) = state_guard.db.set_setting(&key, value).await {
                tracing::warn!("Failed to save impressum setting {key}: {e}");
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

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

/// Read a single admin setting from DB, falling back to its default.
async fn read_admin_setting(db: &crate::db::Database, key: &str) -> String {
    if let Ok(Some(val)) = db.get_setting(key).await {
        return val;
    }
    ADMIN_SETTINGS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

/// Use-case theme definitions — maps `use_case` key to display config
fn use_case_theme(key: &str) -> serde_json::Value {
    match key {
        "company" => serde_json::json!({
            "key": "company",
            "name": "Company Parking",
            "description": "Employee parking for offices and campuses",
            "icon": "buildings",
            "primary_color": "#0d9488",
            "accent_color": "#0ea5e9",
            "terminology": {
                "user": "Employee", "users": "Employees",
                "lot": "Parking Area", "slot": "Spot",
                "booking": "Reservation", "department": "Department"
            },
            "features_emphasis": ["team_calendar", "absence_tracking", "departments", "credits"]
        }),
        "residential" => serde_json::json!({
            "key": "residential",
            "name": "Residential Parking",
            "description": "Parking for apartment buildings and housing complexes",
            "icon": "house-line",
            "primary_color": "#059669",
            "accent_color": "#84cc16",
            "terminology": {
                "user": "Resident", "users": "Residents",
                "lot": "Parking Area", "slot": "Space",
                "booking": "Reservation", "department": "Unit"
            },
            "features_emphasis": ["guest_parking", "long_term_bookings", "public_display"]
        }),
        "shared" => serde_json::json!({
            "key": "shared",
            "name": "Shared Parking",
            "description": "Community or co-working parking spaces",
            "icon": "users-three",
            "primary_color": "#7c3aed",
            "accent_color": "#06b6d4",
            "terminology": {
                "user": "Member", "users": "Members",
                "lot": "Parking Zone", "slot": "Spot",
                "booking": "Booking", "department": "Group"
            },
            "features_emphasis": ["quick_book", "waitlist", "public_display", "qr_codes"]
        }),
        "rental" => serde_json::json!({
            "key": "rental",
            "name": "Rental / Commercial",
            "description": "Paid parking for customers and tenants",
            "icon": "currency-circle-dollar",
            "primary_color": "#2563eb",
            "accent_color": "#f59e0b",
            "terminology": {
                "user": "Customer", "users": "Customers",
                "lot": "Parking Facility", "slot": "Bay",
                "booking": "Rental", "department": "Account"
            },
            "features_emphasis": ["invoicing", "pricing", "revenue_reports", "guest_bookings"]
        }),
        _ => serde_json::json!({
            "key": "personal",
            "name": "Personal / Private",
            "description": "Private parking for family and friends",
            "icon": "car-simple",
            "primary_color": "#e11d48",
            "accent_color": "#f97316",
            "terminology": {
                "user": "Person", "users": "People",
                "lot": "Driveway", "slot": "Spot",
                "booking": "Booking", "department": "Group"
            },
            "features_emphasis": ["simple_booking", "guest_parking"]
        }),
    }
}
)]
pub async fn admin_get_use_case(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }
    let current = read_admin_setting(&state_guard.db, "use_case").await;
    let theme = use_case_theme(&current);
    let all_options: Vec<serde_json::Value> =
        ["company", "residential", "shared", "rental", "personal"]
            .iter()
            .map(|k| use_case_theme(k))
            .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "current": theme,
            "available": all_options,
        }))),
    )
}
)]
pub async fn admin_get_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut data = serde_json::Map::new();
    for (key, default_val) in ADMIN_SETTINGS {
        let value = state_guard
            .db
            .get_setting(key)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| default_val.to_string());
        data.insert(key.to_string(), serde_json::Value::String(value));
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::Value::Object(data))),
    )
}

/// Validate a settings value against its allowed options.
fn validate_setting_value(key: &str, value: &str) -> Result<(), &'static str> {
    match key {
        "use_case" => {
            if !["company", "residential", "shared", "rental", "personal"].contains(&value) {
                return Err("use_case must be company, residential, shared, rental, or personal");
            }
        }
        "self_registration"
        | "allow_guest_bookings"
        | "require_vehicle"
        | "waitlist_enabled"
        | "credits_enabled"
        | "auto_release_enabled" => {
            if value != "true" && value != "false" {
                return Err("Value must be \"true\" or \"false\"");
            }
        }
        "license_plate_mode" => {
            if !["required", "optional", "disabled"].contains(&value) {
                return Err("license_plate_mode must be required, optional, or disabled");
            }
        }
        "display_name_format" => {
            if !["first_name", "full_name", "username"].contains(&value) {
                return Err("display_name_format must be first_name, full_name, or username");
            }
        }
        "max_bookings_per_day" | "auto_release_minutes" | "credits_per_booking" => {
            if value.parse::<i32>().is_err() {
                return Err("Value must be an integer");
            }
        }
        "min_booking_duration_hours" | "max_booking_duration_hours" => {
            if value.parse::<f64>().is_err() {
                return Err("Value must be a number");
            }
        }
        "company_name" => { /* any string is fine */ }
        _ => return Err("Unknown setting key"),
    }
    Ok(())
}
)]
pub async fn admin_update_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let Some(obj) = payload.as_object() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Request body must be a JSON object of key-value pairs",
            )),
        );
    };

    let allowed_keys: Vec<&str> = ADMIN_SETTINGS.iter().map(|(k, _)| *k).collect();
    let mut updated = serde_json::Map::new();

    for (key, val) in obj {
        if !allowed_keys.contains(&key.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_KEY",
                    format!("Unknown setting: {key}"),
                )),
            );
        }

        let value_str = val
            .as_str()
            .map_or_else(|| val.to_string().trim_matches('"').to_string(), String::from);

        if let Err(msg) = validate_setting_value(key, &value_str) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("VALIDATION_ERROR", msg)),
            );
        }

        if let Err(e) = state_guard.db.set_setting(key, &value_str).await {
            tracing::error!("Failed to save setting {}: {}", key, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save setting")),
            );
        }

        updated.insert(key.clone(), serde_json::Value::String(value_str));
    }

    // Audit log
    if state_guard.config.audit_logging_enabled {
        let _entry = AuditEntry::new(AuditEventType::ConfigChanged)
            .user(auth_user.user_id, "admin")
            .resource("settings", "admin_settings")
            .details(serde_json::json!({ "updated": updated }))
            .log();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::Value::Object(updated))),
    )
}

/// Read enabled features from DB, falling back to defaults.
async fn read_features(db: &crate::db::Database) -> Vec<String> {
    match db.get_setting(SETTINGS_FEATURES_KEY).await {
        Ok(Some(json_str)) => serde_json::from_str::<Vec<String>>(&json_str).unwrap_or_else(|_| {
            DEFAULT_FEATURES
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        }),
        _ => DEFAULT_FEATURES
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
    }
}
)]
pub async fn get_features(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    let enabled = read_features(&state_guard.db).await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "enabled": enabled,
            "available": FEATURE_MODULES,
        }))),
    )
}
)]
pub async fn get_public_theme(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    let use_case = read_admin_setting(&state_guard.db, "use_case").await;
    let company = read_admin_setting(&state_guard.db, "company_name").await;
    let theme = use_case_theme(&use_case);

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "use_case": theme,
            "company_name": company,
        }))),
    )
}
)]
pub async fn admin_get_features(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let enabled = read_features(&state_guard.db).await;

    let available: Vec<serde_json::Value> = FEATURE_MODULES
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "enabled": enabled.contains(&id.to_string()),
                "default_enabled": DEFAULT_FEATURES.contains(id),
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "enabled": enabled,
            "available": available,
        }))),
    )
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateFeaturesRequest {
    enabled: Vec<String>,
}
)]
pub async fn admin_update_features(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<UpdateFeaturesRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.write().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Validate: only accept known feature IDs
    let valid: Vec<String> = body
        .enabled
        .iter()
        .filter(|id| FEATURE_MODULES.contains(&id.as_str()))
        .cloned()
        .collect();

    let json_str = serde_json::to_string(&valid).unwrap_or_default();
    if let Err(e) = state_guard
        .db
        .set_setting(SETTINGS_FEATURES_KEY, &json_str)
        .await
    {
        tracing::error!("Failed to save feature flags: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save features",
            )),
        );
    }

    // Audit log
    if state_guard.config.audit_logging_enabled {
        let _entry = AuditEntry::new(AuditEventType::ConfigChanged)
            .user(auth_user.user_id, "admin")
            .resource("settings", "features_enabled")
            .details(serde_json::json!({ "features": valid }))
            .log();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "enabled": valid,
        }))),
    )
}
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
                .filter(|a| {
                    a.active
                        && a.expires_at.is_none_or(|exp| exp > now)
                })
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

    let Some(mut announcement) = announcements.into_iter().find(|a| a.id.to_string() == id)
    else {
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
)]
pub async fn admin_list_guest_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<GuestBooking>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_guest_bookings().await {
        Ok(bookings) => (StatusCode::OK, Json(ApiResponse::success(bookings))),
        Err(e) => {
            tracing::error!("Failed to list guest bookings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list guest bookings",
                )),
            )
        }
    }
}
)]
pub async fn admin_cancel_guest_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<GuestBooking>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut booking = match state_guard.db.get_guest_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Guest booking not found")),
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

    booking.status = BookingStatus::Cancelled;

    if let Err(e) = state_guard.db.save_guest_booking(&booking).await {
        tracing::error!("Failed to cancel guest booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel guest booking",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

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
)]
pub async fn admin_audit_log(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<Vec<crate::db::AuditLogEntry>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100usize)
        .min(500);

    match state_guard.db.list_audit_log(limit).await {
        Ok(entries) => (StatusCode::OK, Json(ApiResponse::success(entries))),
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

/// Occupancy info for a single lot
#[derive(Debug, Serialize)]
pub struct LotOccupancy {
    lot_id: String,
    lot_name: String,
    total_slots: i32,
    occupied_slots: i32,
    available_slots: i32,
}
)]
pub async fn public_occupancy(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<Vec<LotOccupancy>>>) {
    let state_guard = state.read().await;

    let lots = match state_guard.db.list_parking_lots().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to list lots for occupancy: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to get occupancy",
                )),
            );
        }
    };

    // Count active bookings per lot
    let now = Utc::now();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut occupancy = Vec::with_capacity(lots.len());
    for lot in &lots {
        let occupied = i32::try_from(
            bookings
                .iter()
                .filter(|b| {
                    b.lot_id == lot.id
                        && b.start_time <= now
                        && b.end_time >= now
                        && matches!(
                            b.status,
                            parkhub_common::BookingStatus::Confirmed
                                | parkhub_common::BookingStatus::Active
                        )
                })
                .count(),
        )
        .unwrap_or(i32::MAX);

        let available = (lot.total_slots - occupied).max(0);

        occupancy.push(LotOccupancy {
            lot_id: lot.id.to_string(),
            lot_name: lot.name.clone(),
            total_slots: lot.total_slots,
            occupied_slots: occupied,
            available_slots: available,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(occupancy)))
}
)]
pub async fn public_display(State(state): State<SharedState>) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;

    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let now = Utc::now();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="refresh" content="30">
<title>ParkHub — Parking Availability</title>
<style>
  body { font-family: system-ui, sans-serif; margin: 0; padding: 2rem; background: #1a1a2e; color: #eee; }
  h1 { text-align: center; margin-bottom: 2rem; }
  .lots { display: flex; flex-wrap: wrap; gap: 1.5rem; justify-content: center; }
  .lot { background: #16213e; border-radius: 12px; padding: 1.5rem 2rem; min-width: 220px; text-align: center; }
  .lot-name { font-size: 1.2rem; font-weight: 600; margin-bottom: 0.5rem; }
  .available { font-size: 3rem; font-weight: 700; }
  .available.green { color: #4ade80; }
  .available.yellow { color: #facc15; }
  .available.red { color: #f87171; }
  .label { font-size: 0.9rem; color: #94a3b8; }
</style>
</head>
<body>
<h1>Parking Availability</h1>
<div class="lots">
"#,
    );

    for lot in &lots {
        let occupied = i32::try_from(
            bookings
                .iter()
                .filter(|b| {
                    b.lot_id == lot.id
                        && b.start_time <= now
                        && b.end_time >= now
                        && matches!(
                            b.status,
                            parkhub_common::BookingStatus::Confirmed
                                | parkhub_common::BookingStatus::Active
                        )
                })
                .count(),
        )
        .unwrap_or(i32::MAX);

        let available = (lot.total_slots - occupied).max(0);
        let pct = if lot.total_slots > 0 {
            (f64::from(available) / f64::from(lot.total_slots)) * 100.0
        } else {
            0.0
        };
        let color_class = if pct > 30.0 {
            "green"
        } else if pct > 10.0 {
            "yellow"
        } else {
            "red"
        };

        {
            let _ = write!(
                html,
                r#"<div class="lot">
  <div class="lot-name">{}</div>
  <div class="available {}">{}</div>
  <div class="label">of {} available</div>
</div>
"#,
                crate::utils::html_escape(&lot.name),
                color_class,
                available,
                lot.total_slots
            );
        }
    }

    html.push_str("</div>\n</body>\n</html>\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

/// Request body for database reset confirmation
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminResetRequest {
    confirm: String,
}
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

/// Request body for admin user update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminUpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    role: Option<String>,
    is_active: Option<bool>,
}
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
    use parkhub_common::models::UserPreferences;
    use parkhub_common::UserRole;
    use uuid::Uuid;

    fn make_test_user(role: UserRole, is_active: bool) -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password_hash: "hash".to_string(),
            role,
            is_active,
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
            credits_balance: 5,
            credits_monthly_quota: 10,
            credits_last_refilled: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
        }
    }

    #[test]
    fn test_admin_user_response_from_active_admin() {
        let user = make_test_user(UserRole::Admin, true);
        let resp = AdminUserResponse::from(&user);
        assert_eq!(resp.username, "testuser");
        assert_eq!(resp.email, "test@example.com");
        assert_eq!(resp.role, "admin");
        assert_eq!(resp.status, "active");
        assert!(resp.is_active);
        assert_eq!(resp.credits_balance, 5);
        assert_eq!(resp.credits_monthly_quota, 10);
    }

    #[test]
    fn test_admin_user_response_from_disabled_user() {
        let user = make_test_user(UserRole::User, false);
        let resp = AdminUserResponse::from(&user);
        assert_eq!(resp.role, "user");
        assert_eq!(resp.status, "disabled");
        assert!(!resp.is_active);
    }

    #[test]
    fn test_admin_user_response_from_superadmin() {
        let user = make_test_user(UserRole::SuperAdmin, true);
        let resp = AdminUserResponse::from(&user);
        assert_eq!(resp.role, "superadmin");
        assert_eq!(resp.status, "active");
    }

    #[test]
    fn test_admin_user_response_serialize() {
        let user = make_test_user(UserRole::Admin, true);
        let resp = AdminUserResponse::from(&user);
        let json = serde_json::to_string(&resp).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["username"], "testuser");
        assert_eq!(value["role"], "admin");
        assert_eq!(value["status"], "active");
        assert_eq!(value["credits_balance"], 5);
        assert_eq!(value["is_active"], true);
    }

    #[test]
    fn test_admin_user_response_id_is_uuid_string() {
        let user = make_test_user(UserRole::User, true);
        let resp = AdminUserResponse::from(&user);
        // ID should be parseable back to UUID
        assert!(Uuid::parse_str(&resp.id).is_ok());
    }
}
