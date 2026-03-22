//! Parking History & Personal Stats handlers.
//!
//! `GET /api/v1/bookings/history` — paginated booking history with filters
//! `GET /api/v1/bookings/stats` — personal parking stats

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use parkhub_common::{ApiResponse, Booking, BookingStatus, CreditTransactionType};

use super::{AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Query params for booking history
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub lot_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

/// Paginated history response
#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub items: Vec<Booking>,
    pub page: i32,
    pub per_page: i32,
    pub total: i32,
    pub total_pages: i32,
}

/// Personal stats response
#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalStats {
    pub total_bookings: i32,
    pub favorite_lot: Option<String>,
    pub avg_duration_minutes: f64,
    pub busiest_day: Option<String>,
    pub credits_spent: i64,
    pub monthly_trend: Vec<MonthlyTrend>,
}

/// Monthly trend data point
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonthlyTrend {
    pub month: String,
    pub bookings: i32,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/bookings/history` — paginated history with filters
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
pub async fn booking_history(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<HistoryQuery>,
) -> (StatusCode, Json<ApiResponse<HistoryResponse>>) {
    let state = state.read().await;

    let bookings = match state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "Failed to list booking history");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to load history")),
            );
        }
    };

    // Filter completed/cancelled only (past bookings)
    let mut filtered: Vec<Booking> = bookings
        .into_iter()
        .filter(|b| {
            matches!(
                b.status,
                BookingStatus::Completed
                    | BookingStatus::Cancelled
                    | BookingStatus::Expired
                    | BookingStatus::NoShow
            )
        })
        .collect();

    // Apply lot filter
    if let Some(lot_id) = query.lot_id {
        filtered.retain(|b| b.lot_id == lot_id);
    }

    // Apply date range filters
    if let Some(from) = query.from {
        filtered.retain(|b| b.start_time >= from);
    }
    if let Some(to) = query.to {
        filtered.retain(|b| b.start_time <= to);
    }

    // Sort by start_time descending (most recent first)
    filtered.sort_by(|a, b| b.start_time.cmp(&a.start_time));

    let total = filtered.len() as i32;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let total_pages = ((total as f64) / (per_page as f64)).ceil() as i32;

    let start = ((page - 1) * per_page) as usize;
    let items: Vec<Booking> = filtered
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(HistoryResponse {
            items,
            page,
            per_page,
            total,
            total_pages,
        })),
    )
}

/// `GET /api/v1/bookings/stats` — personal parking stats
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
pub async fn booking_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<PersonalStats>>) {
    let state = state.read().await;
    let uid = auth_user.user_id.to_string();

    let bookings = state
        .db
        .list_bookings_by_user(&uid)
        .await
        .unwrap_or_default();

    let total_bookings = bookings.len() as i32;

    // Favorite lot (most bookings)
    let favorite_lot = {
        let mut lot_counts: HashMap<Uuid, usize> = HashMap::new();
        for b in &bookings {
            *lot_counts.entry(b.lot_id).or_insert(0) += 1;
        }
        if let Some((&lot_id, _)) = lot_counts.iter().max_by_key(|(_, &c)| c) {
            state
                .db
                .get_parking_lot(&lot_id.to_string())
                .await
                .ok()
                .flatten()
                .map(|l| l.name)
        } else {
            None
        }
    };

    // Average duration in minutes
    let avg_duration_minutes = if bookings.is_empty() {
        0.0
    } else {
        let total_mins: f64 = bookings
            .iter()
            .map(|b| (b.end_time - b.start_time).num_minutes() as f64)
            .sum();
        total_mins / bookings.len() as f64
    };

    // Busiest day of week
    let busiest_day = {
        let mut day_counts: HashMap<u32, usize> = HashMap::new();
        for b in &bookings {
            let weekday = b.start_time.weekday().num_days_from_monday();
            *day_counts.entry(weekday).or_insert(0) += 1;
        }
        day_counts.iter().max_by_key(|(_, &c)| c).map(|(&d, _)| {
            match d {
                0 => "Monday",
                1 => "Tuesday",
                2 => "Wednesday",
                3 => "Thursday",
                4 => "Friday",
                5 => "Saturday",
                6 => "Sunday",
                _ => "Unknown",
            }
            .to_string()
        })
    };

    // Credits spent
    let credits_spent = state
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|tx| tx.transaction_type == CreditTransactionType::Deduction)
        .map(|tx| i64::from(tx.amount.abs()))
        .sum::<i64>();

    // Monthly trend (last 6 months)
    let now = Utc::now();
    let mut monthly_trend: Vec<MonthlyTrend> = Vec::new();
    for i in (0..6).rev() {
        let month_date = now - chrono::Months::new(i);
        let year = month_date.year();
        let month = month_date.month();
        let count = bookings
            .iter()
            .filter(|b| b.start_time.year() == year && b.start_time.month() == month)
            .count() as i32;
        monthly_trend.push(MonthlyTrend {
            month: format!("{:04}-{:02}", year, month),
            bookings: count,
        });
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(PersonalStats {
            total_bookings,
            favorite_lot,
            avg_duration_minutes,
            busiest_day,
            credits_spent,
            monthly_trend,
        })),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_query_defaults() {
        let json = "{}";
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert!(query.lot_id.is_none());
        assert!(query.from.is_none());
        assert!(query.to.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn test_history_query_with_lot_filter() {
        let json = r#"{"lot_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert!(query.lot_id.is_some());
    }

    #[test]
    fn test_history_query_with_date_range() {
        let json = r#"{"from":"2026-01-01T00:00:00Z","to":"2026-03-01T00:00:00Z"}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert!(query.from.is_some());
        assert!(query.to.is_some());
    }

    #[test]
    fn test_history_query_with_pagination() {
        let json = r#"{"page":2,"per_page":10}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, Some(2));
        assert_eq!(query.per_page, Some(10));
    }

    #[test]
    fn test_personal_stats_serialization() {
        let stats = PersonalStats {
            total_bookings: 42,
            favorite_lot: Some("Garage Alpha".to_string()),
            avg_duration_minutes: 120.5,
            busiest_day: Some("Monday".to_string()),
            credits_spent: 150,
            monthly_trend: vec![
                MonthlyTrend {
                    month: "2026-01".to_string(),
                    bookings: 5,
                },
                MonthlyTrend {
                    month: "2026-02".to_string(),
                    bookings: 8,
                },
            ],
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_bookings\":42"));
        assert!(json.contains("\"favorite_lot\":\"Garage Alpha\""));
        assert!(json.contains("\"avg_duration_minutes\":120.5"));
        assert!(json.contains("\"busiest_day\":\"Monday\""));
        assert!(json.contains("\"credits_spent\":150"));
    }

    #[test]
    fn test_personal_stats_empty() {
        let stats = PersonalStats {
            total_bookings: 0,
            favorite_lot: None,
            avg_duration_minutes: 0.0,
            busiest_day: None,
            credits_spent: 0,
            monthly_trend: vec![],
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deser: PersonalStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.total_bookings, 0);
        assert!(deser.favorite_lot.is_none());
    }

    #[test]
    fn test_monthly_trend_serialization() {
        let trend = MonthlyTrend {
            month: "2026-03".to_string(),
            bookings: 12,
        };
        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("\"month\":\"2026-03\""));
        assert!(json.contains("\"bookings\":12"));
    }

    #[test]
    fn test_history_response_serialization() {
        let resp = HistoryResponse {
            items: vec![],
            page: 1,
            per_page: 20,
            total: 0,
            total_pages: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"total\":0"));
    }
}
