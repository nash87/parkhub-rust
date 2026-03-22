//! Data Import/Export Suite
//!
//! - `POST /api/v1/admin/import/users` — CSV/JSON bulk user import
//! - `POST /api/v1/admin/import/lots` — CSV/JSON lot import with slots
//! - `GET  /api/v1/admin/export/users` — CSV export all users (enhanced)
//! - `GET  /api/v1/admin/export/lots` — CSV export all lots with stats
//! - `GET  /api/v1/admin/export/bookings` — CSV export bookings (date range)

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

use parkhub_common::{ApiResponse, User, UserPreferences, UserRole};

use super::{check_admin, AuthUser};
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Import types
// ─────────────────────────────────────────────────────────────────────────────

const MAX_IMPORT_ROWS: usize = 1000;

/// Result of a bulk import operation.
#[derive(Debug, Serialize)]
pub struct DataImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<DataImportError>,
}

/// Describes a single row that failed during import.
#[derive(Debug, Serialize)]
pub struct DataImportError {
    pub row: usize,
    pub field: String,
    pub message: String,
}

/// Import request body (CSV or JSON, base64-encoded CSV data or raw JSON array).
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    /// "csv" or "json"
    pub format: String,
    /// Base64-encoded CSV or JSON array as string
    pub data: String,
}

/// Lot import entry (JSON)
#[derive(Debug, Deserialize)]
pub struct LotImportEntry {
    pub name: String,
    pub address: Option<String>,
    pub total_slots: Option<u32>,
    pub hourly_rate: Option<f64>,
    pub daily_max: Option<f64>,
    pub currency: Option<String>,
}

