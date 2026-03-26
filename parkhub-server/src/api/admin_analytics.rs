//! Admin analytics endpoints: occupancy trends, revenue summary, and popular lots.
//!
//! - `GET /api/v1/admin/analytics/occupancy`     — hourly occupancy rates for last 7 days
//! - `GET /api/v1/admin/analytics/revenue`        — daily revenue summary for last 30 days
//! - `GET /api/v1/admin/analytics/popular-lots`   — top 10 lots by booking count

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::{Duration, Timelike, Utc};
use parkhub_common::{ApiResponse, BookingStatus};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;

use super::{check_admin, AuthUser};

type SharedState = Arc<RwLock<AppState>>;

// ═══════════════════════════════════════════════════════════════════════════════
// Response types
// ═══════════════════════════════════════════════════════════════════════════════

/// A single hourly occupancy data point.
#[derive(Debug, Clone, Serialize)]
pub struct OccupancyPoint {
    /// ISO-8601 hour label, e.g. `"2026-03-25 14:00"`.
    pub hour: String,
    /// Number of bookings whose `start_time` falls within this hour.
    pub active_bookings: u64,
    /// Total parking slots across all lots (snapshot at query time).
    pub total_slots: u64,
    /// `active_bookings / total_slots * 100`, clamped to `[0, 100]`.
    pub occupancy_rate: f64,
}

/// A single day revenue data point.
#[derive(Debug, Clone, Serialize)]
pub struct RevenueSummaryPoint {
    /// Date label, e.g. `"2026-03-25"`.
    pub date: String,
    /// Sum of `booking.pricing.total` for non-cancelled bookings on this day.
    pub total_revenue: f64,
    /// Number of non-cancelled bookings created on this day.
    pub booking_count: u64,
    /// Average revenue per booking (`total_revenue / booking_count`), or `0` when no bookings.
    pub avg_revenue: f64,
}

