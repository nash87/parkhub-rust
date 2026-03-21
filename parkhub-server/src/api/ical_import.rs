//! iCalendar (.ics) import for absences.
//!
//! Parses VEVENT entries from an iCalendar file/text and creates absence
//! records for the authenticated user.

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::models::{Absence, AbsenceType};
use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

/// Request body for `POST /api/v1/absences/import`.
///
/// Accepts raw iCalendar text. The endpoint parses VEVENT blocks and creates
/// one absence per event.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct IcalImportRequest {
    /// Raw iCalendar (.ics) text content
    pub ical_text: String,
}

/// Summary of a single imported absence.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ImportedAbsence {
    pub id: String,
    pub absence_type: AbsenceType,
    pub start_date: String,
    pub end_date: String,
    pub note: Option<String>,
}

/// Response from the import endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IcalImportResponse {
    /// Number of absences successfully imported
    pub imported: usize,
    /// Number of events skipped (invalid dates, duplicates, etc.)
    pub skipped: usize,
    /// Details of imported absences
    pub absences: Vec<ImportedAbsence>,
}

/// A parsed VEVENT from iCalendar data.
#[derive(Debug)]
struct ParsedEvent {
    dtstart: String,
    dtend: Option<String>,
    summary: Option<String>,
}

/// Parse iCalendar text into a list of VEVENT entries.
///
/// Supports both datetime (YYYYMMDDTHHMMSSZ) and date-only (YYYYMMDD) formats
/// for DTSTART and DTEND. Timezone suffixes and parameters (e.g.
/// `DTSTART;VALUE=DATE:20260301`) are handled.
fn parse_ical_events(text: &str) -> Vec<ParsedEvent> {
    let mut events = Vec::new();
    let mut in_event = false;
    let mut dtstart = None;
    let mut dtend = None;
    let mut summary = None;

    for line in text.lines() {
        let line = line.trim_end_matches('\r').trim();

        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            in_event = true;
            dtstart = None;
            dtend = None;
            summary = None;
        } else if line.eq_ignore_ascii_case("END:VEVENT") {
            if in_event {
                if let Some(start) = dtstart.take() {
                    events.push(ParsedEvent {
                        dtstart: start,
                        dtend: dtend.take(),
                        summary: summary.take(),
                    });
                }
            }
            in_event = false;
        } else if in_event {
            if let Some(val) = extract_ical_property(line, "DTSTART") {
                dtstart = Some(normalize_ical_date(&val));
            } else if let Some(val) = extract_ical_property(line, "DTEND") {
                dtend = Some(normalize_ical_date(&val));
            } else if let Some(val) = extract_ical_property(line, "SUMMARY") {
                summary = Some(val);
            }
        }
    }

    events
}

/// Extract the value of an iCal property, handling parameters.
///
/// E.g. `DTSTART;VALUE=DATE:20260301` → `"20260301"`
/// E.g. `SUMMARY:Team offsite` → `"Team offsite"`
fn extract_ical_property(line: &str, prop: &str) -> Option<String> {
    let upper = line.to_uppercase();
    if !upper.starts_with(prop) {
        return None;
    }
    let rest = &line[prop.len()..];
    // After the property name, expect either ':' or ';' (parameters)
    if let Some(colon_pos) = rest.find(':') {
        let c = rest.as_bytes().first()?;
        if *c == b':' || *c == b';' {
            return Some(rest[colon_pos + 1..].to_string());
        }
    }
    None
}

/// Normalize an iCal date/datetime value to `YYYY-MM-DD`.
///
/// Handles:
/// - `YYYYMMDD` (date-only)
/// - `YYYYMMDDTHHMMSS` (local datetime)
/// - `YYYYMMDDTHHMMSSZ` (UTC datetime)
fn normalize_ical_date(raw: &str) -> String {
    let s = raw.trim();
    // Take only the date part (first 8 chars)
    if s.len() >= 8 {
        let date_part = &s[..8];
        if date_part.chars().all(|c| c.is_ascii_digit()) {
            return format!(
                "{}-{}-{}",
                &date_part[..4],
                &date_part[4..6],
                &date_part[6..8]
            );
        }
    }
    // Fallback: return as-is (will fail validation later)
    s.to_string()
}