/// User import entry (JSON)
#[derive(Debug, Deserialize)]
pub struct UserImportEntry {
    pub username: String,
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub password: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Export query params
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ExportParams {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub format: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV helpers
// ─────────────────────────────────────────────────────────────────────────────

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

fn csv_response(
    filename: &str,
    body: String,
) -> (StatusCode, [(header::HeaderName, String); 2], String) {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    )
}

fn error_response(
    status: StatusCode,
    msg: &str,
) -> (StatusCode, [(header::HeaderName, String); 2], String) {
    (
        status,
        [
            (header::CONTENT_TYPE, "text/plain".to_string()),
            (header::CONTENT_DISPOSITION, "inline".to_string()),
        ],
        msg.to_string(),
    )
}

fn parse_role(s: &str) -> UserRole {
    match s.to_lowercase().trim() {
        "admin" => UserRole::Admin,
        "superadmin" | "super_admin" => UserRole::SuperAdmin,
        "premium" => UserRole::Premium,
        _ => UserRole::User,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/v1/admin/import/users
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/admin/import/users` — bulk user import (CSV or JSON)
#[utoipa::path(post, path = "/api/v1/admin/import/users", tag = "Admin",
    summary = "Bulk user import",
    description = "Import users from CSV (base64) or JSON. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Import result"),
        (status = 400, description = "Invalid input"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn import_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ImportRequest>,
) -> (StatusCode, Json<ApiResponse<DataImportResult>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let entries: Vec<UserImportEntry> = if req.format == "json" {
        match serde_json::from_str(&req.data) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("INVALID_JSON", &e.to_string())),
                )
            }
        }
    } else {
        // CSV: base64-decode then parse
        let csv_data = match base64::engine::general_purpose::STANDARD.decode(&req.data) {
            Ok(d) => String::from_utf8_lossy(&d).to_string(),
            Err(_) => req.data.clone(), // try raw
        };
        let mut rows = Vec::new();
        for (i, line) in csv_data.lines().enumerate() {
            if i == 0 && line.to_lowercase().contains("username") {
                continue; // skip header
            }
            let fields: Vec<&str> = line.splitn(5, ',').collect();
            if fields.len() < 2 {
                continue;
            }
            rows.push(UserImportEntry {
                username: fields[0].trim().to_string(),
                email: fields[1].trim().to_string(),
                name: fields.get(2).map(|s| s.trim().to_string()),
                role: fields.get(3).map(|s| s.trim().to_string()),
                password: fields.get(4).map(|s| s.trim().to_string()),
            });
        }
        rows
    };

    if entries.len() > MAX_IMPORT_ROWS {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "TOO_MANY_ROWS",
                &format!("Maximum {MAX_IMPORT_ROWS} rows per import"),
            )),
        );
    }

    let mut result = DataImportResult {
        imported: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    use base64::Engine;

    for (i, entry) in entries.iter().enumerate() {
        let row = i + 1;

        if entry.username.is_empty() {
            result.errors.push(DataImportError {
                row,
                field: "username".to_string(),
                message: "Username is required".to_string(),
            });
            continue;
        }
        if entry.email.is_empty() || !entry.email.contains('@') {
            result.errors.push(DataImportError {
                row,
                field: "email".to_string(),
                message: "Valid email is required".to_string(),
            });
            continue;
        }

        // Check duplicates
        if state_guard
            .db
            .get_user_by_username(&entry.username)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            result.skipped += 1;
            continue;
        }
        if state_guard
            .db
            .get_user_by_email(&entry.email)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            result.skipped += 1;
            continue;
        }

        let password = entry.password.as_deref().unwrap_or("ParkHub2026!");
        let password_hash = super::hash_password_simple(password);
        let role = parse_role(entry.role.as_deref().unwrap_or("user"));

        let user = User {
            id: uuid::Uuid::new_v4(),
            username: entry.username.clone(),
            email: entry.email.clone(),
            name: entry.name.clone().unwrap_or_else(|| entry.username.clone()),
            password_hash,
            role,
            is_active: true,
            phone: None,
            picture: None,
            preferences: UserPreferences::default(),
            credits_balance: 0,
            credits_monthly_quota: 0,
            credits_last_refilled: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            tenant_id: None,
            accessibility_needs: None,
        };

        match state_guard.db.save_user(&user).await {
            Ok(_) => result.imported += 1,
            Err(e) => result.errors.push(DataImportError {
                row,
                field: String::new(),
                message: e.to_string(),
            }),
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/v1/admin/import/lots
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/admin/import/lots` — bulk lot import (CSV or JSON)
#[utoipa::path(post, path = "/api/v1/admin/import/lots", tag = "Admin",
    summary = "Bulk lot import",
    description = "Import parking lots from CSV (base64) or JSON. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Import result"),
        (status = 400, description = "Invalid input"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn import_lots(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ImportRequest>,
) -> (StatusCode, Json<ApiResponse<DataImportResult>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let entries: Vec<LotImportEntry> = if req.format == "json" {
        match serde_json::from_str(&req.data) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("INVALID_JSON", &e.to_string())),
                )
            }
        }
    } else {
        let csv_data = match base64::engine::general_purpose::STANDARD.decode(&req.data) {
            Ok(d) => String::from_utf8_lossy(&d).to_string(),
            Err(_) => req.data.clone(),
        };
        let mut rows = Vec::new();
        for (i, line) in csv_data.lines().enumerate() {
            if i == 0 && line.to_lowercase().contains("name") {
                continue;
            }
            let fields: Vec<&str> = line.splitn(6, ',').collect();
            if fields.is_empty() || fields[0].trim().is_empty() {
                continue;
            }
            rows.push(LotImportEntry {
                name: fields[0].trim().to_string(),
                address: fields.get(1).map(|s| s.trim().to_string()),
                total_slots: fields.get(2).and_then(|s| s.trim().parse().ok()),
                hourly_rate: fields.get(3).and_then(|s| s.trim().parse().ok()),
                daily_max: fields.get(4).and_then(|s| s.trim().parse().ok()),
                currency: fields.get(5).map(|s| s.trim().to_string()),
            });
        }
        rows
    };

    if entries.len() > MAX_IMPORT_ROWS {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "TOO_MANY_ROWS",
                &format!("Maximum {MAX_IMPORT_ROWS} rows per import"),
            )),
        );
    }

    let mut result = DataImportResult {
        imported: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    use base64::Engine;

    for (i, entry) in entries.iter().enumerate() {
        let row = i + 1;

        if entry.name.is_empty() {
            result.errors.push(DataImportError {
                row,
                field: "name".to_string(),
                message: "Lot name is required".to_string(),
            });
            continue;
        }

        let total_slots = entry.total_slots.unwrap_or(10);
        let lot = parkhub_common::ParkingLot {
            id: uuid::Uuid::new_v4(),
            name: entry.name.clone(),
            address: entry.address.clone().unwrap_or_default(),
            total_slots,
            available_slots: total_slots,
            status: parkhub_common::LotStatus::Open,
            hourly_rate: entry.hourly_rate.unwrap_or(2.0),
            daily_max: entry.daily_max.unwrap_or(20.0),
            monthly_pass: None,
            currency: entry.currency.clone().unwrap_or_else(|| "EUR".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            latitude: None,
            longitude: None,
            tenant_id: None,
        };

        match state_guard.db.save_parking_lot(&lot).await {
            Ok(_) => {
                // Create slots for the lot
                for slot_num in 1..=total_slots {
                    #[allow(clippy::cast_possible_truncation)]
                    let slot = parkhub_common::ParkingSlot {
                        id: uuid::Uuid::new_v4(),
                        lot_id: lot.id,
                        floor_id: lot.floors.first().map_or_else(uuid::Uuid::new_v4, |f| f.id),
                        slot_number: slot_num as i32,
                        row: ((slot_num - 1) / 10 + 1) as i32,
                        column: ((slot_num - 1) % 10 + 1) as i32,
                        slot_type: parkhub_common::SlotType::Standard,
                        status: parkhub_common::SlotStatus::Available,
                        current_booking: None,
                        features: Vec::new(),
                        position: parkhub_common::SlotPosition {
                            x: (((slot_num - 1) % 10) as f32) * 3.0,
                            y: (((slot_num - 1) / 10) as f32) * 5.0,
                            width: 2.5,
                            height: 5.0,
                            rotation: 0.0,
                        },
                        is_accessible: false,
                    };
                    let _ = state_guard.db.save_parking_slot(&slot).await;
                }
                result.imported += 1;
            }
            Err(e) => result.errors.push(DataImportError {
                row,
                field: String::new(),
                message: e.to_string(),
            }),
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/export/lots
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/data/export/lots` — export lots with stats as CSV
#[utoipa::path(get, path = "/api/v1/admin/data/export/lots", tag = "Admin",
    summary = "Export lots as CSV",
    description = "Download all parking lots with stats as CSV. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn export_lots_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return error_response(status, msg);
    }

    let lots = match state_guard.db.list_parking_lots().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to list lots for export: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to export lots");
        }
    };

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut csv = String::from(
        "id,name,address,total_slots,available_slots,status,hourly_rate,daily_max,currency,total_bookings,created_at\n",
    );

    for lot in &lots {
        let booking_count = bookings.iter().filter(|b| b.lot_id == lot.id).count();
        let _ = write!(
            csv,
            "{},{},{},{},{},{},{:.2},{:.2},{},{},{}\n",
            lot.id,
            csv_escape(&lot.name),
            csv_escape(&lot.address),
            lot.total_slots,
            lot.available_slots,
            csv_escape(&format!("{:?}", lot.status).to_lowercase()),
            lot.hourly_rate,
            lot.daily_max,
            csv_escape(&lot.currency),
            booking_count,
            lot.created_at.to_rfc3339(),
        );
    }
    drop(state_guard);

    csv_response("lots.csv", csv)
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/data/export/bookings
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/data/export/bookings` — export bookings as CSV (date range)
#[utoipa::path(get, path = "/api/v1/admin/data/export/bookings", tag = "Admin",
    summary = "Export bookings as CSV (enhanced)",
    description = "Download bookings as CSV with date range filter. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn export_bookings_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<ExportParams>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return error_response(status, msg);
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings for export: {e}");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to export bookings",
            );
        }
    };

    let mut csv = String::from(
        "id,user_id,lot_id,slot_number,start_time,end_time,status,vehicle_plate,total,currency,payment_status\n",
    );

    for b in &bookings {
        if let Some(from) = params.from {
            if b.start_time.date_naive() < from {
                continue;
            }
        }
        if let Some(to) = params.to {
            if b.start_time.date_naive() > to {
                continue;
            }
        }

        let lot_name = match state_guard.db.get_parking_lot(&b.lot_id.to_string()).await {
            Ok(Some(l)) => l.name,
            _ => b.lot_id.to_string(),
        };

        let _ = write!(
            csv,
            "{},{},{},{},{},{},{},{},{:.2},{},{}\n",
            b.id,
            b.user_id,
            csv_escape(&lot_name),
            b.slot_number,
            b.start_time.to_rfc3339(),
            b.end_time.to_rfc3339(),
            csv_escape(&format!("{:?}", b.status).to_lowercase()),
            csv_escape(&b.vehicle.license_plate),
            b.pricing.total,
            csv_escape(&b.pricing.currency),
            csv_escape(&format!("{:?}", b.pricing.payment_status).to_lowercase()),
        );
    }
    drop(state_guard);

    csv_response("bookings.csv", csv)
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/data/export/users
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/data/export/users` — export all users as CSV (enhanced)
#[utoipa::path(get, path = "/api/v1/admin/data/export/users", tag = "Admin",
    summary = "Export users as CSV (enhanced)",
    description = "Download all users as CSV with booking stats. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn export_users_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return error_response(status, msg);
    }

    let users = match state_guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users for export: {e}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to export users");
        }
    };

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut csv = String::from(
        "id,username,email,name,role,is_active,credits_balance,total_bookings,created_at,last_login\n",
    );

    for u in &users {
        let booking_count = bookings.iter().filter(|b| b.user_id == u.id).count();
        let _ = write!(
            csv,
            "{},{},{},{},{},{},{},{},{},{}\n",
            u.id,
            csv_escape(&u.username),
            csv_escape(&u.email),
            csv_escape(&u.name),
            csv_escape(&format!("{:?}", u.role).to_lowercase()),
            u.is_active,
            u.credits_balance,
            booking_count,
            u.created_at.to_rfc3339(),
            u.last_login.map_or_else(String::new, |d| d.to_rfc3339()),
        );
    }
    drop(state_guard);

    csv_response("users.csv", csv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn test_csv_escape_injection() {
        assert_eq!(csv_escape("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(csv_escape("+cmd"), "'+cmd");
    }

    #[test]
    fn test_csv_escape_special_chars() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn test_parse_role_variants() {
        assert!(matches!(parse_role("admin"), UserRole::Admin));
        assert!(matches!(parse_role("superadmin"), UserRole::SuperAdmin));
        assert!(matches!(parse_role("super_admin"), UserRole::SuperAdmin));
        assert!(matches!(parse_role("premium"), UserRole::Premium));
        assert!(matches!(parse_role("user"), UserRole::User));
        assert!(matches!(parse_role("unknown"), UserRole::User));
    }

    #[test]
    fn test_import_request_deserialization() {
        let json = r#"{"format":"csv","data":"dXNlcm5hbWUsZW1haWw="}"#;
        let req: ImportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.format, "csv");
        assert_eq!(req.data, "dXNlcm5hbWUsZW1haWw=");
    }

    #[test]
    fn test_user_import_entry_json() {
        let json = r#"[{"username":"alice","email":"alice@test.com","name":"Alice","role":"admin","password":"secret123"}]"#;
        let entries: Vec<UserImportEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].username, "alice");
        assert_eq!(entries[0].role.as_deref(), Some("admin"));
    }

    #[test]
    fn test_lot_import_entry_json() {
        let json =
            r#"[{"name":"Lot A","address":"123 Main St","total_slots":50,"hourly_rate":3.50}]"#;
        let entries: Vec<LotImportEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Lot A");
        assert_eq!(entries[0].total_slots, Some(50));
        assert_eq!(entries[0].hourly_rate, Some(3.5));
    }

    #[test]
    fn test_data_import_result_serialization() {
        let result = DataImportResult {
            imported: 5,
            skipped: 2,
            errors: vec![DataImportError {
                row: 3,
                field: "email".to_string(),
                message: "Invalid email".to_string(),
            }],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["imported"], 5);
        assert_eq!(json["skipped"], 2);
        assert_eq!(json["errors"][0]["row"], 3);
        assert_eq!(json["errors"][0]["field"], "email");
    }

    #[test]
    fn test_export_params_defaults() {
        let params: ExportParams = serde_json::from_str("{}").unwrap();
        assert!(params.from.is_none());
        assert!(params.to.is_none());
        assert!(params.format.is_none());
    }
}
