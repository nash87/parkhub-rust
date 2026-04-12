//! CSV export endpoints for admin users.
//!
//! - `GET /api/v1/admin/export/bookings` — export bookings as CSV
//! - `GET /api/v1/admin/export/users` — export users as CSV
//! - `GET /api/v1/admin/export/revenue` — export revenue summary as CSV

use axum::{
    Extension,
    extract::{Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use std::fmt::Write;

use super::{AuthUser, SharedState, check_admin};

// ─────────────────────────────────────────────────────────────────────────────
// Query params
// ─────────────────────────────────────────────────────────────────────────────

/// Optional date-range filter for CSV exports.
#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
pub struct ExportDateRange {
    /// Start date (inclusive), e.g. `2026-01-01`
    pub from: Option<NaiveDate>,
    /// End date (inclusive), e.g. `2026-03-21`
    pub to: Option<NaiveDate>,
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV injection protection
// ─────────────────────────────────────────────────────────────────────────────

/// Escape a cell value for CSV.
///
/// 1. Prefix with `'` if the value starts with `=`, `+`, `-`, or `@` (CSV injection).
/// 2. Double any internal double-quotes and wrap in quotes if needed.
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

// ─────────────────────────────────────────────────────────────────────────────
// Response helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build CSV response with Content-Disposition attachment header.
fn csv_response(
    filename: &str,
    body: String,
) -> (StatusCode, [(header::HeaderName, &'static str); 2], String) {
    let disposition = match filename {
        "bookings.csv" => "attachment; filename=\"bookings.csv\"",
        "users.csv" => "attachment; filename=\"users.csv\"",
        "revenue.csv" => "attachment; filename=\"revenue.csv\"",
        _ => "attachment",
    };
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        body,
    )
}

fn error_response(
    status: StatusCode,
    msg: &str,
) -> (StatusCode, [(header::HeaderName, &'static str); 2], String) {
    (
        status,
        [
            (header::CONTENT_TYPE, "text/plain"),
            (header::CONTENT_DISPOSITION, "inline"),
        ],
        msg.to_string(),
    )
}

/// Check whether a UTC datetime falls within the optional date range.
fn in_date_range(dt: &DateTime<Utc>, range: &ExportDateRange) -> bool {
    if let Some(from) = range.from {
        if dt.date_naive() < from {
            return false;
        }
    }
    if let Some(to) = range.to {
        if dt.date_naive() > to {
            return false;
        }
    }
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Bookings CSV
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/export/bookings` — export all bookings as CSV (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/export/bookings",
    tag = "Admin",
    summary = "Export bookings as CSV",
    description = "Download all bookings as a CSV file. Supports optional date filtering via from and to query params (YYYY-MM-DD). Admin only.",
    params(ExportDateRange),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_export_bookings_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(range): Query<ExportDateRange>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return error_response(status, msg);
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings for CSV export: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to export bookings",
            );
        }
    };

    let mut csv = String::from(
        "id,user_id,lot_name,slot_number,start_time,end_time,status,vehicle_plate,total,currency,payment_status\n",
    );

    for b in &bookings {
        if !in_date_range(&b.start_time, &range) {
            continue;
        }

        // Resolve lot name (best-effort)
        let lot_name = match state_guard.db.get_parking_lot(&b.lot_id.to_string()).await {
            Ok(Some(l)) => l.name,
            _ => b.lot_id.to_string(),
        };

        csv.push_str(&csv_escape(&b.id.to_string()));
        csv.push(',');
        csv.push_str(&csv_escape(&b.user_id.to_string()));
        csv.push(',');
        csv.push_str(&csv_escape(&lot_name));
        csv.push(',');
        csv.push_str(&b.slot_number.to_string());
        csv.push(',');
        csv.push_str(&b.start_time.to_rfc3339());
        csv.push(',');
        csv.push_str(&b.end_time.to_rfc3339());
        csv.push(',');
        csv.push_str(&csv_escape(&format!("{:?}", b.status).to_lowercase()));
        csv.push(',');
        csv.push_str(&csv_escape(&b.vehicle.license_plate));
        csv.push(',');
        let _ = write!(csv, "{:.2}", b.pricing.total);
        csv.push(',');
        csv.push_str(&csv_escape(&b.pricing.currency));
        csv.push(',');
        csv.push_str(&csv_escape(
            &format!("{:?}", b.pricing.payment_status).to_lowercase(),
        ));
        csv.push('\n');
    }
    drop(state_guard);

    csv_response("bookings.csv", csv)
}

// ─────────────────────────────────────────────────────────────────────────────
// Users CSV
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/export/users` — export all users as CSV (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/export/users",
    tag = "Admin",
    summary = "Export users as CSV",
    description = "Download all users as a CSV file. Supports optional date filtering (by created_at) via from and to query params (YYYY-MM-DD). Admin only.",
    params(ExportDateRange),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_export_users_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(range): Query<ExportDateRange>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return error_response(status, msg);
    }

    let users = match state_guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users for CSV export: {}", e);
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to export users");
        }
    };

    let mut csv =
        String::from("id,username,email,name,role,is_active,credits_balance,created_at\n");
    for u in &users {
        if !in_date_range(&u.created_at, &range) {
            continue;
        }

        csv.push_str(&csv_escape(&u.id.to_string()));
        csv.push(',');
        csv.push_str(&csv_escape(&u.username));
        csv.push(',');
        csv.push_str(&csv_escape(&u.email));
        csv.push(',');
        csv.push_str(&csv_escape(&u.name));
        csv.push(',');
        csv.push_str(&csv_escape(&format!("{:?}", u.role).to_lowercase()));
        csv.push(',');
        csv.push_str(if u.is_active { "true" } else { "false" });
        csv.push(',');
        csv.push_str(&u.credits_balance.to_string());
        csv.push(',');
        csv.push_str(&u.created_at.to_rfc3339());
        csv.push('\n');
    }
    drop(state_guard);

    csv_response("users.csv", csv)
}

// ─────────────────────────────────────────────────────────────────────────────
// Revenue CSV
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/export/revenue` — export revenue summary as CSV (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/export/revenue",
    tag = "Admin",
    summary = "Export revenue summary as CSV",
    description = "Download a daily revenue summary as a CSV file. Groups completed/active bookings by date. Supports optional date filtering via from and to query params (YYYY-MM-DD). Admin only.",
    params(ExportDateRange),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_export_revenue_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(range): Query<ExportDateRange>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return error_response(status, msg);
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings for revenue CSV export: {}", e);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to export revenue",
            );
        }
    };
    drop(state_guard);

    // Aggregate revenue by date (completed/active/confirmed bookings)
    let mut daily: std::collections::BTreeMap<String, (usize, f64, f64)> =
        std::collections::BTreeMap::new();

    for b in &bookings {
        if !in_date_range(&b.start_time, &range) {
            continue;
        }

        // Only count bookings that represent revenue
        let counts = matches!(
            b.status,
            parkhub_common::BookingStatus::Completed
                | parkhub_common::BookingStatus::Active
                | parkhub_common::BookingStatus::Confirmed
        );
        if !counts {
            continue;
        }

        let date = b.start_time.format("%Y-%m-%d").to_string();
        let entry = daily.entry(date).or_insert((0, 0.0, 0.0));
        entry.0 += 1; // booking count
        entry.1 += b.pricing.total; // gross revenue
        entry.2 += b.pricing.tax; // tax
    }

    let mut csv = String::from("date,booking_count,gross_revenue,tax,net_revenue,currency\n");

    for (date, (count, gross, tax)) in &daily {
        let net = gross - tax;
        csv.push_str(&csv_escape(date));
        csv.push(',');
        csv.push_str(&count.to_string());
        csv.push(',');
        let _ = write!(csv, "{gross:.2}");
        csv.push(',');
        let _ = write!(csv, "{tax:.2}");
        csv.push(',');
        let _ = write!(csv, "{net:.2}");
        csv.push(',');
        csv.push_str("EUR");
        csv.push('\n');
    }

    csv_response("revenue.csv", csv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape_plain_text() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("John Doe"), "John Doe");
    }

    #[test]
    fn test_csv_escape_formula_injection_equals() {
        assert_eq!(csv_escape("=SUM(A1)"), "'=SUM(A1)");
    }

    #[test]
    fn test_csv_escape_formula_injection_plus() {
        assert_eq!(csv_escape("+cmd"), "'+cmd");
    }

    #[test]
    fn test_csv_escape_formula_injection_minus() {
        assert_eq!(csv_escape("-cmd"), "'-cmd");
    }

    #[test]
    fn test_csv_escape_formula_injection_at() {
        assert_eq!(csv_escape("@SUM"), "'@SUM");
    }

    #[test]
    fn test_csv_escape_commas() {
        assert_eq!(csv_escape("hello,world"), "\"hello,world\"");
    }

    #[test]
    fn test_csv_escape_double_quotes() {
        assert_eq!(csv_escape(r#"say "hi""#), r#""say ""hi""""#);
    }

    #[test]
    fn test_csv_escape_newlines() {
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_csv_escape_formula_with_comma() {
        assert_eq!(csv_escape("=1,2"), "\"'=1,2\"");
    }

    #[test]
    fn test_csv_escape_empty_string() {
        assert_eq!(csv_escape(""), "");
    }

    #[test]
    fn test_csv_escape_uuid_not_prefixed() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(csv_escape(uuid_str), uuid_str);
    }

    #[test]
    fn test_csv_escape_minus_number() {
        assert_eq!(csv_escape("-10"), "'-10");
    }

    #[test]
    fn test_in_date_range_no_filter() {
        let dt = Utc::now();
        let range = ExportDateRange::default();
        assert!(in_date_range(&dt, &range));
    }

    #[test]
    fn test_in_date_range_from_only() {
        let dt = DateTime::parse_from_rfc3339("2026-02-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let range = ExportDateRange {
            from: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            to: None,
        };
        assert!(!in_date_range(&dt, &range));

        let range_ok = ExportDateRange {
            from: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            to: None,
        };
        assert!(in_date_range(&dt, &range_ok));
    }

    #[test]
    fn test_in_date_range_to_only() {
        let dt = DateTime::parse_from_rfc3339("2026-06-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let range = ExportDateRange {
            from: None,
            to: Some(NaiveDate::from_ymd_opt(2026, 3, 21).unwrap()),
        };
        assert!(!in_date_range(&dt, &range));
    }

    #[test]
    fn test_in_date_range_both_bounds() {
        let dt = DateTime::parse_from_rfc3339("2026-02-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let range = ExportDateRange {
            from: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2026, 3, 21).unwrap()),
        };
        assert!(in_date_range(&dt, &range));

        let range_outside = ExportDateRange {
            from: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2026, 3, 21).unwrap()),
        };
        assert!(!in_date_range(&dt, &range_outside));
    }

    #[test]
    fn test_csv_response_headers() {
        let (status, headers, body) = csv_response("bookings.csv", "a,b\n1,2\n".to_string());
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[0].1, "text/csv; charset=utf-8");
        assert!(headers[1].1.contains("bookings.csv"));
        assert_eq!(body, "a,b\n1,2\n");
    }
}
