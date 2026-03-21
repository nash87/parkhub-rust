//! Admin settings, feature flags, and use-case theme configuration.

use axum::{extract::State, http::StatusCode, Extension, Json};
use serde::Deserialize;

use parkhub_common::ApiResponse;

use crate::audit::{AuditEntry, AuditEventType};

use super::{check_admin, AuthUser, SharedState};

/// All admin settings with their default values.
pub const ADMIN_SETTINGS: &[(&str, &str)] = &[
    ("company_name", "ParkHub"),
    ("use_case", "company"),
    ("self_registration", "true"),
    ("license_plate_mode", "optional"),
    ("display_name_format", "first_name"),
    ("max_bookings_per_day", "0"),
    ("allow_guest_bookings", "false"),
    ("auto_release_enabled", "false"),
    ("auto_release_minutes", "30"),
    ("require_vehicle", "false"),
    ("waitlist_enabled", "true"),
    ("min_booking_duration_hours", "0"),
    ("max_booking_duration_hours", "0"),
    ("credits_enabled", "false"),
    ("credits_per_booking", "1"),
];

/// Read a single admin setting from DB, falling back to its default.
pub async fn read_admin_setting(db: &crate::db::Database, key: &str) -> String {
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
pub fn use_case_theme(key: &str) -> serde_json::Value {
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

/// `GET /api/v1/admin/settings/use-case` — return current use-case with theme config
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/use-case",
    tag = "Admin",
    summary = "Get use-case configuration",
    description = "Return current use-case with theme config. Admin only.",
    security(("bearer_auth" = []))
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

/// `GET /api/v1/admin/settings` — return all settings (merged defaults + stored values)
#[utoipa::path(get, path = "/api/v1/admin/settings", tag = "Admin",
    summary = "Get system settings (admin)", description = "Returns all system settings. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Settings"), (status = 403, description = "Forbidden"))
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

/// `PUT /api/v1/admin/settings` — update one or more settings (admin only)
#[utoipa::path(put, path = "/api/v1/admin/settings", tag = "Admin",
    summary = "Update system settings (admin)", description = "Saves system settings. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Saved"), (status = 403, description = "Forbidden"))
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

        let value_str = val.as_str().map_or_else(
            || val.to_string().trim_matches('"').to_string(),
            String::from,
        );

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

// ─── Feature Flags ───────────────────────────────────────────────────────────

/// All available feature module IDs.
const FEATURE_MODULES: &[&str] = &[
    "credits",
    "absences",
    "vehicles",
    "analytics",
    "team_view",
    "booking_types",
    "invoices",
    "self_registration",
    "generative_bg",
    "micro_animations",
    "fab_quick_actions",
    "rich_empty_states",
    "onboarding_hints",
];

/// Default enabled features (business use case).
const DEFAULT_FEATURES: &[&str] = &[
    "credits",
    "absences",
    "vehicles",
    "analytics",
    "team_view",
    "booking_types",
    "invoices",
    "generative_bg",
    "micro_animations",
    "fab_quick_actions",
    "rich_empty_states",
    "onboarding_hints",
];

const SETTINGS_FEATURES_KEY: &str = "features_enabled";

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

/// `GET /api/v1/features` — public endpoint returning enabled features
#[utoipa::path(get, path = "/api/v1/features", tag = "Public",
    summary = "Get enabled feature flags",
    description = "Returns enabled and available features. No auth required.",
    responses((status = 200, description = "Success"))
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

/// `GET /api/v1/theme` — public: return current use-case theme (no auth required)
#[utoipa::path(get, path = "/api/v1/theme", tag = "Public",
    summary = "Get current theme",
    description = "Returns theme and company name. No auth required.",
    responses((status = 200, description = "Success"))
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

/// `GET /api/v1/admin/features` — admin: get features with full metadata
#[utoipa::path(get, path = "/api/v1/admin/features", tag = "Admin",
    summary = "Get feature flags (admin)",
    description = "Returns feature modules with status. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
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

/// `PUT /api/v1/admin/features` — admin: update enabled features
#[utoipa::path(put, path = "/api/v1/admin/features", tag = "Admin",
    summary = "Update feature flags (admin)",
    description = "Sets enabled feature modules. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_features_request() {
        let json = r#"{"enabled":["credits","absences","vehicles"]}"#;
        let req: UpdateFeaturesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled.len(), 3);
        assert!(req.enabled.contains(&"credits".to_string()));
    }

    #[test]
    fn test_update_features_request_empty() {
        let json = r#"{"enabled":[]}"#;
        let req: UpdateFeaturesRequest = serde_json::from_str(json).unwrap();
        assert!(req.enabled.is_empty());
    }
}
