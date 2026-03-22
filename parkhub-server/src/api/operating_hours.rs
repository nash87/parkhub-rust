//! Operating hours handlers: per-lot schedule management and validation.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Datelike, NaiveTime, Utc};

use parkhub_common::{ApiResponse, DayHours, OperatingHours};

use super::SharedState;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a time string like "07:00" into a NaiveTime.
fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

/// Get the DayHours for a given weekday (0 = Monday .. 6 = Sunday).
fn day_hours_for_weekday(hours: &OperatingHours, weekday: u32) -> Option<&DayHours> {
    match weekday {
        0 => hours.monday.as_ref(),
        1 => hours.tuesday.as_ref(),
        2 => hours.wednesday.as_ref(),
        3 => hours.thursday.as_ref(),
        4 => hours.friday.as_ref(),
        5 => hours.saturday.as_ref(),
        6 => hours.sunday.as_ref(),
        _ => None,
    }
}

/// Check if a lot is currently open based on its operating hours.
pub fn is_lot_open_now(hours: &OperatingHours) -> bool {
    if hours.is_24h {
        return true;
    }
    let now = Utc::now();
    is_lot_open_at(hours, &now)
}

/// Check if a lot is open at a given datetime.
pub fn is_lot_open_at(hours: &OperatingHours, dt: &DateTime<Utc>) -> bool {
    if hours.is_24h {
        return true;
    }
    // chrono weekday: Mon=0 .. Sun=6
    let weekday = dt.weekday().num_days_from_monday();
    let Some(day) = day_hours_for_weekday(hours, weekday) else {
        // No hours defined for this day = closed
        return false;
    };
    if day.closed {
        return false;
    }
    let Some(open) = parse_time(&day.open) else {
        return false;
    };
    let Some(close) = parse_time(&day.close) else {
        return false;
    };
    let current_time = dt.time();

    if close > open {
        // Normal hours (e.g., 07:00 - 22:00)
        current_time >= open && current_time < close
    } else {
        // Overnight hours (e.g., 22:00 - 06:00)
        current_time >= open || current_time < close
    }
}

/// Validate that a booking time range falls within operating hours.
/// Returns an error message if the booking is outside operating hours.
pub fn validate_booking_hours(
    hours: &OperatingHours,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
) -> Option<String> {
    if hours.is_24h {
        return None;
    }

    // Check start time
    if !is_lot_open_at(hours, start) {
        let weekday = start.weekday().num_days_from_monday();
        let day_name = weekday_name(weekday);
        return Some(format!(
            "Lot is not open at the requested start time ({day_name})"
        ));
    }

    // Check end time
    if !is_lot_open_at(hours, end) {
        let weekday = end.weekday().num_days_from_monday();
        let day_name = weekday_name(weekday);
        return Some(format!(
            "Lot is not open at the requested end time ({day_name})"
        ));
    }

    None
}

fn weekday_name(weekday: u32) -> &'static str {
    match weekday {
        0 => "Monday",
        1 => "Tuesday",
        2 => "Wednesday",
        3 => "Thursday",
        4 => "Friday",
        5 => "Saturday",
        6 => "Sunday",
        _ => "Unknown",
    }
}

