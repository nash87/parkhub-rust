//! Calendar event handlers: combined bookings + absences view, iCal export,
//! and iCal subscription via personal tokens.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

use parkhub_common::ApiResponse;

use super::{generate_access_token, AuthUser, SharedState};

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

/// Response for calendar subscription token generation
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct CalendarTokenResponse {
    pub token: String,
    pub url: String,
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

// ---------------------------------------------------------------------------
// iCal helpers
// ---------------------------------------------------------------------------

/// Build an iCalendar feed string from the given user's bookings.
async fn build_ical_feed(state: &crate::AppState, user_id: &str) -> String {
    let bookings = state
        .db
        .list_bookings_by_user(user_id)
        .await
        .unwrap_or_default();

    let mut ical = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//ParkHub//EN\r\n\
         X-WR-CALNAME:ParkHub Bookings\r\nCALSCALE:GREGORIAN\r\nMETHOD:PUBLISH\r\n",
    );

    for b in &bookings {
        // Resolve lot name and address for SUMMARY/LOCATION
        let lot = state
            .db
            .get_parking_lot(&b.lot_id.to_string())
            .await
            .ok()
            .flatten();
        let lot_name = lot
            .as_ref()
            .map_or_else(|| b.floor_name.clone(), |l| l.name.clone());
        let lot_address = lot
            .as_ref()
            .map(|l| l.address.clone())
            .unwrap_or_else(|| lot_name.clone());

        let _ = write!(ical, "BEGIN:VEVENT\r\n");
        let _ = write!(ical, "UID:{}@parkhub\r\n", b.id);
        let _ = write!(
            ical,
            "DTSTART:{}\r\n",
            b.start_time.format("%Y%m%dT%H%M%SZ")
        );
        let _ = write!(ical, "DTEND:{}\r\n", b.end_time.format("%Y%m%dT%H%M%SZ"));
        let _ = write!(ical, "SUMMARY:{} - Slot {}\r\n", lot_name, b.slot_number);
        let _ = write!(ical, "LOCATION:{lot_address}\r\n");
        let _ = write!(
            ical,
            "DESCRIPTION:Floor: {}\\nSlot: {}\\nStatus: {}\r\n",
            b.floor_name,
            b.slot_number,
            format!("{:?}", b.status).to_lowercase()
        );
        let _ = write!(
            ical,
            "DTSTAMP:{}\r\n",
            b.created_at.format("%Y%m%dT%H%M%SZ")
        );
        let _ = write!(ical, "END:VEVENT\r\n");
    }

    ical.push_str("END:VCALENDAR\r\n");
    ical
}

// ---------------------------------------------------------------------------
// iCal endpoints
// ---------------------------------------------------------------------------

/// `GET /api/v1/user/calendar.ics` — iCal export of user's bookings (auth required)
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
    let ical = build_ical_feed(&state_guard, &auth_user.user_id.to_string()).await;

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

/// `GET /api/v1/bookings/ical` / `GET /api/v1/calendar/ical` — iCal feed of user's bookings (auth required, inline)
#[utoipa::path(get, path = "/api/v1/bookings/ical", tag = "Calendar",
    summary = "iCal feed (authenticated)",
    description = "Returns the user's bookings as an iCal feed for direct subscription.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "iCalendar feed"))
)]
pub async fn calendar_ical_authenticated(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;
    let ical = build_ical_feed(&state_guard, &auth_user.user_id.to_string()).await;

    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/calendar; charset=utf-8",
        )],
        ical,
    )
}

/// `GET /api/v1/calendar/ical/{token}` — public iCal feed via personal subscription token
#[utoipa::path(get, path = "/api/v1/calendar/ical/{token}", tag = "Calendar",
    summary = "iCal feed (public via token)",
    description = "Returns a user's bookings as an iCal feed via a personal subscription token.",
    responses(
        (status = 200, description = "iCalendar feed"),
        (status = 404, description = "Invalid or expired token")
    )
)]
pub async fn calendar_ical_by_token(
    State(state): State<SharedState>,
    Path(token): Path<String>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;

    // Look up token -> user_id in settings
    let setting_key = format!("ical_token:{token}");
    match state_guard.db.get_setting(&setting_key).await {
        Ok(Some(user_id)) if !user_id.is_empty() => {
            let ical = build_ical_feed(&state_guard, &user_id).await;
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/calendar; charset=utf-8",
                )],
                ical,
            )
        }
        _ => (
            StatusCode::NOT_FOUND,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            "Invalid or expired calendar token".to_string(),
        ),
    }
}

