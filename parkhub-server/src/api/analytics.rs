//! Admin analytics overview endpoint.
//!
//! `GET /api/v1/admin/analytics/overview` returns a comprehensive dashboard
//! including daily bookings, revenue, peak hours, top lots, user growth, and
//! average booking duration for the requested date range.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{Duration, TimeDelta, Timelike, Utc};
use parkhub_common::ApiResponse;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;

use super::{check_admin, AuthUser};

type SharedState = Arc<RwLock<AppState>>;

// ═══════════════════════════════════════════════════════════════════════════════
// Request / Response types
// ═══════════════════════════════════════════════════════════════════════════════

/// Query parameters for the analytics overview.
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// Number of days to look back (default: 30).
    pub days: Option<i64>,
}

/// A single day data point for bookings or revenue.
#[derive(Debug, Clone, Serialize)]
pub struct DailyDataPoint {
    pub date: String,
    pub value: f64,
}

/// Peak hours histogram bin (0–23).
#[derive(Debug, Clone, Serialize)]
pub struct HourBin {
    pub hour: u8,
    pub count: u64,
}

/// Top lot by utilization.
#[derive(Debug, Clone, Serialize)]
pub struct TopLot {
    pub lot_id: String,
    pub lot_name: String,
    pub total_slots: u64,
    pub bookings: u64,
    pub utilization_percent: f64,
}

/// User growth data point (monthly).
#[derive(Debug, Clone, Serialize)]
pub struct MonthlyGrowth {
    pub month: String,
    pub count: u64,
}