/// Response including open/closed status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperatingHoursResponse {
    /// The configured operating hours
    #[serde(flatten)]
    pub hours: OperatingHours,
    /// Whether the lot is currently open
    pub is_open_now: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/{id}/hours` — returns operating hours + current open/closed status.
#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}/hours",
    tag = "Operating Hours",
    summary = "Get lot operating hours",
    description = "Returns the operating hours schedule and whether the lot is currently open.",
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Operating hours"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn get_operating_hours(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<OperatingHoursResponse>>) {
    let state = state.read().await;

    match state.db.get_parking_lot(&id).await {
        Ok(Some(lot)) => {
            let is_open = is_lot_open_now(&lot.operating_hours);
            let resp = OperatingHoursResponse {
                hours: lot.operating_hours,
                is_open_now: is_open,
            };
            (StatusCode::OK, Json(ApiResponse::success(resp)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
        ),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get parking lot");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

/// `PUT /api/v1/admin/lots/{id}/hours` — admin: set operating hours for a lot.
#[utoipa::path(
    put,
    path = "/api/v1/admin/lots/{id}/hours",
    tag = "Operating Hours",
    summary = "Update lot operating hours (admin)",
    description = "Set the operating hours schedule for a parking lot.",
    params(("id" = String, Path, description = "Parking lot ID")),
    request_body = inline(serde_json::Value),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Updated operating hours"),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn admin_update_operating_hours(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(new_hours): Json<OperatingHours>,
) -> (StatusCode, Json<ApiResponse<OperatingHoursResponse>>) {
    let state = state.read().await;

    // Validate times
    if !new_hours.is_24h {
        for (day_name, day_opt) in [
            ("monday", &new_hours.monday),
            ("tuesday", &new_hours.tuesday),
            ("wednesday", &new_hours.wednesday),
            ("thursday", &new_hours.thursday),
            ("friday", &new_hours.friday),
            ("saturday", &new_hours.saturday),
            ("sunday", &new_hours.sunday),
        ] {
            if let Some(day) = day_opt {
                if !day.closed {
                    if parse_time(&day.open).is_none() {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::error(
                                "VALIDATION_ERROR",
                                format!("Invalid open time for {day_name}"),
                            )),
                        );
                    }
                    if parse_time(&day.close).is_none() {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::error(
                                "VALIDATION_ERROR",
                                format!("Invalid close time for {day_name}"),
                            )),
                        );
                    }
                }
            }
        }
    }

    // Fetch and update lot
    let mut lot = match state.db.get_parking_lot(&id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get parking lot");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    lot.operating_hours = new_hours;
    lot.updated_at = Utc::now();

    if let Err(e) = state.db.save_parking_lot(&lot).await {
        tracing::error!(error = %e, "Failed to save parking lot");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update operating hours",
            )),
        );
    }

    let is_open = is_lot_open_now(&lot.operating_hours);
    let resp = OperatingHoursResponse {
        hours: lot.operating_hours,
        is_open_now: is_open,
    };

    tracing::info!(lot_id = %id, "Updated operating hours");
    (StatusCode::OK, Json(ApiResponse::success(resp)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_24h() -> OperatingHours {
        OperatingHours {
            is_24h: true,
            monday: None,
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        }
    }

    fn make_weekday_hours() -> OperatingHours {
        let weekday = DayHours {
            open: "07:00".to_string(),
            close: "22:00".to_string(),
            closed: false,
        };
        let weekend = DayHours {
            open: "09:00".to_string(),
            close: "18:00".to_string(),
            closed: false,
        };
        let sunday = DayHours {
            open: "00:00".to_string(),
            close: "00:00".to_string(),
            closed: true,
        };
        OperatingHours {
            is_24h: false,
            monday: Some(weekday.clone()),
            tuesday: Some(weekday.clone()),
            wednesday: Some(weekday.clone()),
            thursday: Some(weekday.clone()),
            friday: Some(weekday),
            saturday: Some(weekend),
            sunday: Some(sunday),
        }
    }

    #[test]
    fn test_24h_always_open() {
        let hours = make_24h();
        // Monday 3am
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 3, 0, 0).unwrap(); // Monday
        assert!(is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_weekday_open_during_hours() {
        let hours = make_weekday_hours();
        // Monday 10:00
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 10, 0, 0).unwrap();
        assert!(is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_weekday_closed_before_open() {
        let hours = make_weekday_hours();
        // Monday 05:00 (before 07:00 open)
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 5, 0, 0).unwrap();
        assert!(!is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_weekday_closed_after_close() {
        let hours = make_weekday_hours();
        // Monday 23:00 (after 22:00 close)
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 23, 0, 0).unwrap();
        assert!(!is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_at_open_time_is_open() {
        let hours = make_weekday_hours();
        // Monday exactly at 07:00
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 7, 0, 0).unwrap();
        assert!(is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_at_close_time_is_closed() {
        let hours = make_weekday_hours();
        // Monday exactly at 22:00 (close time = closed)
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 22, 0, 0).unwrap();
        assert!(!is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_saturday_shorter_hours() {
        let hours = make_weekday_hours();
        // Saturday 15:00 (within 09:00-18:00)
        let dt = Utc.with_ymd_and_hms(2026, 3, 28, 15, 0, 0).unwrap();
        assert!(is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_saturday_closed_late() {
        let hours = make_weekday_hours();
        // Saturday 19:00 (after 18:00 close)
        let dt = Utc.with_ymd_and_hms(2026, 3, 28, 19, 0, 0).unwrap();
        assert!(!is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_sunday_closed() {
        let hours = make_weekday_hours();
        // Sunday 12:00 (closed flag)
        let dt = Utc.with_ymd_and_hms(2026, 3, 29, 12, 0, 0).unwrap();
        assert!(!is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_no_hours_defined_is_closed() {
        let hours = OperatingHours {
            is_24h: false,
            monday: None,
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        };
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 12, 0, 0).unwrap();
        assert!(!is_lot_open_at(&hours, &dt));
    }

    #[test]
    fn test_overnight_hours() {
        let hours = OperatingHours {
            is_24h: false,
            monday: Some(DayHours {
                open: "22:00".to_string(),
                close: "06:00".to_string(),
                closed: false,
            }),
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        };
        // Monday 23:00 (within 22:00-06:00)
        let dt = Utc.with_ymd_and_hms(2026, 3, 23, 23, 0, 0).unwrap();
        assert!(is_lot_open_at(&hours, &dt));
        // Monday 02:00 (within 22:00-06:00, but checked on Monday schedule)
        let dt2 = Utc.with_ymd_and_hms(2026, 3, 23, 2, 0, 0).unwrap();
        assert!(is_lot_open_at(&hours, &dt2));
    }

    #[test]
    fn test_validate_booking_24h() {
        let hours = make_24h();
        let start = Utc.with_ymd_and_hms(2026, 3, 23, 3, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 23, 5, 0, 0).unwrap();
        assert!(validate_booking_hours(&hours, &start, &end).is_none());
    }

    #[test]
    fn test_validate_booking_within_hours() {
        let hours = make_weekday_hours();
        let start = Utc.with_ymd_and_hms(2026, 3, 23, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 23, 14, 0, 0).unwrap();
        assert!(validate_booking_hours(&hours, &start, &end).is_none());
    }

    #[test]
    fn test_validate_booking_start_outside_hours() {
        let hours = make_weekday_hours();
        let start = Utc.with_ymd_and_hms(2026, 3, 23, 5, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 23, 10, 0, 0).unwrap();
        let err = validate_booking_hours(&hours, &start, &end);
        assert!(err.is_some());
        assert!(err.unwrap().contains("start time"));
    }

    #[test]
    fn test_validate_booking_end_outside_hours() {
        let hours = make_weekday_hours();
        let start = Utc.with_ymd_and_hms(2026, 3, 23, 20, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 23, 23, 0, 0).unwrap();
        let err = validate_booking_hours(&hours, &start, &end);
        assert!(err.is_some());
        assert!(err.unwrap().contains("end time"));
    }

    #[test]
    fn test_validate_booking_on_closed_day() {
        let hours = make_weekday_hours();
        // Sunday is closed
        let start = Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 29, 12, 0, 0).unwrap();
        let err = validate_booking_hours(&hours, &start, &end);
        assert!(err.is_some());
    }

    #[test]
    fn test_parse_time_valid() {
        assert!(parse_time("07:00").is_some());
        assert!(parse_time("23:59").is_some());
        assert!(parse_time("00:00").is_some());
    }

    #[test]
    fn test_parse_time_invalid() {
        assert!(parse_time("25:00").is_none());
        assert!(parse_time("abc").is_none());
        assert!(parse_time("").is_none());
    }

    #[test]
    fn test_operating_hours_serde_roundtrip() {
        let hours = make_weekday_hours();
        let json = serde_json::to_string(&hours).unwrap();
        let back: OperatingHours = serde_json::from_str(&json).unwrap();
        assert!(!back.is_24h);
        assert!(back.monday.is_some());
        assert!(back.sunday.as_ref().unwrap().closed);
    }

    #[test]
    fn test_day_hours_closed_default() {
        // closed defaults to false when not present in JSON
        let json = r#"{"open":"07:00","close":"22:00"}"#;
        let day: DayHours = serde_json::from_str(json).unwrap();
        assert!(!day.closed);
        assert_eq!(day.open, "07:00");
        assert_eq!(day.close, "22:00");
    }

    #[test]
    fn test_response_serde() {
        let resp = OperatingHoursResponse {
            hours: make_24h(),
            is_open_now: true,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"is_open_now\":true"));
        assert!(json.contains("\"is_24h\":true"));
    }
}