/// A single popular lot entry.
#[derive(Debug, Clone, Serialize)]
pub struct PopularLotEntry {
    /// Parking lot UUID.
    pub lot_id: String,
    /// Human-readable lot name.
    pub lot_name: String,
    /// Total bookings for this lot (all time).
    pub booking_count: u64,
    /// Cumulative revenue for this lot (all time).
    pub total_revenue: f64,
    /// Rank (1 = most popular).
    pub rank: u32,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Handlers
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/analytics/occupancy`
///
/// Returns hourly occupancy rates for the last 7 days (168 hourly bins).
/// Each bin reports the number of bookings whose `start_time` falls within that
/// hour and an occupancy rate relative to total available parking slots.
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_occupancy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<OccupancyPoint>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let now = Utc::now();
    let cutoff = now - Duration::days(7);

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();

    let total_slots: u64 = lots.iter().map(|l| l.total_slots as u64).sum();

    // Build a map: "YYYY-MM-DD HH:00" -> count
    let mut hourly: HashMap<String, u64> = HashMap::new();

    // Pre-fill all 168 hourly bins with 0
    for h in 0..(7 * 24_i64) {
        let slot = cutoff + Duration::hours(h);
        let key = slot.format("%Y-%m-%d %H:00").to_string();
        hourly.entry(key).or_insert(0);
    }

    for b in &bookings {
        if matches!(b.status, BookingStatus::Cancelled) {
            continue;
        }
        if b.start_time < cutoff || b.start_time >= now {
            continue;
        }
        let key = b
            .start_time
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0))
            .unwrap_or(b.start_time)
            .format("%Y-%m-%d %H:00")
            .to_string();
        *hourly.entry(key).or_insert(0) += 1;
    }

    let mut points: Vec<OccupancyPoint> = hourly
        .into_iter()
        .map(|(hour, active_bookings)| {
            let occupancy_rate = if total_slots > 0 {
                let raw = (active_bookings as f64 / total_slots as f64 * 100.0)
                    .clamp(0.0, 100.0);
                (raw * 100.0).round() / 100.0
            } else {
                0.0
            };
            OccupancyPoint {
                hour,
                active_bookings,
                total_slots,
                occupancy_rate,
            }
        })
        .collect();

    points.sort_by(|a, b| a.hour.cmp(&b.hour));

    (StatusCode::OK, Json(ApiResponse::success(points)))
}

/// `GET /api/v1/admin/analytics/revenue`
///
/// Returns a daily revenue summary for the last 30 days.
/// Only non-cancelled bookings are included.
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_revenue_summary(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<RevenueSummaryPoint>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let now = Utc::now();
    let cutoff = now - Duration::days(30);

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    // revenue (f64) and count (u64) per day
    let mut daily: HashMap<String, (f64, u64)> = HashMap::new();

    // Pre-fill all 30 day bins with 0
    for d in 0..30_i64 {
        let day = (cutoff + Duration::days(d)).format("%Y-%m-%d").to_string();
        daily.entry(day).or_insert((0.0, 0));
    }

    for b in &bookings {
        if matches!(b.status, BookingStatus::Cancelled) {
            continue;
        }
        if b.created_at < cutoff || b.created_at >= now {
            continue;
        }
        let day = b.created_at.format("%Y-%m-%d").to_string();
        let entry = daily.entry(day).or_insert((0.0, 0));
        entry.0 += b.pricing.total;
        entry.1 += 1;
    }

    let mut points: Vec<RevenueSummaryPoint> = daily
        .into_iter()
        .map(|(date, (total_revenue, booking_count))| {
            let total_revenue = (total_revenue * 100.0).round() / 100.0;
            let avg_revenue = if booking_count > 0 {
                (total_revenue / booking_count as f64 * 100.0).round() / 100.0
            } else {
                0.0
            };
            RevenueSummaryPoint {
                date,
                total_revenue,
                booking_count,
                avg_revenue,
            }
        })
        .collect();

    points.sort_by(|a, b| a.date.cmp(&b.date));

    (StatusCode::OK, Json(ApiResponse::success(points)))
}

/// `GET /api/v1/admin/analytics/popular-lots`
///
/// Returns the top 10 parking lots ranked by all-time booking count.
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_popular_lots(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<PopularLotEntry>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();

    let lot_names: HashMap<String, String> = lots
        .iter()
        .map(|l| (l.id.to_string(), l.name.clone()))
        .collect();

    // lot_id -> (booking_count, total_revenue)
    let mut lot_stats: HashMap<String, (u64, f64)> = HashMap::new();

    for b in &bookings {
        if matches!(b.status, BookingStatus::Cancelled) {
            continue;
        }
        let entry = lot_stats.entry(b.lot_id.to_string()).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += b.pricing.total;
    }

    let mut entries: Vec<PopularLotEntry> = lot_stats
        .into_iter()
        .map(|(lot_id, (booking_count, total_revenue))| {
            let lot_name = lot_names
                .get(&lot_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            let total_revenue = (total_revenue * 100.0).round() / 100.0;
            PopularLotEntry {
                lot_id,
                lot_name,
                booking_count,
                total_revenue,
                rank: 0, // assigned below
            }
        })
        .collect();

    // Sort by booking count descending, then by lot_name for determinism
    entries.sort_by(|a, b| {
        b.booking_count
            .cmp(&a.booking_count)
            .then_with(|| a.lot_name.cmp(&b.lot_name))
    });
    entries.truncate(10);

    // Assign ranks
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.rank = (i + 1) as u32;
    }

    (StatusCode::OK, Json(ApiResponse::success(entries)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── OccupancyPoint ───────────────────────────────────────────────────────

    #[test]
    fn occupancy_point_serializes() {
        let point = OccupancyPoint {
            hour: "2026-03-25 14:00".to_string(),
            active_bookings: 5,
            total_slots: 50,
            occupancy_rate: 10.0,
        };
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("2026-03-25 14:00"));
        assert!(json.contains("\"active_bookings\":5"));
        assert!(json.contains("\"total_slots\":50"));
        assert!(json.contains("10.0"));
    }

    #[test]
    fn occupancy_rate_zero_when_no_slots() {
        // Simulates the calculation when total_slots == 0
        let total_slots: u64 = 0;
        let active_bookings: u64 = 10;
        let rate = if total_slots > 0 {
            (active_bookings as f64 / total_slots as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn occupancy_rate_clamped_at_100() {
        // More bookings than slots should not exceed 100%
        let total_slots: u64 = 5;
        let active_bookings: u64 = 10;
        let rate = (active_bookings as f64 / total_slots as f64 * 100.0).min(100.0);
        assert_eq!(rate, 100.0);
    }

    // ── RevenueSummaryPoint ──────────────────────────────────────────────────

    #[test]
    fn revenue_summary_point_serializes() {
        let point = RevenueSummaryPoint {
            date: "2026-03-25".to_string(),
            total_revenue: 125.50,
            booking_count: 10,
            avg_revenue: 12.55,
        };
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("2026-03-25"));
        assert!(json.contains("125.5"));
        assert!(json.contains("\"booking_count\":10"));
        assert!(json.contains("12.55"));
    }

    #[test]
    fn avg_revenue_zero_when_no_bookings() {
        let booking_count: u64 = 0;
        let total_revenue: f64 = 0.0;
        let avg = if booking_count > 0 {
            total_revenue / booking_count as f64
        } else {
            0.0
        };
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn revenue_summary_multiple_points_sorted() {
        let mut points = vec![
            RevenueSummaryPoint {
                date: "2026-03-25".to_string(),
                total_revenue: 50.0,
                booking_count: 5,
                avg_revenue: 10.0,
            },
            RevenueSummaryPoint {
                date: "2026-03-23".to_string(),
                total_revenue: 80.0,
                booking_count: 8,
                avg_revenue: 10.0,
            },
            RevenueSummaryPoint {
                date: "2026-03-24".to_string(),
                total_revenue: 30.0,
                booking_count: 3,
                avg_revenue: 10.0,
            },
        ];
        points.sort_by(|a, b| a.date.cmp(&b.date));
        assert_eq!(points[0].date, "2026-03-23");
        assert_eq!(points[1].date, "2026-03-24");
        assert_eq!(points[2].date, "2026-03-25");
    }

    // ── PopularLotEntry ──────────────────────────────────────────────────────

    #[test]
    fn popular_lot_entry_serializes() {
        let entry = PopularLotEntry {
            lot_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            lot_name: "Main Garage".to_string(),
            booking_count: 250,
            total_revenue: 1250.75,
            rank: 1,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Main Garage"));
        assert!(json.contains("\"booking_count\":250"));
        assert!(json.contains("1250.75"));
        assert!(json.contains("\"rank\":1"));
    }

    #[test]
    fn popular_lots_ranked_and_truncated() {
        let mut entries: Vec<PopularLotEntry> = (1..=15u64)
            .map(|i| PopularLotEntry {
                lot_id: format!("lot-{}", i),
                lot_name: format!("Lot {}", i),
                booking_count: i * 10,
                total_revenue: i as f64 * 100.0,
                rank: 0,
            })
            .collect();

        // Sort descending by booking_count
        entries.sort_by(|a, b| b.booking_count.cmp(&a.booking_count));
        entries.truncate(10);
        for (i, e) in entries.iter_mut().enumerate() {
            e.rank = (i + 1) as u32;
        }

        assert_eq!(entries.len(), 10);
        assert_eq!(entries[0].rank, 1);
        assert_eq!(entries[0].booking_count, 150); // lot-15 has 150 bookings
        assert_eq!(entries[9].rank, 10);
    }

    #[test]
    fn popular_lots_empty_when_no_bookings() {
        let entries: Vec<PopularLotEntry> = vec![];
        assert!(entries.is_empty());
        let json = serde_json::to_string(&entries).unwrap();
        assert_eq!(json, "[]");
    }
}
