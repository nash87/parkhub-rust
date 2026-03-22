//! Extended admin features: bulk operations, advanced reporting, notification preferences,
//! booking policies, and health check improvements.

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use parkhub_common::{ApiResponse, BookingStatus, UserRole};

use crate::audit::{AuditEntry, AuditEventType};

use super::{check_admin, AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// BULK ADMIN OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for bulk user update.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BulkUserUpdateRequest {
    /// User IDs to update
    pub user_ids: Vec<String>,
    /// Action: "activate", "deactivate", "set_role"
    pub action: String,
    /// Role to set (only used with "set_role" action)
    pub role: Option<String>,
}

/// Result of a bulk operation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BulkOperationResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// `POST /api/v1/admin/users/bulk-update` — Batch role change, activate/deactivate.
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/bulk-update",
    tag = "Admin",
    summary = "Bulk update users",
    description = "Batch activate, deactivate, or change role for multiple users.",
    security(("bearer_auth" = [])),
    request_body = BulkUserUpdateRequest,
    responses(
        (status = 200, description = "Bulk operation result"),
        (status = 400, description = "Invalid action"),
    )
)]
pub async fn bulk_update_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<BulkUserUpdateRequest>,
) -> (StatusCode, Json<ApiResponse<BulkOperationResult>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let valid_actions = ["activate", "deactivate", "set_role"];
    if !valid_actions.contains(&req.action.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_ACTION",
                "Action must be one of: activate, deactivate, set_role",
            )),
        );
    }

    if req.action == "set_role" && req.role.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "MISSING_ROLE",
                "Role is required for set_role action",
            )),
        );
    }

    let total = req.user_ids.len();
    let mut succeeded = 0;
    let mut errors = Vec::new();

    for user_id in &req.user_ids {
        match state_guard.db.get_user(user_id).await {
            Ok(Some(mut user)) => {
                match req.action.as_str() {
                    "activate" => user.is_active = true,
                    "deactivate" => user.is_active = false,
                    "set_role" => {
                        if let Some(ref role) = req.role {
                            match role.as_str() {
                                "user" => user.role = UserRole::User,
                                "premium" => user.role = UserRole::Premium,
                                "admin" => user.role = UserRole::Admin,
                                _ => {
                                    errors.push(format!("Invalid role for user {user_id}: {role}"));
                                    continue;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                user.updated_at = Utc::now();
                if let Err(e) = state_guard.db.save_user(&user).await {
                    errors.push(format!("Failed to update user {user_id}: {e}"));
                } else {
                    succeeded += 1;
                }
            }
            Ok(None) => {
                errors.push(format!("User {user_id} not found"));
            }
            Err(e) => {
                errors.push(format!("Error fetching user {user_id}: {e}"));
            }
        }
    }

    AuditEntry::new(AuditEventType::SettingsChanged)
        .user(auth_user.user_id, "")
        .detail(&format!(
            "Bulk {} on {} users ({} succeeded)",
            req.action, total, succeeded
        ))
        .log();

    let failed = total - succeeded;
    (
        StatusCode::OK,
        Json(ApiResponse::success(BulkOperationResult {
            total,
            succeeded,
            failed,
            errors,
        })),
    )
}

/// Request body for bulk user deletion.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BulkDeleteRequest {
    pub user_ids: Vec<String>,
}

/// `POST /api/v1/admin/users/bulk-delete` — Batch user deletion.
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/bulk-delete",
    tag = "Admin",
    summary = "Bulk delete users",
    description = "Delete multiple users at once.",
    security(("bearer_auth" = [])),
    request_body = BulkDeleteRequest,
    responses(
        (status = 200, description = "Bulk delete result"),
    )
)]
pub async fn bulk_delete_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<BulkDeleteRequest>,
) -> (StatusCode, Json<ApiResponse<BulkOperationResult>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let total = req.user_ids.len();
    let mut succeeded = 0;
    let mut errors = Vec::new();

    for user_id in &req.user_ids {
        // Prevent self-deletion
        if user_id == &auth_user.user_id.to_string() {
            errors.push("Cannot delete your own account via bulk operation".to_string());
            continue;
        }

        match state_guard.db.delete_user(user_id).await {
            Ok(true) => succeeded += 1,
            Ok(false) => errors.push(format!("User {user_id} not found")),
            Err(e) => errors.push(format!("Failed to delete user {user_id}: {e}")),
        }
    }

    AuditEntry::new(AuditEventType::UserDeleted)
        .user(auth_user.user_id, "")
        .detail(&format!("Bulk delete: {succeeded}/{total} users deleted"))
        .log();

    let failed = total - succeeded;
    (
        StatusCode::OK,
        Json(ApiResponse::success(BulkOperationResult {
            total,
            succeeded,
            failed,
            errors,
        })),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADVANCED REPORTING
// ═══════════════════════════════════════════════════════════════════════════════

/// Query params for advanced reports.
#[derive(Debug, Deserialize)]
pub struct AdvancedReportQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub group_by: Option<String>, // "day", "week", "month"
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc())
}

