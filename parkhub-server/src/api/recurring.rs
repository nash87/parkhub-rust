//! Recurring booking handlers: list, create, delete, update.

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::models::RecurringBooking;
use parkhub_common::{ApiResponse, UserRole};

use super::{AuthUser, SharedState};

/// `GET /api/v1/recurring-bookings` — list user's recurring bookings
#[utoipa::path(
    get,
    path = "/api/v1/recurring-bookings",
    tag = "Bookings",
    summary = "List recurring bookings",
    description = "List the current user's recurring booking patterns.",
    security(("bearer_auth" = []))
)]
pub async fn list_recurring_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<RecurringBooking>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_recurring_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(bookings) => Json(ApiResponse::success(bookings)),
        Err(e) => {
            tracing::error!("Failed to list recurring bookings: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list recurring bookings",
            ))
        }
    }
}

/// Request body for creating a recurring booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateRecurringBookingRequest {
    lot_id: Uuid,
    slot_id: Option<Uuid>,
    days_of_week: Vec<u8>,
    start_date: String,
    end_date: Option<String>,
    start_time: String,
    end_time: String,
    vehicle_plate: Option<String>,
}

/// Reject a recurring rule the expansion job could never turn into bookings.
///
/// The endpoint performed no validation at all: any payload that
/// deserialised was persisted and answered `201 Created` with a fully
/// populated body. The consumer (`jobs::expand_recurring`) then parsed the
/// same fields and silently `warn`-and-continued on anything malformed, so
/// a user saw a series that was created successfully and produced nothing,
/// for ever, with no feedback anywhere they could see.
///
/// Worse than dropping it: `start_time: "22:00"` with `end_time: "06:00"`
/// parses fine and yields `end < start`, which the job turns into bookings
/// with a negative duration.
///
/// The formats mirror what the job actually parses — `%H:%M` for times,
/// `%Y-%m-%d` for dates, and `days_of_week` as 0 = Monday … 6 = Sunday
/// (`chrono::Weekday::num_days_from_monday`).
fn validate_create_request(req: &CreateRecurringBookingRequest) -> Result<(), (&'static str, String)> {
    if req.days_of_week.is_empty() {
        return Err(("VALIDATION_ERROR", "days_of_week must not be empty".into()));
    }
    if let Some(bad) = req.days_of_week.iter().find(|d| **d > 6) {
        return Err((
            "VALIDATION_ERROR",
            format!("days_of_week values must be 0 (Monday) to 6 (Sunday); got {bad}"),
        ));
    }

    let Ok(start_time) = NaiveTime::parse_from_str(&req.start_time, "%H:%M") else {
        return Err((
            "VALIDATION_ERROR",
            format!("start_time must be HH:MM; got '{}'", req.start_time),
        ));
    };
    let Ok(end_time) = NaiveTime::parse_from_str(&req.end_time, "%H:%M") else {
        return Err((
            "VALIDATION_ERROR",
            format!("end_time must be HH:MM; got '{}'", req.end_time),
        ));
    };
    if end_time <= start_time {
        return Err((
            "VALIDATION_ERROR",
            "end_time must be after start_time; overnight series are not supported".into(),
        ));
    }

    let Ok(start_date) = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d") else {
        return Err((
            "VALIDATION_ERROR",
            format!("start_date must be YYYY-MM-DD; got '{}'", req.start_date),
        ));
    };
    if let Some(ref raw_end) = req.end_date {
        let Ok(end_date) = NaiveDate::parse_from_str(raw_end, "%Y-%m-%d") else {
            return Err((
                "VALIDATION_ERROR",
                format!("end_date must be YYYY-MM-DD; got '{raw_end}'"),
            ));
        };
        if end_date < start_date {
            return Err(("VALIDATION_ERROR", "end_date must not precede start_date".into()));
        }
    }

    Ok(())
}

/// `POST /api/v1/recurring-bookings` — create a recurring booking
#[utoipa::path(
    post,
    path = "/api/v1/recurring-bookings",
    tag = "Bookings",
    summary = "Create recurring booking",
    description = "Create a new recurring booking pattern (e.g. every Tuesday 8-17).",
    security(("bearer_auth" = []))
)]
pub async fn create_recurring_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateRecurringBookingRequest>,
) -> (StatusCode, Json<ApiResponse<RecurringBooking>>) {
    if let Err((code, message)) = validate_create_request(&req) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(code, message)),
        );
    }

    let state_guard = state.read().await;

    let booking = RecurringBooking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        days_of_week: req.days_of_week,
        start_date: req.start_date,
        end_date: req.end_date,
        start_time: req.start_time,
        end_time: req.end_time,
        vehicle_plate: req.vehicle_plate,
        active: true,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_recurring_booking(&booking).await {
        tracing::error!("Failed to save recurring booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create recurring booking",
            )),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

/// `DELETE /api/v1/recurring-bookings/{id}` — delete recurring booking (verify ownership)
#[utoipa::path(
    delete,
    path = "/api/v1/recurring-bookings/{id}",
    tag = "Bookings",
    summary = "Delete recurring booking",
    description = "Delete a recurring booking pattern. Verifies ownership.",
    security(("bearer_auth" = []))
)]
pub async fn delete_recurring_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Check ownership via listing user's recurring bookings
    let user_bookings = state_guard
        .db
        .list_recurring_bookings_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let Ok(id_uuid) = Uuid::parse_str(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "Invalid ID format")),
        );
    };

    if !user_bookings.iter().any(|b| b.id == id_uuid) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    match state_guard.db.delete_recurring_booking(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NOT_FOUND",
                "Recurring booking not found",
            )),
        ),
        Err(e) => {
            tracing::error!("Failed to delete recurring booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete recurring booking",
                )),
            )
        }
    }
}

