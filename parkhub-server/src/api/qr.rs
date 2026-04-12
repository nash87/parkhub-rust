//! QR code parking pass generation.
//!
//! `GET /api/v1/bookings/:id/qr` generates a QR code PNG image encoding
//! booking details (`booking_id`, `user_email`, `lot_name`, start/end timestamps).

use axum::{
    Extension, Json,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use image::Luma;
use qrcode::QrCode;
use serde::Serialize;
use std::io::Cursor;

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

/// JSON payload embedded in the QR code.
#[derive(Debug, Serialize)]
struct QrPassPayload {
    booking_id: String,
    user_email: String,
    lot_name: String,
    start: String,
    end: String,
}

/// Generate a QR code PNG image for a booking.
///
/// The QR content is a JSON object with booking metadata so that
/// scanners/validators can verify the pass offline.
#[utoipa::path(
    get,
    path = "/api/v1/bookings/{id}/qr",
    tag = "Bookings",
    summary = "Generate QR code parking pass",
    description = "Returns a PNG image containing a QR code that encodes booking details (ID, user email, lot name, start/end times). Requires authentication; only the booking owner or an admin may request it. Rate limited to 10 requests per minute per IP.",
    params(
        ("id" = String, Path, description = "Booking UUID")
    ),
    responses(
        (status = 200, description = "QR code PNG image", content_type = "image/png"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — not the booking owner"),
        (status = 404, description = "Booking not found"),
        (status = 500, description = "QR generation failed")
    ),
    security(("bearer_auth" = []))
)]
#[allow(clippy::too_many_lines)]
pub async fn booking_qr_code(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Response {
    let state_guard = state.read().await;

    // Fetch booking
    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("NOT_FOUND", "Booking not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error fetching booking for QR: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "Internal server error",
                )),
            )
                .into_response();
        }
    };

    // Ownership check — only the booking owner or an admin may generate the QR
    let is_admin = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => matches!(
            u.role,
            parkhub_common::UserRole::Admin | parkhub_common::UserRole::SuperAdmin
        ),
        _ => false,
    };

    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("FORBIDDEN", "Access denied")),
        )
            .into_response();
    }

    // Resolve user email
    let user_email = match state_guard.db.get_user(&booking.user_id.to_string()).await {
        Ok(Some(u)) => u.email,
        _ => String::from("unknown"),
    };

    // Resolve lot name
    let lot_name = match state_guard
        .db
        .get_parking_lot(&booking.lot_id.to_string())
        .await
    {
        Ok(Some(lot)) => lot.name,
        _ => String::from("Unknown Lot"),
    };

    drop(state_guard);

    // Build QR payload
    let payload = QrPassPayload {
        booking_id: booking.id.to_string(),
        user_email,
        lot_name,
        start: booking.start_time.to_rfc3339(),
        end: booking.end_time.to_rfc3339(),
    };

    let json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to serialize QR payload: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "QR generation failed",
                )),
            )
                .into_response();
        }
    };

    // Generate QR code
    let code = match QrCode::new(json.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("QR code generation failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "QR generation failed",
                )),
            )
                .into_response();
        }
    };

    // Render to PNG bytes in memory
    let image = code.render::<Luma<u8>>().min_dimensions(300, 300).build();

    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    if let Err(e) = image.write_to(&mut cursor, image::ImageFormat::Png) {
        tracing::error!("PNG encoding failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "SERVER_ERROR",
                "QR generation failed",
            )),
        )
            .into_response();
    }

    // Return PNG with appropriate headers
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "private, max-age=300")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"parking-pass-{}.png\"", booking.id),
        )
        .body(Body::from(png_bytes))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response",
            )
                .into_response()
        })
}

/// JSON payload embedded in the slot QR code.
#[derive(Debug, Serialize)]
struct QrSlotPayload {
    r#type: String,
    slot_id: String,
    lot_id: String,
}

/// `GET /api/v1/lots/{lot_id}/slots/{slot_id}/qr` — QR code PNG for a parking slot.
#[utoipa::path(
    get,
    path = "/api/v1/lots/{lot_id}/slots/{slot_id}/qr",
    tag = "Lots",
    summary = "Generate QR code for a parking slot",
    description = "Returns a PNG image containing a QR code encoding the slot identity. Requires authentication.",
    params(
        ("lot_id" = String, Path, description = "Lot UUID"),
        ("slot_id" = String, Path, description = "Slot UUID")
    ),
    responses(
        (status = 200, description = "QR code PNG image", content_type = "image/png"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Slot not found"),
        (status = 500, description = "QR generation failed")
    ),
    security(("bearer_auth" = []))
)]
pub async fn slot_qr_code(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path((lot_id, slot_id)): Path<(String, String)>,
) -> Response {
    let state_guard = state.read().await;

    // Look up the slot
    let slot = match state_guard.db.get_parking_slot(&slot_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error(
                    "NOT_FOUND",
                    "Parking slot not found",
                )),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error fetching slot for QR: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "Internal server error",
                )),
            )
                .into_response();
        }
    };

    // Verify slot belongs to the given lot
    if slot.lot_id.to_string() != lot_id {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error(
                "NOT_FOUND",
                "Parking slot not found in this lot",
            )),
        )
            .into_response();
    }

    drop(state_guard);

    // Build QR payload
    let payload = QrSlotPayload {
        r#type: "slot".to_string(),
        slot_id: slot.id.to_string(),
        lot_id: slot.lot_id.to_string(),
    };

    let json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to serialize slot QR payload: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "QR generation failed",
                )),
            )
                .into_response();
        }
    };

    // Generate QR code
    let code = match QrCode::new(json.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Slot QR code generation failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "QR generation failed",
                )),
            )
                .into_response();
        }
    };

    // Render to PNG bytes in memory
    let image = code.render::<Luma<u8>>().min_dimensions(300, 300).build();

    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    if let Err(e) = image.write_to(&mut cursor, image::ImageFormat::Png) {
        tracing::error!("PNG encoding failed for slot QR: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "SERVER_ERROR",
                "QR generation failed",
            )),
        )
            .into_response();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "private, max-age=300")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"slot-qr-{}.png\"", slot.id),
        )
        .body(Body::from(png_bytes))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response",
            )
                .into_response()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_payload_serialization() {
        let payload = QrPassPayload {
            booking_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            user_email: "test@example.com".to_string(),
            lot_name: "Main Garage".to_string(),
            start: "2026-03-21T08:00:00+00:00".to_string(),
            end: "2026-03-21T17:00:00+00:00".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("booking_id"));
        assert!(json.contains("550e8400"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("Main Garage"));
    }

    #[test]
    fn test_qr_code_generation() {
        let data = r#"{"booking_id":"abc","user_email":"a@b.com","lot_name":"Lot A","start":"2026-01-01T00:00:00Z","end":"2026-01-01T12:00:00Z"}"#;
        let code = QrCode::new(data.as_bytes()).expect("QR generation should succeed");
        let image = code.render::<Luma<u8>>().min_dimensions(300, 300).build();

        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);
        image
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("PNG encoding should succeed");

        // Verify we got valid PNG bytes (PNG magic: 0x89 P N G)
        assert!(buf.len() > 100, "PNG should have reasonable size");
        assert_eq!(&buf[1..4], b"PNG", "Should be valid PNG header");
    }
}
