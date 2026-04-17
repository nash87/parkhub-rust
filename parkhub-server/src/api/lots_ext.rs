//! Extended lot handlers: QR code generation, admin dashboard charts.
//!
//! Extracted from mod.rs — Phase 3 API extraction.

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::{TimeDelta, Timelike, Utc};
use image::Luma;
use qrcode::QrCode;
use serde::Serialize;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::AppState;

use super::{AuthUser, check_admin, read_admin_setting};

type SharedState = Arc<RwLock<AppState>>;

// ═══════════════════════════════════════════════════════════════════════════════
// QR CODE GENERATION (LOCAL)
// ═══════════════════════════════════════════════════════════════════════════════

/// QR code response: an inline data URL plus the plain lot booking URL.
///
/// The `qr_url` field is a `data:image/png;base64,…` URL rendered in-process
/// via the `qrcode` + `image` crates so that self-hosted deployments never
/// leak visitor intent to a third-party QR service.
#[derive(Debug, Serialize)]
pub struct LotQrResponse {
    /// `data:image/png;base64,…` URL. Suitable for `<img src={qr_url}>`.
    qr_url: String,
    /// The lot's booking page URL that the QR code encodes.
    lot_url: String,
}

/// `GET /api/v1/lots/{id}/qr` — generate a QR code for a parking lot locally.
///
/// Encodes the lot's booking URL as a 300×300 PNG in memory and returns it as
/// a `data:image/png;base64,…` data URL alongside the plain booking URL. No
/// external services are contacted.
#[utoipa::path(get, path = "/api/v1/lots/{id}/qr", tag = "Lots",
    summary = "Generate QR code for a lot (local)",
    description = "Returns a `data:image/png;base64,…` URL with a QR code encoding the lot's booking page. Generated locally via the `qrcode` + `image` crates — no external service.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Lot UUID")),
    responses((status = 200, description = "QR data URL + lot URL"), (status = 404, description = "Lot not found"), (status = 500, description = "QR generation failed"))
)]
pub async fn lot_qr_code(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<LotQrResponse>>) {
    let state_guard = state.read().await;

    // Verify lot exists
    match state_guard.db.get_parking_lot(&id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    }

    // Derive base_url from admin setting or fall back to localhost
    let base_url = read_admin_setting(&state_guard.db, "base_url").await;
    let base_url = if base_url.is_empty() {
        format!("https://localhost:{}", state_guard.config.port)
    } else {
        base_url
    };

    drop(state_guard);

    let lot_url = format!("{base_url}/book?lot={id}");

    let qr_url = match render_qr_data_url(&lot_url) {
        Ok(url) => url,
        Err(e) => {
            tracing::error!("Lot QR generation failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "QR generation failed")),
            );
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(LotQrResponse { qr_url, lot_url })),
    )
}

/// Render `payload` as a `data:image/png;base64,…` URL at 300×300.
fn render_qr_data_url(payload: &str) -> Result<String, String> {
    let code = QrCode::new(payload.as_bytes()).map_err(|e| format!("qr encode failed: {e}"))?;
    let image = code.render::<Luma<u8>>().min_dimensions(300, 300).build();

    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("png encode failed: {e}"))?;

    Ok(format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(&png_bytes)
    ))
}

// ═══════════════════════════════════════════════════════════════════════════════
// DASHBOARD CHARTS (ADMIN)
// ═══════════════════════════════════════════════════════════════════════════════

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

/// `GET /api/v1/admin/dashboard/charts` — aggregated chart data for the admin
/// dashboard.  Returns bookings-by-day (last 30 days), bookings-by-lot,
/// average occupancy by hour-of-day, and top-10 users by booking count.
#[utoipa::path(get, path = "/api/v1/admin/dashboard/charts", tag = "Admin",
    summary = "Admin dashboard chart data",
    description = "Returns aggregated chart data for the admin dashboard.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Chart data"), (status = 403, description = "Forbidden"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_qr_data_url_produces_png_data_uri() {
        let url = render_qr_data_url("https://parkhub.local/book?lot=abc123")
            .expect("QR generation should succeed for a sane URL");
        assert!(
            url.starts_with("data:image/png;base64,"),
            "expected data URL, got {}",
            &url[..url.len().min(40)]
        );
        // PNG magic number (bytes 0x89 0x50 0x4E 0x47) base64-encodes with
        // the prefix "iVBOR", so any valid PNG data URL starts with it.
        let b64 = url.strip_prefix("data:image/png;base64,").unwrap();
        assert!(b64.starts_with("iVBOR"), "payload is not a PNG");
    }

    #[test]
    fn render_qr_data_url_is_offline() {
        // Guard against a regression to an external QR service: if rendering
        // ever reaches the network this test will fail in offline CI.
        render_qr_data_url("offline-sanity-check")
            .expect("local QR rendering must not require network");
    }

    #[test]
    fn test_bookings_by_day_serialization() {
        let b = BookingsByDay {
            date: "2026-03-24".to_string(),
            count: 5,
        };
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("2026-03-24"));
        assert!(json.contains('5'));
    }

    #[test]
    fn test_bookings_by_lot_serialization() {
        let b = BookingsByLot {
            lot_name: "Main Garage".to_string(),
            count: 42,
        };
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("Main Garage"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_occupancy_by_hour_serialization() {
        let o = OccupancyByHour {
            hour: 9,
            avg_occupancy: 75.5,
        };
        let json = serde_json::to_string(&o).unwrap();
        assert!(json.contains("\"hour\":9"));
        assert!(json.contains("75.5"));
    }

    #[test]
    fn test_top_user_serialization() {
        let u = TopUser {
            username: "alice".to_string(),
            booking_count: 10,
        };
        let json = serde_json::to_string(&u).unwrap();
        assert!(json.contains("alice"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_dashboard_charts_serialization() {
        let charts = DashboardCharts {
            bookings_by_day: vec![BookingsByDay {
                date: "2026-03-01".to_string(),
                count: 3,
            }],
            bookings_by_lot: vec![BookingsByLot {
                lot_name: "Lot B".to_string(),
                count: 7,
            }],
            occupancy_by_hour: vec![OccupancyByHour {
                hour: 14,
                avg_occupancy: 60.0,
            }],
            top_users: vec![TopUser {
                username: "bob".to_string(),
                booking_count: 4,
            }],
        };
        let json = serde_json::to_string(&charts).unwrap();
        assert!(json.contains("bookings_by_day"));
        assert!(json.contains("bookings_by_lot"));
        assert!(json.contains("occupancy_by_hour"));
        assert!(json.contains("top_users"));
        assert!(json.contains("Lot B"));
        assert!(json.contains("bob"));
    }

    #[test]
    fn test_dashboard_charts_empty() {
        let charts = DashboardCharts {
            bookings_by_day: vec![],
            bookings_by_lot: vec![],
            occupancy_by_hour: vec![],
            top_users: vec![],
        };
        let json = serde_json::to_string(&charts).unwrap();
        assert!(json.contains("[]"));
    }
}