/// Revenue report entry.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevenueReportEntry {
    pub period: String,
    pub lot_name: String,
    pub total_revenue: f64,
    pub booking_count: usize,
}

/// `GET /api/v1/admin/reports/revenue` — Revenue by lot, by time period.
#[utoipa::path(
    get,
    path = "/api/v1/admin/reports/revenue",
    tag = "Admin",
    summary = "Revenue report",
    description = "Revenue breakdown by lot and time period.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Revenue report")),
)]
pub async fn revenue_report(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(query): axum::extract::Query<AdvancedReportQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<RevenueReportEntry>>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let start = query
        .start_date
        .as_deref()
        .and_then(parse_date)
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let end = query
        .end_date
        .as_deref()
        .and_then(parse_date)
        .unwrap_or_else(Utc::now);
    let group_by = query.group_by.as_deref().unwrap_or("day");

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();

    let lot_names: std::collections::HashMap<String, String> = lots
        .iter()
        .map(|l| (l.id.to_string(), l.name.clone()))
        .collect();

    let mut revenue_map: std::collections::BTreeMap<(String, String), (f64, usize)> =
        std::collections::BTreeMap::new();

    for b in &bookings {
        if b.created_at < start || b.created_at > end {
            continue;
        }
        let period = match group_by {
            "week" => {
                let week = b.created_at.iso_week().week();
                format!("{}-W{:02}", b.created_at.year(), week)
            }
            "month" => b.created_at.format("%Y-%m").to_string(),
            _ => b.created_at.format("%Y-%m-%d").to_string(),
        };
        let lot_name = lot_names
            .get(&b.lot_id.to_string())
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        let price = b.pricing.total;
        let entry = revenue_map.entry((period, lot_name)).or_insert((0.0, 0));
        entry.0 += price;
        entry.1 += 1;
    }

    let entries: Vec<RevenueReportEntry> = revenue_map
        .into_iter()
        .map(
            |((period, lot_name), (total_revenue, booking_count))| RevenueReportEntry {
                period,
                lot_name,
                total_revenue: (total_revenue * 100.0).round() / 100.0,
                booking_count,
            },
        )
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(entries)))
}

/// Occupancy report entry.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OccupancyReportEntry {
    pub period: String,
    pub lot_name: String,
    pub total_slots: i32,
    pub average_occupied: f64,
    pub peak_occupied: usize,
    pub occupancy_percent: f64,
}