/// Request body for updating a recurring booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateRecurringBookingRequest {
    pub days_of_week: Option<Vec<u8>>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// `PUT /api/v1/recurring-bookings/{id}` — update a recurring booking pattern
#[utoipa::path(
    put,
    path = "/api/v1/recurring-bookings/{id}",
    tag = "Bookings",
    summary = "Update a recurring booking",
    description = "Update days_of_week, start_date, or end_date of a recurring booking. Only the owner or an admin may update.",
    security(("bearer_auth" = []))
)]
pub async fn update_recurring_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRecurringBookingRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let Ok(id_uuid) = Uuid::parse_str(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "Invalid ID format")),
        );
    };

    let state_guard = state.read().await;

    // Fetch caller to check admin status
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };
    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;

    // Try ownership lookup first
    let user_bookings = state_guard
        .db
        .list_recurring_bookings_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let Some(mut booking) = user_bookings.into_iter().find(|b| b.id == id_uuid) else {
        if !is_admin {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NOT_FOUND",
                "Recurring booking not found",
            )),
        );
    };

    if let Some(days) = req.days_of_week {
        booking.days_of_week = days;
    }
    if let Some(start_date) = req.start_date {
        booking.start_date = start_date;
    }
    if let Some(end_date) = req.end_date {
        booking.end_date = Some(end_date);
    }

    if let Err(e) = state_guard.db.save_recurring_booking(&booking).await {
        tracing::error!("Failed to update recurring booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update recurring booking",
            )),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "id": booking.id.to_string(),
            "days_of_week": booking.days_of_week,
            "start_date": booking.start_date,
            "end_date": booking.end_date,
            "note": "Future expanded bookings are not re-generated automatically. Trigger re-expansion separately if needed."
        }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_recurring_booking_request_full() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "slot_id":"660e8400-e29b-41d4-a716-446655440001",
            "days_of_week":[1,3,5],
            "start_date":"2026-04-01",
            "end_date":"2026-06-30",
            "start_time":"08:00",
            "end_time":"17:00",
            "vehicle_plate":"B-AB 1234"
        }"#;
        let req: CreateRecurringBookingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.days_of_week, vec![1, 3, 5]);
        assert_eq!(req.start_date, "2026-04-01");
        assert_eq!(req.end_date.as_deref(), Some("2026-06-30"));
        assert_eq!(req.vehicle_plate.as_deref(), Some("B-AB 1234"));
    }

    #[test]
    fn test_create_recurring_booking_request_minimal() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "days_of_week":[1],
            "start_date":"2026-04-01",
            "start_time":"09:00",
            "end_time":"18:00"
        }"#;
        let req: CreateRecurringBookingRequest = serde_json::from_str(json).unwrap();
        assert!(req.slot_id.is_none());
        assert!(req.end_date.is_none());
        assert!(req.vehicle_plate.is_none());
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    fn valid() -> CreateRecurringBookingRequest {
        CreateRecurringBookingRequest {
            lot_id: Uuid::nil(),
            slot_id: None,
            days_of_week: vec![0, 1, 2, 3, 4],
            start_date: "2026-09-01".into(),
            end_date: Some("2026-12-01".into()),
            start_time: "09:00".into(),
            end_time: "17:00".into(),
            vehicle_plate: None,
        }
    }

    #[test]
    fn a_well_formed_rule_is_accepted() {
        assert!(validate_create_request(&valid()).is_ok());
    }

    #[test]
    fn an_open_ended_series_is_allowed() {
        let mut req = valid();
        req.end_date = None;
        assert!(validate_create_request(&req).is_ok());
    }

    #[test]
    fn days_of_week_must_not_be_empty() {
        let mut req = valid();
        req.days_of_week = vec![];
        assert!(validate_create_request(&req).is_err());
    }

    /// 0 = Monday … 6 = Sunday. A 7 is the classic off-by-one from an
    /// ISO-weekday API and would silently match no day at all.
    #[test]
    fn days_of_week_must_be_in_range() {
        let mut req = valid();
        req.days_of_week = vec![0, 7];
        assert!(validate_create_request(&req).is_err());
    }

    #[test]
    fn times_must_parse() {
        let mut req = valid();
        req.start_time = "9am".into();
        assert!(validate_create_request(&req).is_err());

        let mut req = valid();
        req.end_time = "".into();
        assert!(validate_create_request(&req).is_err());
    }

    /// The damaging one: this parses cleanly and the expansion job turns it
    /// into bookings whose end precedes their start.
    #[test]
    fn an_overnight_window_is_rejected_rather_than_expanded_backwards() {
        let mut req = valid();
        req.start_time = "22:00".into();
        req.end_time = "06:00".into();
        assert!(validate_create_request(&req).is_err());
    }

    #[test]
    fn a_zero_length_window_is_rejected() {
        let mut req = valid();
        req.end_time = req.start_time.clone();
        assert!(validate_create_request(&req).is_err());
    }

    #[test]
    fn dates_must_parse_and_be_ordered() {
        let mut req = valid();
        req.start_date = "01/09/2026".into();
        assert!(validate_create_request(&req).is_err());

        let mut req = valid();
        req.end_date = Some("2026-08-01".into());
        assert!(validate_create_request(&req).is_err(), "end_date before start_date was accepted");
    }
}