/// The full analytics overview response.
#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub daily_bookings: Vec<DailyDataPoint>,
    pub daily_revenue: Vec<DailyDataPoint>,
    pub peak_hours: Vec<HourBin>,
    pub top_lots: Vec<TopLot>,
    pub user_growth: Vec<MonthlyGrowth>,
    pub avg_booking_duration_minutes: f64,
    pub total_bookings: u64,
    pub total_revenue: f64,
    pub active_users: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Handler
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/analytics/overview`
///
/// Returns a comprehensive analytics overview including daily bookings,
/// revenue per day, peak hours histogram, top 10 lots by utilization,
/// user growth over the last 12 months, and average booking duration.
/// `GET /api/v1/admin/analytics/overview`
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn analytics_overview(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<AnalyticsQuery>,
) -> (StatusCode, Json<ApiResponse<AnalyticsOverview>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let days = query.days.unwrap_or(30);
    let cutoff = Utc::now() - TimeDelta::days(days);
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let users = state_guard.db.list_users().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();

    // ── Daily bookings ──────────────────────────────────────────────────────
    let mut daily_bookings_map: BTreeMap<String, u64> = BTreeMap::new();
    let mut daily_revenue_map: BTreeMap<String, f64> = BTreeMap::new();
    let mut peak_hours: [u64; 24] = [0; 24];
    let mut lot_booking_count: HashMap<uuid::Uuid, u64> = HashMap::new();
    let mut total_duration_minutes: f64 = 0.0;
    let mut duration_count: u64 = 0;
    let mut total_revenue: f64 = 0.0;
    let mut total_bookings_in_range: u64 = 0;

    // Pre-fill all days in range with 0
    for i in 0..days {
        let day = (Utc::now() - Duration::days(days - 1 - i))
            .format("%Y-%m-%d")
            .to_string();
        daily_bookings_map.entry(day.clone()).or_insert(0);
        daily_revenue_map.entry(day).or_insert(0.0);
    }

    for b in &bookings {
        if b.created_at >= cutoff {
            let date = b.created_at.format("%Y-%m-%d").to_string();
            *daily_bookings_map.entry(date.clone()).or_insert(0) += 1;
            total_bookings_in_range += 1;

            // Revenue from booking pricing
            let price = b.pricing.total;
            *daily_revenue_map.entry(date).or_insert(0.0) += price;
            total_revenue += price;

            // Peak hours
            let hour = b.start_time.hour() as usize;
            if hour < 24 {
                peak_hours[hour] += 1;
            }

            // Lot booking count
            *lot_booking_count.entry(b.lot_id).or_insert(0) += 1;

            // Duration
            let dur = (b.end_time - b.start_time).num_minutes() as f64;
            if dur > 0.0 {
                total_duration_minutes += dur;
                duration_count += 1;
            }
        }
    }

    let daily_bookings: Vec<DailyDataPoint> = daily_bookings_map
        .into_iter()
        .map(|(date, value)| DailyDataPoint {
            date,
            value: value as f64,
        })
        .collect();

    let daily_revenue: Vec<DailyDataPoint> = daily_revenue_map
        .into_iter()
        .map(|(date, value)| DailyDataPoint {
            date,
            value: (value * 100.0).round() / 100.0,
        })
        .collect();

    let peak_hours_vec: Vec<HourBin> = peak_hours
        .iter()
        .enumerate()
        .map(|(hour, &count)| HourBin {
            hour: hour as u8,
            count,
        })
        .collect();

    // ── Top 10 lots by utilization ──────────────────────────────────────────
    let mut top_lots: Vec<TopLot> = lots
        .iter()
        .map(|lot| {
            let lot_uuid = lot.id;
            let bookings_count = lot_booking_count.get(&lot_uuid).copied().unwrap_or(0);
            let total_slots = lot.total_slots as u64;
            #[allow(clippy::cast_precision_loss)]
            let utilization = if total_slots > 0 && days > 0 {
                (bookings_count as f64 / (total_slots as f64 * days as f64)) * 100.0
            } else {
                0.0
            };
            TopLot {
                lot_id: lot.id.to_string(),
                lot_name: lot.name.clone(),
                total_slots,
                bookings: bookings_count,
                utilization_percent: (utilization * 100.0).round() / 100.0,
            }
        })
        .collect();
    top_lots.sort_by(|a, b| {
        b.utilization_percent
            .partial_cmp(&a.utilization_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    top_lots.truncate(10);

    // ── User growth (last 12 months) ────────────────────────────────────────
    let twelve_months_ago = Utc::now() - TimeDelta::days(365);
    let mut monthly_growth: BTreeMap<String, u64> = BTreeMap::new();
    // Pre-fill 12 months
    for i in 0..12 {
        let month_date = Utc::now() - Duration::days(30 * (11 - i));
        let key = month_date.format("%Y-%m").to_string();
        monthly_growth.entry(key).or_insert(0);
    }
    for u in &users {
        if u.created_at >= twelve_months_ago {
            let key = u.created_at.format("%Y-%m").to_string();
            *monthly_growth.entry(key).or_insert(0) += 1;
        }
    }
    let user_growth: Vec<MonthlyGrowth> = monthly_growth
        .into_iter()
        .map(|(month, count)| MonthlyGrowth { month, count })
        .collect();

    // ── Average booking duration ────────────────────────────────────────────
    let avg_duration = if duration_count > 0 {
        (total_duration_minutes / duration_count as f64 * 100.0).round() / 100.0
    } else {
        0.0
    };

    // ── Active users (users with at least 1 booking in range) ───────────────
    let active_user_ids: std::collections::HashSet<uuid::Uuid> = bookings
        .iter()
        .filter(|b| b.created_at >= cutoff)
        .map(|b| b.user_id)
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(AnalyticsOverview {
            daily_bookings,
            daily_revenue,
            peak_hours: peak_hours_vec,
            top_lots,
            user_growth,
            avg_booking_duration_minutes: avg_duration,
            total_bookings: total_bookings_in_range,
            total_revenue: (total_revenue * 100.0).round() / 100.0,
            active_users: active_user_ids.len() as u64,
        })),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_data_point_serializes() {
        let dp = DailyDataPoint {
            date: "2026-03-22".to_string(),
            value: 42.5,
        };
        let json = serde_json::to_string(&dp).unwrap();
        assert!(json.contains("2026-03-22"));
        assert!(json.contains("42.5"));
    }

    #[test]
    fn hour_bin_covers_full_day() {
        let bins: Vec<HourBin> = (0..24)
            .map(|h| HourBin {
                hour: h,
                count: h as u64 * 10,
            })
            .collect();
        assert_eq!(bins.len(), 24);
        assert_eq!(bins[0].hour, 0);
        assert_eq!(bins[23].hour, 23);
        assert_eq!(bins[12].count, 120);
    }

    #[test]
    fn top_lot_serializes() {
        let lot = TopLot {
            lot_id: "lot-1".to_string(),
            lot_name: "Main Garage".to_string(),
            total_slots: 50,
            bookings: 120,
            utilization_percent: 85.5,
        };
        let json = serde_json::to_string(&lot).unwrap();
        assert!(json.contains("Main Garage"));
        assert!(json.contains("85.5"));
    }

    #[test]
    fn monthly_growth_serializes() {
        let g = MonthlyGrowth {
            month: "2026-03".to_string(),
            count: 15,
        };
        let json = serde_json::to_string(&g).unwrap();
        assert!(json.contains("2026-03"));
        assert!(json.contains("15"));
    }

    #[test]
    fn analytics_overview_default_fields() {
        let overview = AnalyticsOverview {
            daily_bookings: vec![],
            daily_revenue: vec![],
            peak_hours: vec![],
            top_lots: vec![],
            user_growth: vec![],
            avg_booking_duration_minutes: 0.0,
            total_bookings: 0,
            total_revenue: 0.0,
            active_users: 0,
        };
        let json = serde_json::to_string(&overview).unwrap();
        assert!(json.contains("daily_bookings"));
        assert!(json.contains("peak_hours"));
        assert!(json.contains("avg_booking_duration_minutes"));
    }

    #[test]
    fn analytics_overview_with_data() {
        let overview = AnalyticsOverview {
            daily_bookings: vec![
                DailyDataPoint { date: "2026-03-20".into(), value: 5.0 },
                DailyDataPoint { date: "2026-03-21".into(), value: 8.0 },
            ],
            daily_revenue: vec![
                DailyDataPoint { date: "2026-03-20".into(), value: 25.0 },
                DailyDataPoint { date: "2026-03-21".into(), value: 40.0 },
            ],
            peak_hours: (0..24).map(|h| HourBin { hour: h, count: h as u64 }).collect(),
            top_lots: vec![TopLot {
                lot_id: "lot-1".into(),
                lot_name: "HQ Garage".into(),
                total_slots: 100,
                bookings: 250,
                utilization_percent: 83.33,
            }],
            user_growth: vec![MonthlyGrowth { month: "2026-03".into(), count: 12 }],
            avg_booking_duration_minutes: 180.0,
            total_bookings: 13,
            total_revenue: 65.0,
            active_users: 7,
        };
        let json = serde_json::to_string(&overview).unwrap();
        assert!(json.contains("HQ Garage"));
        assert!(json.contains("180"));
        assert_eq!(overview.peak_hours.len(), 24);
        assert_eq!(overview.top_lots.len(), 1);
    }
}