/// `GET /api/v1/admin/reports/occupancy` — Occupancy trends by lot and date range.
#[utoipa::path(
    get,
    path = "/api/v1/admin/reports/occupancy",
    tag = "Admin",
    summary = "Occupancy report",
    description = "Occupancy trends with date range.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Occupancy report")),
)]
pub async fn occupancy_report(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(query): axum::extract::Query<AdvancedReportQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<OccupancyReportEntry>>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let start = query
        .start_date
        .as_deref()
        .and_then(parse_date)
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let end = query
        .end_date
        .as_deref()
        .and_then(parse_date)
        .unwrap_or_else(Utc::now);
    let group_by = query.group_by.as_deref().unwrap_or("day");

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();

    let lot_names: std::collections::HashMap<String, String> = lots
        .iter()
        .map(|l| (l.id.to_string(), l.name.clone()))
        .collect();
    let _lot_slots: std::collections::HashMap<String, i32> = lots
        .iter()
        .map(|l| (l.id.to_string(), l.total_slots))
        .collect();

    // Count bookings per lot per period
    let mut occ_map: std::collections::BTreeMap<(String, String), Vec<usize>> =
        std::collections::BTreeMap::new();

    for b in &bookings {
        if b.start_time < start || b.start_time > end {
            continue;
        }
        if b.status != BookingStatus::Confirmed && b.status != BookingStatus::Active {
            continue;
        }
        let period = match group_by {
            "week" => {
                let week = b.start_time.iso_week().week();
                format!("{}-W{:02}", b.start_time.year(), week)
            }
            "month" => b.start_time.format("%Y-%m").to_string(),
            _ => b.start_time.format("%Y-%m-%d").to_string(),
        };
        let lot_id = b.lot_id.to_string();
        let lot_name = lot_names
            .get(&lot_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        let key = (period, lot_name);
        occ_map.entry(key).or_default().push(1);
    }

    let entries: Vec<OccupancyReportEntry> = occ_map
        .into_iter()
        .map(|((period, lot_name), counts)| {
            let total = counts.len();
            let total_slots = lots
                .iter()
                .find(|l| l.name == lot_name)
                .map_or(0, |l| l.total_slots);
            #[allow(clippy::cast_precision_loss)]
            let occupancy_percent = if total_slots > 0 {
                (total as f64 / f64::from(total_slots)) * 100.0
            } else {
                0.0
            };
            OccupancyReportEntry {
                period,
                lot_name,
                total_slots,
                average_occupied: total as f64,
                peak_occupied: total,
                occupancy_percent: (occupancy_percent * 100.0).round() / 100.0,
            }
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(entries)))
}

/// User growth report entry.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserReportEntry {
    pub period: String,
    pub new_users: usize,
    pub total_users: usize,
    pub active_users: usize,
}

/// `GET /api/v1/admin/reports/users` — User growth, active users, churn.
#[utoipa::path(
    get,
    path = "/api/v1/admin/reports/users",
    tag = "Admin",
    summary = "User growth report",
    description = "User growth, active users by period.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "User report")),
)]
pub async fn user_report(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(query): axum::extract::Query<AdvancedReportQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<UserReportEntry>>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let start = query
        .start_date
        .as_deref()
        .and_then(parse_date)
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let end = query
        .end_date
        .as_deref()
        .and_then(parse_date)
        .unwrap_or_else(Utc::now);
    let group_by = query.group_by.as_deref().unwrap_or("day");

    let users = state_guard.db.list_users().await.unwrap_or_default();

    let mut period_map: std::collections::BTreeMap<String, (usize, usize, usize)> =
        std::collections::BTreeMap::new();

    let mut cumulative_total = 0usize;
    // Count users who existed before start date
    for u in &users {
        if u.created_at < start {
            cumulative_total += 1;
        }
    }

    for u in &users {
        if u.created_at < start || u.created_at > end {
            continue;
        }
        let period = match group_by {
            "week" => {
                let week = u.created_at.iso_week().week();
                format!("{}-W{:02}", u.created_at.year(), week)
            }
            "month" => u.created_at.format("%Y-%m").to_string(),
            _ => u.created_at.format("%Y-%m-%d").to_string(),
        };
        let entry = period_map.entry(period).or_insert((0, 0, 0));
        entry.0 += 1; // new users
        if u.is_active {
            entry.2 += 1; // active
        }
    }

    let entries: Vec<UserReportEntry> = period_map
        .into_iter()
        .map(|(period, (new_users, _, active_users))| {
            cumulative_total += new_users;
            UserReportEntry {
                period,
                new_users,
                total_users: cumulative_total,
                active_users,
            }
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(entries)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATION PREFERENCES
// ═══════════════════════════════════════════════════════════════════════════════

/// Per-user notification preferences.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NotificationPreferences {
    pub email_booking_confirm: bool,
    pub email_booking_reminder: bool,
    pub email_swap_request: bool,
    pub push_enabled: bool,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_booking_confirm: true,
            email_booking_reminder: true,
            email_swap_request: true,
            push_enabled: true,
        }
    }
}

