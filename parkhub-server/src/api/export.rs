//! CSV export endpoints for admin users.
//!
//! - `GET /api/v1/admin/users/export-csv`
//! - `GET /api/v1/admin/bookings/export-csv`

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Extension,
};

use super::{check_admin, AuthUser, SharedState};

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
        format!("'{}", value)
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
// Users CSV
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/users/export-csv` — export all users as CSV (admin only)
pub(super) async fn admin_export_users_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, [(header::CONTENT_TYPE, "text/plain")], msg.to_string());
    }

    let users = match state_guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users for CSV export: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                "Failed to export users".to_string(),
            );
        }
    };

    let mut csv = String::from("id,username,email,name,role,is_active,created_at\n");
    for u in &users {
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
        csv.push_str(&u.created_at.to_rfc3339());
        csv.push('\n');
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
        csv,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Bookings CSV
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/bookings/export-csv` — export all bookings as CSV (admin only)
pub(super) async fn admin_export_bookings_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, [(header::CONTENT_TYPE, "text/plain")], msg.to_string());
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings for CSV export: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                "Failed to export bookings".to_string(),
            );
        }
    };

    let mut csv =
        String::from("id,user_id,lot_name,slot_number,start_time,end_time,status,vehicle_plate\n");

    for b in &bookings {
        // Resolve lot name (best-effort)
        let lot_name = match state_guard
            .db
            .get_parking_lot(&b.lot_id.to_string())
            .await
        {
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
        csv.push('\n');
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
        csv,
    )
}