/// Guess the absence type from the VEVENT SUMMARY field.
///
/// Keywords: "vacation", "urlaub", "holiday" → Vacation;
/// "sick", "krank", "illness" → Sick;
/// "homeoffice", "home office", "remote" → Homeoffice;
/// "training", "schulung", "workshop" → Training;
/// Otherwise → Other.
fn guess_absence_type(summary: Option<&str>) -> AbsenceType {
    let Some(s) = summary else {
        return AbsenceType::Other;
    };
    let lower = s.to_lowercase();
    if lower.contains("vacation")
        || lower.contains("urlaub")
        || lower.contains("holiday")
        || lower.contains("ferien")
    {
        AbsenceType::Vacation
    } else if lower.contains("sick")
        || lower.contains("krank")
        || lower.contains("illness")
    {
        AbsenceType::Sick
    } else if lower.contains("homeoffice")
        || lower.contains("home office")
        || lower.contains("remote")
    {
        AbsenceType::Homeoffice
    } else if lower.contains("training")
        || lower.contains("schulung")
        || lower.contains("workshop")
    {
        AbsenceType::Training
    } else {
        AbsenceType::Other
    }
}

/// Validate a date string is YYYY-MM-DD format.
fn is_valid_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// `POST /api/v1/absences/import` — import absences from iCalendar data
#[utoipa::path(
    post,
    path = "/api/v1/absences/import",
    tag = "Absences",
    summary = "Import absences from iCal",
    description = "Parse an iCalendar (.ics) file and create absence records for each VEVENT. Supports date-only (YYYYMMDD) and datetime formats. Auto-detects absence type from SUMMARY.",
    security(("bearer_auth" = [])),
    request_body = IcalImportRequest,
    responses(
        (status = 200, description = "Import result with created absences"),
        (status = 400, description = "No valid events found or empty input"),
    )
)]
pub async fn import_absences_ical(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<IcalImportRequest>,
) -> (StatusCode, Json<ApiResponse<IcalImportResponse>>) {
    if req.ical_text.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "ical_text must not be empty",
            )),
        );
    }

    let events = parse_ical_events(&req.ical_text);

    if events.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "No VEVENT entries found in iCalendar data",
            )),
        );
    }

    let state_guard = state.read().await;
    let mut imported = Vec::new();
    let mut skipped = 0usize;

    for event in &events {
        let start = &event.dtstart;
        let end = event.dtend.as_deref().unwrap_or(start);

        if !is_valid_date(start) || !is_valid_date(end) {
            skipped += 1;
            continue;
        }
        if start > end {
            skipped += 1;
            continue;
        }

        let absence_type = guess_absence_type(event.summary.as_deref());

        let absence = Absence {
            id: Uuid::new_v4(),
            user_id: auth_user.user_id,
            absence_type: absence_type.clone(),
            start_date: start.clone(),
            end_date: end.clone(),
            note: event.summary.clone(),
            source: "ical_import".to_string(),
            created_at: Utc::now(),
        };

        match state_guard.db.save_absence(&absence).await {
            Ok(()) => {
                imported.push(ImportedAbsence {
                    id: absence.id.to_string(),
                    absence_type,
                    start_date: start.clone(),
                    end_date: end.clone(),
                    note: event.summary.clone(),
                });
            }
            Err(e) => {
                tracing::warn!("Failed to save imported absence: {}", e);
                skipped += 1;
            }
        }
    }
    drop(state_guard);

    tracing::info!(
        "iCal import: {} imported, {} skipped for user {}",
        imported.len(),
        skipped,
        auth_user.user_id
    );

    let response = IcalImportResponse {
        imported: imported.len(),
        skipped,
        absences: imported,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_ical_events ──

    #[test]
    fn parse_single_vevent() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART:20260301\r\nDTEND:20260305\r\nSUMMARY:Vacation\r\nEND:VEVENT\r\nEND:VCALENDAR";
        let events = parse_ical_events(ical);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].dtstart, "2026-03-01");
        assert_eq!(events[0].dtend.as_deref(), Some("2026-03-05"));
        assert_eq!(events[0].summary.as_deref(), Some("Vacation"));
    }

    #[test]
    fn parse_multiple_vevents() {
        let ical = "\
BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20260301\r\n\
DTEND:20260302\r\n\
SUMMARY:Day 1\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20260310\r\n\
DTEND:20260315\r\n\
SUMMARY:Day 2\r\n\
END:VEVENT\r\n\
END:VCALENDAR";
        let events = parse_ical_events(ical);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn parse_datetime_format() {
        let ical = "BEGIN:VEVENT\r\nDTSTART:20260615T090000Z\r\nDTEND:20260620T170000Z\r\nSUMMARY:Conference\r\nEND:VEVENT";
        let events = parse_ical_events(ical);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].dtstart, "2026-06-15");
        assert_eq!(events[0].dtend.as_deref(), Some("2026-06-20"));
    }

    #[test]
    fn parse_value_date_parameter() {
        let ical = "BEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260401\r\nDTEND;VALUE=DATE:20260405\r\nSUMMARY:Easter\r\nEND:VEVENT";
        let events = parse_ical_events(ical);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].dtstart, "2026-04-01");
        assert_eq!(events[0].dtend.as_deref(), Some("2026-04-05"));
    }

    #[test]
    fn parse_missing_dtend() {
        let ical = "BEGIN:VEVENT\r\nDTSTART:20260301\r\nSUMMARY:Single day\r\nEND:VEVENT";
        let events = parse_ical_events(ical);
        assert_eq!(events.len(), 1);
        assert!(events[0].dtend.is_none());
    }

    #[test]
    fn parse_no_events() {
        let ical = "BEGIN:VCALENDAR\r\nPRODID:-//Test//EN\r\nEND:VCALENDAR";
        let events = parse_ical_events(ical);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_event_without_dtstart_is_skipped() {
        let ical = "BEGIN:VEVENT\r\nSUMMARY:No start\r\nEND:VEVENT";
        let events = parse_ical_events(ical);
        assert!(events.is_empty());
    }

    // ── normalize_ical_date ──

    #[test]
    fn normalize_date_only() {
        assert_eq!(normalize_ical_date("20260315"), "2026-03-15");
    }

    #[test]
    fn normalize_datetime_utc() {
        assert_eq!(normalize_ical_date("20260315T120000Z"), "2026-03-15");
    }

    #[test]
    fn normalize_datetime_local() {
        assert_eq!(normalize_ical_date("20260315T120000"), "2026-03-15");
    }

    // ── guess_absence_type ──

    #[test]
    fn guess_vacation() {
        assert_eq!(guess_absence_type(Some("Vacation")), AbsenceType::Vacation);
        assert_eq!(guess_absence_type(Some("Urlaub 2026")), AbsenceType::Vacation);
        assert_eq!(guess_absence_type(Some("Public Holiday")), AbsenceType::Vacation);
    }

    #[test]
    fn guess_sick() {
        assert_eq!(guess_absence_type(Some("Sick leave")), AbsenceType::Sick);
        assert_eq!(guess_absence_type(Some("Krank")), AbsenceType::Sick);
    }

    #[test]
    fn guess_homeoffice() {
        assert_eq!(
            guess_absence_type(Some("Homeoffice")),
            AbsenceType::Homeoffice
        );
        assert_eq!(
            guess_absence_type(Some("Remote work")),
            AbsenceType::Homeoffice
        );
    }

    #[test]
    fn guess_training() {
        assert_eq!(
            guess_absence_type(Some("Training session")),
            AbsenceType::Training
        );
        assert_eq!(
            guess_absence_type(Some("Schulung")),
            AbsenceType::Training
        );
    }

    #[test]
    fn guess_other_for_unknown() {
        assert_eq!(guess_absence_type(Some("Team meeting")), AbsenceType::Other);
        assert_eq!(guess_absence_type(None), AbsenceType::Other);
    }

    // ── extract_ical_property ──

    #[test]
    fn extract_simple_property() {
        assert_eq!(
            extract_ical_property("SUMMARY:Team offsite", "SUMMARY"),
            Some("Team offsite".to_string())
        );
    }

    #[test]
    fn extract_property_with_params() {
        assert_eq!(
            extract_ical_property("DTSTART;VALUE=DATE:20260301", "DTSTART"),
            Some("20260301".to_string())
        );
    }

    #[test]
    fn extract_wrong_property_returns_none() {
        assert_eq!(
            extract_ical_property("DTEND:20260301", "DTSTART"),
            None
        );
    }

    // ── IcalImportRequest serde ──

    #[test]
    fn ical_import_request_deserialize() {
        let json = r#"{"ical_text": "BEGIN:VCALENDAR\nEND:VCALENDAR"}"#;
        let req: IcalImportRequest = serde_json::from_str(json).unwrap();
        assert!(req.ical_text.contains("VCALENDAR"));
    }

    // ── is_valid_date ──

    #[test]
    fn valid_date_accepted() {
        assert!(is_valid_date("2026-03-15"));
        assert!(is_valid_date("2026-12-31"));
    }

    #[test]
    fn invalid_date_rejected() {
        assert!(!is_valid_date("20260315")); // no hyphens
        assert!(!is_valid_date("2026-13-01")); // month > 12
        assert!(!is_valid_date("not-a-date"));
    }
}