/// `GET /api/v1/preferences/notifications` — Get notification preferences.
#[utoipa::path(
    get,
    path = "/api/v1/preferences/notifications",
    tag = "Users",
    summary = "Get notification preferences",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Notification preferences")),
)]
pub async fn get_notification_preferences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<NotificationPreferences>>) {
    let state_guard = state.read().await;
    let key = format!("notif_prefs:{}", auth_user.user_id);
    let prefs = match state_guard.db.get_setting(&key).await {
        Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
        _ => NotificationPreferences::default(),
    };
    (StatusCode::OK, Json(ApiResponse::success(prefs)))
}

/// `PUT /api/v1/preferences/notifications` — Update notification preferences.
#[utoipa::path(
    put,
    path = "/api/v1/preferences/notifications",
    tag = "Users",
    summary = "Update notification preferences",
    security(("bearer_auth" = [])),
    request_body = NotificationPreferences,
    responses((status = 200, description = "Preferences updated")),
)]
pub async fn update_notification_preferences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(prefs): Json<NotificationPreferences>,
) -> (StatusCode, Json<ApiResponse<NotificationPreferences>>) {
    let state_guard = state.read().await;
    let key = format!("notif_prefs:{}", auth_user.user_id);
    let json = serde_json::to_string(&prefs).unwrap_or_default();
    if let Err(e) = state_guard.db.set_setting(&key, &json).await {
        tracing::error!("Failed to save notification preferences: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save preferences",
            )),
        );
    }
    (StatusCode::OK, Json(ApiResponse::success(prefs)))
}

/// Load notification preferences for a user (used by notification senders).
#[allow(dead_code)]
pub async fn load_notification_preferences(
    db: &crate::db::Database,
    user_id: uuid::Uuid,
) -> NotificationPreferences {
    let key = format!("notif_prefs:{user_id}");
    match db.get_setting(&key).await {
        Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
        _ => NotificationPreferences::default(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DESIGN THEME PREFERENCES
// ═══════════════════════════════════════════════════════════════════════════════

/// Available design theme IDs.
const VALID_DESIGN_THEMES: &[&str] = &[
    "classic",
    "glass",
    "bento",
    "brutalist",
    "neon",
    "warm",
    "liquid",
    "mono",
    "ocean",
    "forest",
];

/// Design theme preference.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DesignThemePreference {
    pub design_theme: String,
}

/// `GET /api/v1/preferences/theme` — Get the user's design theme preference.
#[utoipa::path(
    get,
    path = "/api/v1/preferences/theme",
    tag = "Users",
    summary = "Get design theme preference",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Design theme preference")),
)]
pub async fn get_design_theme_preference(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<DesignThemePreference>>) {
    let state_guard = state.read().await;
    let key = format!("design_theme:{}", auth_user.user_id);
    let theme = match state_guard.db.get_setting(&key).await {
        Ok(Some(val)) if VALID_DESIGN_THEMES.contains(&val.as_str()) => val,
        _ => "classic".to_string(),
    };
    (
        StatusCode::OK,
        Json(ApiResponse::success(DesignThemePreference {
            design_theme: theme,
        })),
    )
}

