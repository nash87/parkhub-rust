//! Calendar event handlers: combined bookings + absences view, iCal export.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

/// Query params for calendar events
#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Calendar event response
#[derive(Debug, Serialize)]
pub struct CalendarEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// `GET /api/v1/calendar/events` — return user's bookings + absences as calendar events
#[utoipa::path(get, path = "/api/v1/calendar/events", tag = "Calendar",
    summary = "Calendar events",
    description = "Returns bookings and absences as calendar events.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn calendar_events(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<CalendarQuery>,
) -> Json<ApiResponse<Vec<CalendarEvent>>> {
    let state_guard = state.read().await;
    let mut events = Vec::new();

    // Parse date range for filtering
    let from_date = query
        .from
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let to_date = query
        .to
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    // Bookings as events
    if let Ok(bookings) = state_guard
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        for b in bookings {
            if let Some(from) = from_date {
                if b.start_time.date_naive() < from {
                    continue;
                }
            }
            if let Some(to) = to_date {
                if b.start_time.date_naive() > to {
                    continue;
                }
            }

            events.push(CalendarEvent {
                id: b.id.to_string(),
                event_type: "booking".to_string(),
                title: format!("Parking - Slot {}", b.slot_number),
                start: b.start_time,
                end: b.end_time,
                lot_name: Some(b.floor_name.clone()),
                slot_number: Some(b.slot_number),
                status: Some(format!("{:?}", b.status).to_lowercase()),
            });
        }
    }

    // Absences as events
    if let Ok(absences) = state_guard
        .db
        .list_absences_by_user(&auth_user.user_id.to_string())
        .await
    {
        for a in absences {
            let start = chrono::NaiveDate::parse_from_str(&a.start_date, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
            let end = chrono::NaiveDate::parse_from_str(&a.end_date, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(23, 59, 59))
                .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

            if let (Some(start_dt), Some(end_dt)) = (start, end) {
                if let Some(from) = from_date {
                    if end_dt.date_naive() < from {
                        continue;
                    }
                }
                if let Some(to) = to_date {
                    if start_dt.date_naive() > to {
                        continue;
                    }
                }

                let type_label = format!("{:?}", a.absence_type);
                events.push(CalendarEvent {
                    id: a.id.to_string(),
                    event_type: "absence".to_string(),
                    title: type_label,
                    start: start_dt,
                    end: end_dt,
                    lot_name: None,
                    slot_number: None,
                    status: None,
                });
            }
        }
    }

    // Sort by start time
    events.sort_by(|a, b| a.start.cmp(&b.start));

    Json(ApiResponse::success(events))
}

/// `GET /api/v1/user/calendar.ics` — iCal export of user's bookings
#[utoipa::path(get, path = "/api/v1/user/calendar.ics", tag = "Calendar",
    summary = "Export bookings as iCal",
    description = "Returns the user's bookings in iCalendar format.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn user_calendar_ics(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;
    let bookings = state_guard
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let mut ical = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//ParkHub//EN\r\n");

    for b in &bookings {
        let _ = writeln!(ical, "BEGIN:VEVENT");
        let _ = writeln!(ical, "UID:{}", b.id);
        let _ = writeln!(ical, "DTSTART:{}", b.start_time.format("%Y%m%dT%H%M%SZ"));
        let _ = writeln!(ical, "DTEND:{}", b.end_time.format("%Y%m%dT%H%M%SZ"));
        let _ = writeln!(
            ical,
            "SUMMARY:Parking Slot {} ({})",
            b.slot_number,
            format!("{:?}", b.status).to_lowercase()
        );
        let _ = writeln!(ical, "DESCRIPTION:Floor: {}", b.floor_name);
        let _ = writeln!(ical, "END:VEVENT");
    }

    ical.push_str("END:VCALENDAR\r\n");

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/calendar; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"parkhub.ics\"",
            ),
        ],
        ical,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    #[test]
    fn test_calendar_query_deserialize() {
        let json = r#"{"from":"2026-03-01","to":"2026-03-31"}"#;
        let q: CalendarQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.from.as_deref(), Some("2026-03-01"));
        assert_eq!(q.to.as_deref(), Some("2026-03-31"));
    }

    #[test]
    fn test_calendar_event_serialize_skip_none() {
        let event = CalendarEvent {
            id: "evt-1".to_string(),
            event_type: "booking".to_string(),
            title: "Slot A3".to_string(),
            start: Utc::now(),
            end: Utc::now() + TimeDelta::hours(2),
            lot_name: None,
            slot_number: None,
            status: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("lot_name"));
        assert!(!json.contains("slot_number"));
        assert!(!json.contains("status"));
    }

    #[test]
    fn test_calendar_event_serialize_with_optionals() {
        let event = CalendarEvent {
            id: "evt-2".to_string(),
            event_type: "booking".to_string(),
            title: "Slot B1".to_string(),
            start: Utc::now(),
            end: Utc::now() + TimeDelta::hours(1),
            lot_name: Some("Lot Alpha".to_string()),
            slot_number: Some(42),
            status: Some("confirmed".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Lot Alpha"));
        assert!(json.contains("42"));
        assert!(json.contains("confirmed"));
        // Check rename
        assert!(json.contains(r#""type":"booking"#));
    }
}
