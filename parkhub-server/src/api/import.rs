//! Import endpoints: bulk CSV user creation and iCal absence import.
//!
//! - `POST /api/v1/admin/users/import` — import users from CSV (admin only)
//! - `POST /api/v1/absences/import/ical` — import absences from iCal (user-scoped)

use axum::{extract::State, http::StatusCode, Extension, Json};
use base64::Engine;
use chrono::Utc;
use uuid::Uuid;

use parkhub_common::models::{Absence, AbsenceType};
use parkhub_common::{ApiResponse, User, UserPreferences, UserRole};

use super::hash_password_simple;
use super::{check_admin, AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

const MAX_IMPORT_ROWS: usize = 500;

// ─────────────────────────────────────────────────────────────────────────────
// Response types
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a bulk CSV user import operation.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ImportResult {
    /// Number of users successfully imported.
    pub imported: usize,
    /// Number of rows skipped (duplicates).
    pub skipped: usize,
    /// Rows that failed validation or could not be imported.
    pub errors: Vec<ImportError>,
}

/// Describes a single row that failed during import.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ImportError {
    /// 1-based row number in the CSV (excluding the header).
    pub row: usize,
    /// The field that caused the failure (empty string if row-level).
    pub field: String,
    /// Human-readable error message.
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV parsing helpers
// ─────────────────────────────────────────────────────────────────────────────

/// One parsed (but not yet validated) CSV data row.
#[derive(Debug)]
struct CsvRow {
    username: String,
    email: String,
    name: String,
    role: String,
    password: String,
}

/// Parse a single CSV data line into a [`CsvRow`].
///
/// Expected column order: `username,email,name,role,password`
/// `role` and `password` are optional (may be empty).
fn parse_csv_line(line: &str) -> Result<CsvRow, (String, String)> {
    let fields: Vec<&str> = line.splitn(5, ',').collect();

    if fields.len() < 2 {
        return Err((
            String::new(),
            "Row must have at least username and email columns".to_string(),
        ));
    }

    let username = fields[0].trim().to_string();
    let email = fields[1].trim().to_string();
    let name = fields
        .get(2)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let role = fields
        .get(3)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let password = fields
        .get(4)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if username.is_empty() {
        return Err(("username".to_string(), "username is required".to_string()));
    }
    if email.is_empty() {
        return Err(("email".to_string(), "email is required".to_string()));
    }

    Ok(CsvRow {
        username,
        email,
        name,
        role,
        password,
    })
}

/// Parse a role string into a [`UserRole`], defaulting to [`UserRole::User`].
fn parse_role(role_str: &str) -> UserRole {
    match role_str.to_lowercase().as_str() {
        "premium" => UserRole::Premium,
        "admin" => UserRole::Admin,
        "superadmin" | "super_admin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    }
}

/// Generate a random password of 16 URL-safe characters.
fn generate_password() -> String {
    let mut bytes = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    // base64url without padding — 16 chars
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/admin/users/import` — bulk-import users from a CSV body (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/import",
    tag = "Admin",
    summary = "Bulk import users from CSV",
    description = "Upload a plain-text CSV body to create multiple users at once. \
        Column order: `username,email,name,role,password` (role and password optional). \
        Maximum 500 rows per request. Admin only.",
    request_body(
        content = String,
        content_type = "text/plain",
        description = "CSV data with header row: username,email,name,role,password"
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Import completed (partial success possible)", body = ImportResult),
        (status = 400, description = "Empty or oversized CSV"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn import_users_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    body: String,
) -> (StatusCode, Json<ApiResponse<ImportResult>>) {
    let state_guard = state.read().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Collect non-empty lines, skip header
    let mut lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_CSV", "CSV body is empty")),
        );
    }

    // Skip header row if present (first field is "username" or "user")
    {
        let first = lines[0].to_lowercase();
        if first.starts_with("username") || first.starts_with("user,") {
            lines.remove(0);
        }
    }

    if lines.is_empty() {
        return (
            StatusCode::OK,
            Json(ApiResponse::success(ImportResult {
                imported: 0,
                skipped: 0,
                errors: vec![],
            })),
        );
    }

    if lines.len() > MAX_IMPORT_ROWS {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "TOO_MANY_ROWS",
                format!("CSV exceeds maximum of {MAX_IMPORT_ROWS} rows"),
            )),
        );
    }

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<ImportError> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let row_num = idx + 1; // 1-based

        // Parse line
        let csv_row = match parse_csv_line(line) {
            Ok(r) => r,
            Err((field, message)) => {
                errors.push(ImportError {
                    row: row_num,
                    field,
                    message,
                });
                continue;
            }
        };

        // Duplicate check: username
        match state_guard.db.get_user_by_username(&csv_row.username).await {
            Ok(Some(_)) => {
                skipped += 1;
                continue;
            }
            Err(e) => {
                tracing::error!("DB error checking username {}: {}", csv_row.username, e);
                errors.push(ImportError {
                    row: row_num,
                    field: "username".to_string(),
                    message: "Database error while checking username".to_string(),
                });
                continue;
            }
            Ok(None) => {}
        }

        // Duplicate check: email
        match state_guard.db.get_user_by_email(&csv_row.email).await {
            Ok(Some(_)) => {
                skipped += 1;
                continue;
            }
            Err(e) => {
                tracing::error!("DB error checking email {}: {}", csv_row.email, e);
                errors.push(ImportError {
                    row: row_num,
                    field: "email".to_string(),
                    message: "Database error while checking email".to_string(),
                });
                continue;
            }
            Ok(None) => {}
        }

        // Resolve password
        let raw_password = if csv_row.password.is_empty() {
            generate_password()
        } else {
            csv_row.password.clone()
        };

        // Hash password
        let password_hash = match hash_password_simple(&raw_password).await {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to hash password for row {}: {}", row_num, e);
                errors.push(ImportError {
                    row: row_num,
                    field: "password".to_string(),
                    message: "Failed to hash password".to_string(),
                });
                continue;
            }
        };

        // Build user
        let now = Utc::now();
        let user = User {
            id: Uuid::new_v4(),
            username: csv_row.username,
            email: csv_row.email,
            password_hash,
            name: if csv_row.name.is_empty() {
                "Imported User".to_string()
            } else {
                csv_row.name
            },
            picture: None,
            phone: None,
            role: parse_role(&csv_row.role),
            created_at: now,
            updated_at: now,
            last_login: None,
            preferences: UserPreferences {
                default_duration_minutes: Some(60),
                favorite_slots: vec![],
                notifications_enabled: true,
                email_reminders: true,
                language: "en".to_string(),
                theme: "dark".to_string(),
            },
            is_active: true,
            credits_balance: 40,
            credits_monthly_quota: 40,
            credits_last_refilled: Some(now),
            tenant_id: None,
            accessibility_needs: None,
            cost_center: None,
            department: None,
        };

        // Persist
        match state_guard.db.save_user(&user).await {
            Ok(()) => imported += 1,
            Err(e) => {
                tracing::error!("Failed to save imported user row {}: {}", row_num, e);
                errors.push(ImportError {
                    row: row_num,
                    field: String::new(),
                    message: "Failed to save user to database".to_string(),
                });
            }
        }
    }

    drop(state_guard);

    (
        StatusCode::OK,
        Json(ApiResponse::success(ImportResult {
            imported,
            skipped,
            errors,
        })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// iCal absence import
// ─────────────────────────────────────────────────────────────────────────────

/// Result of an iCal absence import.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IcalImportResult {
    /// Number of absences successfully imported.
    pub imported: usize,
    /// Number of events skipped (missing required fields).
    pub skipped: usize,
}

/// Map an iCal SUMMARY or CATEGORIES string to an [`AbsenceType`].
fn summary_to_absence_type(summary: &str) -> AbsenceType {
    let lower = summary.to_lowercase();
    if lower.contains("homeoffice") || lower.contains("home office") || lower.contains("remote") {
        AbsenceType::Homeoffice
    } else if lower.contains("vacation")
        || lower.contains("urlaub")
        || lower.contains("holiday")
        || lower.contains("ferien")
    {
        AbsenceType::Vacation
    } else if lower.contains("sick")
        || lower.contains("krank")
        || lower.contains("illness")
        || lower.contains("krankenstand")
    {
        AbsenceType::Sick
    } else if lower.contains("training")
        || lower.contains("schulung")
        || lower.contains("workshop")
        || lower.contains("conference")
        || lower.contains("konferenz")
    {
        AbsenceType::Training
    } else {
        AbsenceType::Other
    }
}

/// Parse a date string from iCal (YYYYMMDD or YYYY-MM-DD) into "YYYY-MM-DD".
fn parse_ical_date(value: &str) -> Option<String> {
    let v = value.trim();
    if v.len() == 8 && v.chars().all(|c| c.is_ascii_digit()) {
        // YYYYMMDD
        Some(format!("{}-{}-{}", &v[0..4], &v[4..6], &v[6..8]))
    } else if v.len() == 10
        && v.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
    {
        Some(v.to_string())
    } else {
        None
    }
}

/// Parse VEVENT blocks from an iCal string.
/// Returns a list of `(dtstart, dtend, summary, description)` tuples.
fn parse_vevents(ical: &str) -> Vec<(String, String, String, Option<String>)> {
    let mut events = Vec::new();
    let mut in_event = false;
    let mut dtstart = String::new();
    let mut dtend = String::new();
    let mut summary = String::new();
    let mut description: Option<String> = None;

    for line in ical.lines() {
        let line = line.trim_end_matches('\r');
        match line {
            "BEGIN:VEVENT" => {
                in_event = true;
                dtstart.clear();
                dtend.clear();
                summary.clear();
                description = None;
            }
            "END:VEVENT" if in_event => {
                in_event = false;
                if !dtstart.is_empty() && !dtend.is_empty() {
                    events.push((
                        dtstart.clone(),
                        dtend.clone(),
                        summary.clone(),
                        description.clone(),
                    ));
                }
            }
            _ if in_event => {
                if let Some(rest) = line.strip_prefix("DTSTART") {
                    // strip property params: DTSTART;VALUE=DATE:20240101 or DTSTART:20240101
                    let val = rest.split_once(':').map_or("", |x| x.1).trim();
                    if let Some(d) = parse_ical_date(val) {
                        dtstart = d;
                    }
                } else if let Some(rest) = line.strip_prefix("DTEND") {
                    let val = rest.split_once(':').map_or("", |x| x.1).trim();
                    if let Some(d) = parse_ical_date(val) {
                        dtend = d;
                    }
                } else if let Some(val) = line.strip_prefix("SUMMARY:") {
                    summary = val.trim().to_string();
                } else if let Some(val) = line.strip_prefix("DESCRIPTION:") {
                    description = Some(val.trim().to_string());
                }
            }
            _ => {}
        }
    }

    events
}

/// `POST /api/v1/absences/import/ical` — import absences from an iCal body
#[utoipa::path(
    post,
    path = "/api/v1/absences/import/ical",
    tag = "Absences",
    summary = "Import absences from iCal",
    description = "Parse VEVENT blocks from an iCal (RFC 5545) body and create absences for the \
        authenticated user. DTSTART and DTEND are required; SUMMARY is used to infer the absence type.",
    request_body(
        content = String,
        content_type = "text/calendar",
        description = "iCal data (VCALENDAR with VEVENT blocks)"
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Import completed", body = IcalImportResult),
        (status = 400, description = "Empty or invalid iCal body"),
    )
)]
pub async fn import_absences_ical(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    body: String,
) -> (StatusCode, Json<ApiResponse<IcalImportResult>>) {
    if body.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("EMPTY_ICAL", "iCal body is empty")),
        );
    }

    let events = parse_vevents(&body);
    if events.is_empty() {
        return (
            StatusCode::OK,
            Json(ApiResponse::success(IcalImportResult {
                imported: 0,
                skipped: 0,
            })),
        );
    }

    let state_guard = state.read().await;
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let now = Utc::now();

    for (dtstart, dtend, summary, desc) in events {
        if dtstart.is_empty() || dtend.is_empty() {
            skipped += 1;
            continue;
        }

        let absence_type = summary_to_absence_type(&summary);
        let note = desc.or_else(|| {
            if summary.is_empty() {
                None
            } else {
                Some(summary.clone())
            }
        });

        let absence = Absence {
            id: Uuid::new_v4(),
            user_id: auth_user.user_id,
            absence_type,
            start_date: dtstart,
            end_date: dtend,
            note,
            source: "ical".to_string(),
            created_at: now,
        };

        match state_guard.db.save_absence(&absence).await {
            Ok(()) => imported += 1,
            Err(e) => {
                tracing::error!("Failed to save iCal-imported absence: {e}");
                skipped += 1;
            }
        }
    }

    drop(state_guard);

    (
        StatusCode::OK,
        Json(ApiResponse::success(IcalImportResult { imported, skipped })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_valid_row() {
        let row = parse_csv_line("alice,alice@example.com,Alice Smith,user,Secret1!").unwrap();
        assert_eq!(row.username, "alice");
        assert_eq!(row.email, "alice@example.com");
        assert_eq!(row.name, "Alice Smith");
        assert_eq!(row.role, "user");
        assert_eq!(row.password, "Secret1!");
    }

    #[test]
    fn test_parse_csv_valid_row_minimal() {
        // Only username + email
        let row = parse_csv_line("bob,bob@example.com").unwrap();
        assert_eq!(row.username, "bob");
        assert_eq!(row.email, "bob@example.com");
        assert_eq!(row.name, "");
        assert_eq!(row.role, "");
        assert_eq!(row.password, "");
    }

    #[test]
    fn test_parse_csv_missing_email() {
        // Only one field → no email
        let err = parse_csv_line("alice_only").unwrap_err();
        assert_eq!(err.0, "");
        assert!(err.1.contains("at least"));
    }

    #[test]
    fn test_parse_csv_empty_email_field() {
        let err = parse_csv_line("alice,").unwrap_err();
        assert_eq!(err.0, "email");
        assert!(err.1.contains("required"));
    }

    #[test]
    fn test_parse_csv_empty_username_field() {
        let err = parse_csv_line(",alice@example.com").unwrap_err();
        assert_eq!(err.0, "username");
        assert!(err.1.contains("required"));
    }

    #[test]
    fn test_parse_csv_whitespace_trimming() {
        let row = parse_csv_line("  alice  , alice@example.com , Alice , admin , ").unwrap();
        assert_eq!(row.username, "alice");
        assert_eq!(row.email, "alice@example.com");
        assert_eq!(row.name, "Alice");
        assert_eq!(row.role, "admin");
        assert_eq!(row.password, "");
    }

    #[test]
    fn test_parse_csv_empty_body() {
        let lines: Vec<&str> = ""
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        assert!(lines.is_empty());
    }

    #[test]
    fn test_parse_csv_header_only() {
        let body = "username,email,name,role,password\n";
        let mut lines: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        // Simulate header skip
        if !lines.is_empty() {
            let first = lines[0].to_lowercase();
            if first.starts_with("username") {
                lines.remove(0);
            }
        }
        assert!(lines.is_empty());
    }

    #[test]
    fn test_parse_role_variants() {
        assert_eq!(parse_role("user"), UserRole::User);
        assert_eq!(parse_role(""), UserRole::User);
        assert_eq!(parse_role("premium"), UserRole::Premium);
        assert_eq!(parse_role("ADMIN"), UserRole::Admin);
        assert_eq!(parse_role("superadmin"), UserRole::SuperAdmin);
        assert_eq!(parse_role("super_admin"), UserRole::SuperAdmin);
        assert_eq!(parse_role("unknown"), UserRole::User);
    }

    #[test]
    fn test_max_rows_constant() {
        assert_eq!(MAX_IMPORT_ROWS, 500);
    }
}
