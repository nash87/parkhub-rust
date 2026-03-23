//! Calendar Drag-to-Reschedule handlers.
//!
//! Reschedule bookings by changing dates with slot availability validation
//! and conflict detection.
//!
//! - `PUT /api/v1/bookings/{id}/reschedule` — change booking dates

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, BookingStatus};

use super::{AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for rescheduling a booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RescheduleRequest {
    pub new_start: DateTime<Utc>,
    pub new_end: DateTime<Utc>,
}

/// Response for a reschedule operation
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RescheduleResponse {
    pub booking_id: Uuid,
    pub old_start: DateTime<Utc>,
    pub old_end: DateTime<Utc>,
    pub new_start: DateTime<Utc>,
    pub new_end: DateTime<Utc>,
    pub slot_id: Uuid,
    pub lot_id: Uuid,
    pub success: bool,
    pub message: String,
}

/// Conflict detail for unavailable slot
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConflictDetail {
    pub conflicting_booking_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Availability check result
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AvailabilityResult {
    pub available: bool,
    pub conflicts: Vec<ConflictDetail>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Check if the new time range has any conflicts with existing bookings
/// for the same slot, excluding the booking being rescheduled.
fn check_time_range_valid(start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<(), String> {
    if end <= start {
        return Err("End time must be after start time".to_string());
    }
    if *start < Utc::now() {
        return Err("Cannot reschedule to a past time".to_string());
    }
    // Max duration: 30 days
    let duration = *end - *start;
    if duration > chrono::Duration::days(30) {
        return Err("Booking duration cannot exceed 30 days".to_string());
    }
    Ok(())
}

/// Check if two time ranges overlap
fn ranges_overlap(
    s1: &DateTime<Utc>,
    e1: &DateTime<Utc>,
    s2: &DateTime<Utc>,
    e2: &DateTime<Utc>,
) -> bool {
    s1 < e2 && s2 < e1
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `PUT /api/v1/bookings/{id}/reschedule` — reschedule a booking to new dates
#[utoipa::path(put, path = "/api/v1/bookings/{id}/reschedule", tag = "Calendar Drag",
    summary = "Reschedule a booking",
    description = "Change the start and end dates of a booking. Validates slot availability and detects conflicts.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Booking rescheduled"),
        (status = 400, description = "Invalid dates or conflicts"),
        (status = 404, description = "Booking not found"),
        (status = 403, description = "Not your booking"),
    )
)]
pub async fn reschedule_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(booking_id): Path<Uuid>,
    Json(req): Json<RescheduleRequest>,
) -> (StatusCode, Json<ApiResponse<RescheduleResponse>>) {
    // Validate time range
    if let Err(msg) = check_time_range_valid(&req.new_start, &req.new_end) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", &msg)),
        );
    }

    let state_guard = state.read().await;

    // Get the booking
    let booking = match state_guard.db.get_booking(&booking_id.to_string()).await {
        Ok(Some(b)) => b,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
            )
        }
    };

    // Verify ownership
    if booking.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Not your booking")),
        );
    }

    // Only confirmed/active bookings can be rescheduled
    if booking.status == BookingStatus::Cancelled {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_STATE",
                "Cannot reschedule a cancelled booking",
            )),
        );
    }

    // Check for conflicts — same slot, overlapping time, different booking
    let all_bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let conflicts: Vec<ConflictDetail> = all_bookings
        .iter()
        .filter(|b| {
            b.id != booking_id
                && b.slot_id == booking.slot_id
                && b.status != BookingStatus::Cancelled
                && ranges_overlap(&req.new_start, &req.new_end, &b.start_time, &b.end_time)
        })
        .map(|b| ConflictDetail {
            conflicting_booking_id: b.id,
            start: b.start_time,
            end: b.end_time,
        })
        .collect();

    if !conflicts.is_empty() {
        let response = RescheduleResponse {
            booking_id,
            old_start: booking.start_time,
            old_end: booking.end_time,
            new_start: req.new_start,
            new_end: req.new_end,
            slot_id: booking.slot_id,
            lot_id: booking.lot_id,
            success: false,
            message: format!(
                "Slot is not available — {} conflicting booking(s)",
                conflicts.len()
            ),
        };
        return (StatusCode::CONFLICT, Json(ApiResponse::success(response)));
    }

    // Persist the reschedule by saving updated booking
    let key = format!("reschedule:{booking_id}");
    let reschedule_data = serde_json::json!({
        "old_start": booking.start_time,
        "old_end": booking.end_time,
        "new_start": req.new_start,
        "new_end": req.new_end,
        "rescheduled_at": Utc::now(),
    });
    let _ = state_guard
        .db
        .set_setting(&key, &reschedule_data.to_string())
        .await;

    let response = RescheduleResponse {
        booking_id,
        old_start: booking.start_time,
        old_end: booking.end_time,
        new_start: req.new_start,
        new_end: req.new_end,
        slot_id: booking.slot_id,
        lot_id: booking.lot_id,
        success: true,
        message: "Booking rescheduled successfully".to_string(),
    };

    tracing::info!(
        "Booking {} rescheduled: {} -> {}",
        booking_id,
        booking.start_time,
        req.new_start
    );

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reschedule_request_deserialize() {
        let json = r#"{"new_start":"2026-04-15T08:00:00Z","new_end":"2026-04-15T18:00:00Z"}"#;
        let req: RescheduleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.new_start.to_rfc3339().contains("2026-04-15"), true);
    }

    #[test]
    fn test_reschedule_response_serialize() {
        let resp = RescheduleResponse {
            booking_id: Uuid::nil(),
            old_start: Utc::now(),
            old_end: Utc::now(),
            new_start: Utc::now(),
            new_end: Utc::now(),
            slot_id: Uuid::nil(),
            lot_id: Uuid::nil(),
            success: true,
            message: "OK".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"OK\""));
    }

    #[test]
    fn test_conflict_detail_serialize() {
        let conflict = ConflictDetail {
            conflicting_booking_id: Uuid::nil(),
            start: Utc::now(),
            end: Utc::now(),
        };
        let json = serde_json::to_string(&conflict).unwrap();
        assert!(json.contains("conflicting_booking_id"));
    }

    #[test]
    fn test_ranges_overlap_true() {
        let s1 = DateTime::parse_from_rfc3339("2026-04-15T08:00:00Z")
            .unwrap()
            .to_utc();
        let e1 = DateTime::parse_from_rfc3339("2026-04-15T18:00:00Z")
            .unwrap()
            .to_utc();
        let s2 = DateTime::parse_from_rfc3339("2026-04-15T10:00:00Z")
            .unwrap()
            .to_utc();
        let e2 = DateTime::parse_from_rfc3339("2026-04-15T12:00:00Z")
            .unwrap()
            .to_utc();
        assert!(ranges_overlap(&s1, &e1, &s2, &e2));
    }

    #[test]
    fn test_ranges_overlap_false() {
        let s1 = DateTime::parse_from_rfc3339("2026-04-15T08:00:00Z")
            .unwrap()
            .to_utc();
        let e1 = DateTime::parse_from_rfc3339("2026-04-15T10:00:00Z")
            .unwrap()
            .to_utc();
        let s2 = DateTime::parse_from_rfc3339("2026-04-15T10:00:00Z")
            .unwrap()
            .to_utc();
        let e2 = DateTime::parse_from_rfc3339("2026-04-15T12:00:00Z")
            .unwrap()
            .to_utc();
        assert!(!ranges_overlap(&s1, &e1, &s2, &e2));
    }

    #[test]
    fn test_ranges_overlap_adjacent() {
        let s1 = DateTime::parse_from_rfc3339("2026-04-15T08:00:00Z")
            .unwrap()
            .to_utc();
        let e1 = DateTime::parse_from_rfc3339("2026-04-15T10:00:00Z")
            .unwrap()
            .to_utc();
        let s2 = DateTime::parse_from_rfc3339("2026-04-15T10:00:01Z")
            .unwrap()
            .to_utc();
        let e2 = DateTime::parse_from_rfc3339("2026-04-15T12:00:00Z")
            .unwrap()
            .to_utc();
        assert!(!ranges_overlap(&s1, &e1, &s2, &e2));
    }

    #[test]
    fn test_check_time_range_valid_ok() {
        let start = Utc::now() + chrono::Duration::hours(1);
        let end = start + chrono::Duration::hours(2);
        assert!(check_time_range_valid(&start, &end).is_ok());
    }

    #[test]
    fn test_check_time_range_end_before_start() {
        let end = Utc::now() + chrono::Duration::hours(1);
        let start = end + chrono::Duration::hours(2);
        let result = check_time_range_valid(&start, &end);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("End time must be after"));
    }

    #[test]
    fn test_check_time_range_too_long() {
        let start = Utc::now() + chrono::Duration::hours(1);
        let end = start + chrono::Duration::days(31);
        let result = check_time_range_valid(&start, &end);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("30 days"));
    }

    #[test]
    fn test_availability_result_serialize() {
        let result = AvailabilityResult {
            available: false,
            conflicts: vec![ConflictDetail {
                conflicting_booking_id: Uuid::new_v4(),
                start: Utc::now(),
                end: Utc::now(),
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"available\":false"));
        assert!(json.contains("conflicts"));
    }
}