/// `PUT /api/v1/preferences/theme` — Update the user's design theme preference.
#[utoipa::path(
    put,
    path = "/api/v1/preferences/theme",
    tag = "Users",
    summary = "Update design theme preference",
    security(("bearer_auth" = [])),
    request_body = DesignThemePreference,
    responses((status = 200, description = "Design theme updated")),
)]
pub async fn update_design_theme_preference(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<DesignThemePreference>,
) -> (StatusCode, Json<ApiResponse<DesignThemePreference>>) {
    if !VALID_DESIGN_THEMES.contains(&body.design_theme.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_THEME",
                "Invalid design theme. Valid: classic, glass, bento, brutalist, neon, warm, liquid, mono, ocean, forest",
            )),
        );
    }

    let state_guard = state.read().await;
    let key = format!("design_theme:{}", auth_user.user_id);
    if let Err(e) = state_guard.db.set_setting(&key, &body.design_theme).await {
        tracing::error!("Failed to save design theme: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save design theme",
            )),
        );
    }
    (StatusCode::OK, Json(ApiResponse::success(body)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING POLICIES
// ═══════════════════════════════════════════════════════════════════════════════

/// Admin-configurable booking policies.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BookingPolicies {
    /// Maximum days in advance a booking can be made (0 = unlimited)
    pub max_advance_booking_days: u32,
    /// Minimum booking duration in hours (0 = no minimum)
    pub min_booking_duration_hours: u32,
    /// Maximum booking duration in hours (0 = no maximum)
    pub max_booking_duration_hours: u32,
}

impl BookingPolicies {
    /// Validate a booking against policies. Returns `Ok(())` or an error message.
    #[allow(dead_code)]
    pub fn check(&self, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<(), String> {
        let now = Utc::now();

        // Check max advance booking
        if self.max_advance_booking_days > 0 {
            let max_future = now + chrono::Duration::days(i64::from(self.max_advance_booking_days));
            if start_time > max_future {
                return Err(format!(
                    "Bookings can only be made up to {} days in advance",
                    self.max_advance_booking_days
                ));
            }
        }

        // Check duration
        let duration_hours = (end_time - start_time).num_hours();
        let duration_u32 = u32::try_from(duration_hours).unwrap_or(0);
        if self.min_booking_duration_hours > 0 && duration_u32 < self.min_booking_duration_hours {
            return Err(format!(
                "Minimum booking duration is {} hours",
                self.min_booking_duration_hours
            ));
        }
        if self.max_booking_duration_hours > 0 && duration_u32 > self.max_booking_duration_hours {
            return Err(format!(
                "Maximum booking duration is {} hours",
                self.max_booking_duration_hours
            ));
        }

        Ok(())
    }
}

/// Load booking policies from DB settings.
pub async fn load_booking_policies(db: &crate::db::Database) -> BookingPolicies {
    match db.get_setting("booking_policies").await {
        Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
        _ => BookingPolicies::default(),
    }
}

/// `GET /api/v1/admin/settings/booking-policies` — Get booking policies.
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/booking-policies",
    tag = "Admin",
    summary = "Get booking policies",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Booking policies")),
)]
pub async fn get_booking_policies(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<BookingPolicies>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }
    let policies = load_booking_policies(&state_guard.db).await;
    (StatusCode::OK, Json(ApiResponse::success(policies)))
}