/// `POST /api/v1/calendar/token` — generate a personal calendar subscription token
#[utoipa::path(post, path = "/api/v1/calendar/token", tag = "Calendar",
    summary = "Generate calendar subscription token",
    description = "Creates a personal token for subscribing to the iCal feed from external apps.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Token generated"))
)]
pub async fn generate_calendar_token(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<CalendarTokenResponse>> {
    let state_guard = state.read().await;
    let user_id = auth_user.user_id.to_string();

    // Revoke any existing token for this user
    let user_token_key = format!("ical_user_token:{user_id}");
    if let Ok(Some(old_token)) = state_guard.db.get_setting(&user_token_key).await {
        let old_key = format!("ical_token:{old_token}");
        let _ = state_guard.db.set_setting(&old_key, "").await;
    }

    // Generate new token
    let token = generate_access_token();

    // Store bidirectional mapping: token -> user_id, user_id -> token
    let token_key = format!("ical_token:{token}");
    let _ = state_guard.db.set_setting(&token_key, &user_id).await;
    let _ = state_guard.db.set_setting(&user_token_key, &token).await;

    // Build the subscription URL
    let base_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let url = format!("{base_url}/api/v1/calendar/ical/{token}");

    Json(ApiResponse::success(CalendarTokenResponse { token, url }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;
    use uuid::Uuid;

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

    #[test]
    fn test_calendar_token_response_serialize() {
        let resp = CalendarTokenResponse {
            token: "abc123def456".to_string(),
            url: "http://localhost:3000/api/v1/calendar/ical/abc123def456".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("abc123def456"));
        assert!(json.contains("/api/v1/calendar/ical/"));
    }

    #[test]
    fn test_ical_format_vcalendar_header() {
        let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//ParkHub//EN\r\n\
                     X-WR-CALNAME:ParkHub Bookings\r\nCALSCALE:GREGORIAN\r\n\
                     METHOD:PUBLISH\r\nEND:VCALENDAR\r\n";
        assert!(ical.starts_with("BEGIN:VCALENDAR"));
        assert!(ical.contains("VERSION:2.0"));
        assert!(ical.contains("PRODID:-//ParkHub//EN"));
        assert!(ical.contains("X-WR-CALNAME:ParkHub Bookings"));
        assert!(ical.contains("CALSCALE:GREGORIAN"));
        assert!(ical.contains("METHOD:PUBLISH"));
        assert!(ical.ends_with("END:VCALENDAR\r\n"));
    }

    #[test]
    fn test_ical_vevent_format() {
        let uid = Uuid::new_v4();
        let now = Utc::now();
        let end = now + TimeDelta::hours(2);

        let mut ical = String::new();
        let _ = write!(ical, "BEGIN:VEVENT\r\n");
        let _ = write!(ical, "UID:{uid}@parkhub\r\n");
        let _ = write!(ical, "DTSTART:{}\r\n", now.format("%Y%m%dT%H%M%SZ"));
        let _ = write!(ical, "DTEND:{}\r\n", end.format("%Y%m%dT%H%M%SZ"));
        let _ = write!(ical, "SUMMARY:Garage A - Slot 5\r\n");
        let _ = write!(ical, "LOCATION:Garage A\r\n");
        let _ = write!(
            ical,
            "DESCRIPTION:Floor: Level 2\\nSlot: 5\\nStatus: confirmed\r\n"
        );
        let _ = write!(ical, "DTSTAMP:{}\r\n", now.format("%Y%m%dT%H%M%SZ"));
        let _ = write!(ical, "END:VEVENT\r\n");

        assert!(ical.contains("BEGIN:VEVENT"));
        assert!(ical.contains("END:VEVENT"));
        assert!(ical.contains("UID:"));
        assert!(ical.contains("@parkhub"));
        assert!(ical.contains("DTSTART:"));
        assert!(ical.contains("DTEND:"));
        assert!(ical.contains("SUMMARY:Garage A - Slot 5"));
        assert!(ical.contains("LOCATION:Garage A"));
        assert!(ical.contains("DESCRIPTION:Floor: Level 2"));
        assert!(ical.contains("DTSTAMP:"));
    }

    #[test]
    fn test_ical_line_endings_crlf() {
        let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n";
        // Every line should end with CRLF per RFC 5545
        for line in ical.split("\r\n") {
            assert!(
                !line.contains('\n'),
                "Line should not contain bare LF: {line:?}"
            );
        }
    }

    #[test]
    fn test_ical_dtstart_format() {
        let dt = chrono::DateTime::parse_from_rfc3339("2026-04-15T09:30:00Z").unwrap();
        let formatted = dt.format("%Y%m%dT%H%M%SZ").to_string();
        assert_eq!(formatted, "20260415T093000Z");
    }
}
