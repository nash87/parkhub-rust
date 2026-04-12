//! Guest booking handlers: create, admin list, admin cancel.

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::models::GuestBooking;
use parkhub_common::{ApiResponse, BookingStatus};

use super::settings::read_admin_setting;
use super::{AuthUser, SharedState, check_admin};

/// Request body for creating a guest booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateGuestBookingRequest {
    pub lot_id: Uuid,
    pub slot_id: Uuid,
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: chrono::DateTime<Utc>,
    pub guest_name: String,
    pub guest_email: Option<String>,
}

/// Generate an 8-character random alphanumeric guest code
pub fn generate_guest_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// `POST /api/v1/bookings/guest` — create a guest booking
#[utoipa::path(
    post,
    path = "/api/v1/bookings/guest",
    tag = "Bookings",
    summary = "Create guest booking",
    description = "Create a visitor parking booking with a guest code.",
    security(("bearer_auth" = []))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, guest_name = %req.guest_name))]
pub async fn create_guest_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateGuestBookingRequest>,
) -> (StatusCode, Json<ApiResponse<GuestBooking>>) {
    let state_guard = state.read().await;

    // Check allow_guest_bookings setting
    let allowed = read_admin_setting(&state_guard.db, "allow_guest_bookings").await;
    if allowed != "true" {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "GUEST_BOOKINGS_DISABLED",
                "Guest bookings are not enabled",
            )),
        );
    }

    let guest_booking = GuestBooking {
        id: Uuid::new_v4(),
        created_by: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        guest_name: req.guest_name,
        guest_email: req.guest_email,
        guest_code: generate_guest_code(),
        start_time: req.start_time,
        end_time: req.end_time,
        vehicle_plate: None,
        status: BookingStatus::Confirmed,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_guest_booking(&guest_booking).await {
        tracing::error!("Failed to save guest booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create guest booking",
            )),
        );
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(guest_booking)),
    )
}

/// `GET /api/v1/admin/guest-bookings` — admin: list all guest bookings
#[utoipa::path(
    get,
    path = "/api/v1/admin/guest-bookings",
    tag = "Admin",
    summary = "List guest bookings",
    description = "List all guest bookings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_list_guest_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<GuestBooking>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_guest_bookings().await {
        Ok(bookings) => (StatusCode::OK, Json(ApiResponse::success(bookings))),
        Err(e) => {
            tracing::error!("Failed to list guest bookings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list guest bookings",
                )),
            )
        }
    }
}

/// `PATCH /api/v1/admin/guest-bookings/{id}/cancel` — admin: cancel a guest booking
#[utoipa::path(
    patch,
    path = "/api/v1/admin/guest-bookings/{id}/cancel",
    tag = "Admin",
    summary = "Cancel guest booking",
    description = "Cancel a guest booking by ID. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_cancel_guest_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<GuestBooking>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut booking = match state_guard.db.get_guest_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Guest booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    booking.status = BookingStatus::Cancelled;

    if let Err(e) = state_guard.db.save_guest_booking(&booking).await {
        tracing::error!("Failed to cancel guest booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel guest booking",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_guest_code_length() {
        let code = generate_guest_code();
        assert_eq!(code.len(), 8);
    }

    #[test]
    fn test_generate_guest_code_charset() {
        let valid_chars: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        for _ in 0..20 {
            let code = generate_guest_code();
            for c in code.chars() {
                assert!(
                    valid_chars.contains(c),
                    "Invalid char '{}' in guest code",
                    c
                );
            }
        }
    }

    #[test]
    fn test_generate_guest_code_uniqueness() {
        let codes: Vec<String> = (0..50).map(|_| generate_guest_code()).collect();
        let unique: std::collections::HashSet<&String> = codes.iter().collect();
        assert!(unique.len() > 45);
    }

    #[test]
    fn test_create_guest_booking_request() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "slot_id":"660e8400-e29b-41d4-a716-446655440001",
            "start_time":"2026-04-01T08:00:00Z",
            "end_time":"2026-04-01T17:00:00Z",
            "guest_name":"Visitor One",
            "guest_email":"visitor@example.com"
        }"#;
        let req: CreateGuestBookingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.guest_name, "Visitor One");
        assert_eq!(req.guest_email.as_deref(), Some("visitor@example.com"));
    }

    #[test]
    fn test_create_guest_booking_request_no_email() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "slot_id":"660e8400-e29b-41d4-a716-446655440001",
            "start_time":"2026-04-01T08:00:00Z",
            "end_time":"2026-04-01T17:00:00Z",
            "guest_name":"Walk-in"
        }"#;
        let req: CreateGuestBookingRequest = serde_json::from_str(json).unwrap();
        assert!(req.guest_email.is_none());
    }
}