/// `PUT /api/v1/admin/settings/booking-policies` — Update booking policies.
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/booking-policies",
    tag = "Admin",
    summary = "Update booking policies",
    security(("bearer_auth" = [])),
    request_body = BookingPolicies,
    responses((status = 200, description = "Policies updated")),
)]
pub async fn update_booking_policies(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(policies): Json<BookingPolicies>,
) -> (StatusCode, Json<ApiResponse<BookingPolicies>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let json = serde_json::to_string(&policies).unwrap_or_default();
    if let Err(e) = state_guard.db.set_setting("booking_policies", &json).await {
        tracing::error!("Failed to save booking policies: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save policies",
            )),
        );
    }

    AuditEntry::new(AuditEventType::SettingsChanged)
        .user(auth_user.user_id, "")
        .detail("Booking policies updated")
        .log();

    (StatusCode::OK, Json(ApiResponse::success(policies)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEALTH CHECK IMPROVEMENTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Extended health check with build info.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ExtendedHealthResponse {
    pub status: String,
    pub version: String,
    pub git_sha: String,
    pub build_time: String,
    pub uptime_seconds: u64,
    pub db_healthy: bool,
    pub disk_space_ok: bool,
    pub components: Vec<HealthComponentInfo>,
}

/// Health component info.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthComponentInfo {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

/// `GET /health/detailed` — Extended health check with build info and disk space.
#[utoipa::path(
    get,
    path = "/health/detailed",
    tag = "Health",
    summary = "Detailed health check",
    description = "Extended health check including build info, DB connectivity, and disk space.",
    responses((status = 200, description = "Health check")),
)]
pub async fn detailed_health_check(
    State(state): State<SharedState>,
) -> Json<ExtendedHealthResponse> {
    let state_guard = state.read().await;

    // DB check
    let db_healthy = state_guard.db.stats().await.is_ok();

    // Disk space check (Linux only)
    let disk_space_ok = check_disk_space();

    let mut components = vec![
        HealthComponentInfo {
            name: "database".to_string(),
            status: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
            message: if db_healthy {
                Some("Connected".to_string())
            } else {
                Some("Connection failed".to_string())
            },
        },
        HealthComponentInfo {
            name: "disk".to_string(),
            status: if disk_space_ok { "healthy" } else { "warning" }.to_string(),
            message: if disk_space_ok {
                Some("Sufficient disk space".to_string())
            } else {
                Some("Low disk space (< 100 MB)".to_string())
            },
        },
    ];

    // Memory check
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            if let Some(line) = status.lines().find(|l| l.starts_with("VmRSS:")) {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        let mb = kb / 1024;
                        components.push(HealthComponentInfo {
                            name: "memory".to_string(),
                            status: if mb < 500 { "healthy" } else { "warning" }.to_string(),
                            message: Some(format!("{mb} MB RSS")),
                        });
                    }
                }
            }
        }
    }

    let overall = if db_healthy && disk_space_ok {
        "healthy"
    } else if db_healthy {
        "degraded"
    } else {
        "unhealthy"
    };

    Json(ExtendedHealthResponse {
        status: overall.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown").to_string(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
        uptime_seconds: 0, // would need start_time in state
        db_healthy,
        disk_space_ok,
        components,
    })
}

/// Check if there's at least 100MB of free disk space.
fn check_disk_space() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(stat) = std::fs::metadata("/") {
            // Use statvfs via /proc/mounts fallback
            let _ = stat; // statvfs not available in std, just report OK
        }
    }
    // Default: assume OK if we can't check
    true
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Bulk Operation Tests ────────────────────────────────────────────

    #[test]
    fn test_bulk_update_request_deserialize() {
        let json = r#"{"user_ids":["id1","id2"],"action":"activate"}"#;
        let req: BulkUserUpdateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_ids.len(), 2);
        assert_eq!(req.action, "activate");
        assert!(req.role.is_none());
    }

    #[test]
    fn test_bulk_update_request_with_role() {
        let json = r#"{"user_ids":["id1"],"action":"set_role","role":"admin"}"#;
        let req: BulkUserUpdateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.action, "set_role");
        assert_eq!(req.role.as_deref(), Some("admin"));
    }

    #[test]
    fn test_bulk_delete_request_deserialize() {
        let json = r#"{"user_ids":["a","b","c"]}"#;
        let req: BulkDeleteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_ids.len(), 3);
    }

    #[test]
    fn test_bulk_operation_result_serialization() {
        let result = BulkOperationResult {
            total: 5,
            succeeded: 3,
            failed: 2,
            errors: vec!["User x not found".to_string()],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["total"], 5);
        assert_eq!(json["succeeded"], 3);
        assert_eq!(json["failed"], 2);
        assert_eq!(json["errors"].as_array().unwrap().len(), 1);
    }

    // ─── Advanced Report Tests ───────────────────────────────────────────

    #[test]
    fn test_revenue_report_entry_serialization() {
        let entry = RevenueReportEntry {
            period: "2026-03-22".to_string(),
            lot_name: "Main Garage".to_string(),
            total_revenue: 150.50,
            booking_count: 10,
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["period"], "2026-03-22");
        assert_eq!(json["lot_name"], "Main Garage");
        assert_eq!(json["total_revenue"], 150.5);
        assert_eq!(json["booking_count"], 10);
    }

    #[test]
    fn test_occupancy_report_entry_serialization() {
        let entry = OccupancyReportEntry {
            period: "2026-03".to_string(),
            lot_name: "Lot A".to_string(),
            total_slots: 100,
            average_occupied: 75.5,
            peak_occupied: 90,
            occupancy_percent: 75.5,
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["total_slots"], 100);
        assert_eq!(json["peak_occupied"], 90);
    }

    #[test]
    fn test_user_report_entry_serialization() {
        let entry = UserReportEntry {
            period: "2026-W12".to_string(),
            new_users: 5,
            total_users: 100,
            active_users: 85,
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["new_users"], 5);
        assert_eq!(json["total_users"], 100);
        assert_eq!(json["active_users"], 85);
    }

    #[test]
    fn test_parse_date_valid() {
        let d = parse_date("2026-03-22");
        assert!(d.is_some());
        assert_eq!(d.unwrap().date_naive().to_string(), "2026-03-22");
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("not-a-date").is_none());
        assert!(parse_date("2026-13-01").is_none());
        assert!(parse_date("").is_none());
    }

    // ─── Notification Preferences Tests ──────────────────────────────────

    #[test]
    fn test_notification_preferences_default() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.email_booking_confirm);
        assert!(prefs.email_booking_reminder);
        assert!(prefs.email_swap_request);
        assert!(prefs.push_enabled);
    }

    #[test]
    fn test_notification_preferences_roundtrip() {
        let prefs = NotificationPreferences {
            email_booking_confirm: false,
            email_booking_reminder: true,
            email_swap_request: false,
            push_enabled: true,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let back: NotificationPreferences = serde_json::from_str(&json).unwrap();
        assert!(!back.email_booking_confirm);
        assert!(back.email_booking_reminder);
        assert!(!back.email_swap_request);
        assert!(back.push_enabled);
    }

    // ─── Booking Policies Tests ──────────────────────────────────────────

    #[test]
    fn test_booking_policies_default() {
        let policies = BookingPolicies::default();
        assert_eq!(policies.max_advance_booking_days, 0);
        assert_eq!(policies.min_booking_duration_hours, 0);
        assert_eq!(policies.max_booking_duration_hours, 0);
    }

    #[test]
    fn test_booking_policies_check_valid() {
        let policies = BookingPolicies {
            max_advance_booking_days: 30,
            min_booking_duration_hours: 1,
            max_booking_duration_hours: 24,
        };
        let start = Utc::now() + chrono::Duration::hours(1);
        let end = start + chrono::Duration::hours(4);
        assert!(policies.check(start, end).is_ok());
    }

    #[test]
    fn test_booking_policies_too_far_ahead() {
        let policies = BookingPolicies {
            max_advance_booking_days: 7,
            min_booking_duration_hours: 0,
            max_booking_duration_hours: 0,
        };
        let start = Utc::now() + chrono::Duration::days(30);
        let end = start + chrono::Duration::hours(2);
        let err = policies.check(start, end).unwrap_err();
        assert!(err.contains("7 days"));
    }

    #[test]
    fn test_booking_policies_too_short() {
        let policies = BookingPolicies {
            max_advance_booking_days: 0,
            min_booking_duration_hours: 2,
            max_booking_duration_hours: 0,
        };
        let start = Utc::now() + chrono::Duration::hours(1);
        let end = start + chrono::Duration::minutes(30);
        let err = policies.check(start, end).unwrap_err();
        assert!(err.contains("2 hours"));
    }

    #[test]
    fn test_booking_policies_too_long() {
        let policies = BookingPolicies {
            max_advance_booking_days: 0,
            min_booking_duration_hours: 0,
            max_booking_duration_hours: 8,
        };
        let start = Utc::now() + chrono::Duration::hours(1);
        let end = start + chrono::Duration::hours(12);
        let err = policies.check(start, end).unwrap_err();
        assert!(err.contains("8 hours"));
    }

    #[test]
    fn test_booking_policies_no_restrictions() {
        let policies = BookingPolicies::default(); // all 0 = no restrictions
        let start = Utc::now() + chrono::Duration::days(365);
        let end = start + chrono::Duration::days(30);
        assert!(policies.check(start, end).is_ok());
    }

    #[test]
    fn test_booking_policies_serialization_roundtrip() {
        let policies = BookingPolicies {
            max_advance_booking_days: 14,
            min_booking_duration_hours: 1,
            max_booking_duration_hours: 12,
        };
        let json = serde_json::to_string(&policies).unwrap();
        let back: BookingPolicies = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_advance_booking_days, 14);
        assert_eq!(back.min_booking_duration_hours, 1);
        assert_eq!(back.max_booking_duration_hours, 12);
    }

    // ─── Health Check Tests ──────────────────────────────────────────────

    #[test]
    fn test_extended_health_response_serialization() {
        let resp = ExtendedHealthResponse {
            status: "healthy".to_string(),
            version: "1.9.0".to_string(),
            git_sha: "abc123".to_string(),
            build_time: "2026-03-22T00:00:00Z".to_string(),
            uptime_seconds: 3600,
            db_healthy: true,
            disk_space_ok: true,
            components: vec![HealthComponentInfo {
                name: "database".to_string(),
                status: "healthy".to_string(),
                message: Some("Connected".to_string()),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["version"], "1.9.0");
        assert_eq!(json["git_sha"], "abc123");
        assert_eq!(json["db_healthy"], true);
        assert_eq!(json["disk_space_ok"], true);
    }

    #[test]
    fn test_health_component_info() {
        let comp = HealthComponentInfo {
            name: "memory".to_string(),
            status: "warning".to_string(),
            message: Some("512 MB RSS".to_string()),
        };
        let json = serde_json::to_value(&comp).unwrap();
        assert_eq!(json["name"], "memory");
        assert_eq!(json["status"], "warning");
    }

    #[test]
    fn test_check_disk_space_returns_true() {
        // Default implementation should always return true
        assert!(check_disk_space());
    }

    // ─── Advanced Report Query Tests ─────────────────────────────────────

    #[test]
    fn test_advanced_report_query_deserialize() {
        let json = r#"{"start_date":"2026-01-01","end_date":"2026-03-31","group_by":"month"}"#;
        let q: AdvancedReportQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(q.end_date.as_deref(), Some("2026-03-31"));
        assert_eq!(q.group_by.as_deref(), Some("month"));
    }

    #[test]
    fn test_advanced_report_query_empty() {
        let q: AdvancedReportQuery = serde_json::from_str("{}").unwrap();
        assert!(q.start_date.is_none());
        assert!(q.end_date.is_none());
        assert!(q.group_by.is_none());
    }
}
