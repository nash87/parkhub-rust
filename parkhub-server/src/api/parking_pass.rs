//! Digital Parking Pass / QR Badge handlers.
//!
//! Generate digital passes with QR codes for quick entry verification.
//!
//! - `GET /api/v1/bookings/:id/pass` — generate digital pass with QR code
//! - `GET /api/v1/pass/verify/:code` — public verification endpoint
//! - `GET /api/v1/me/passes` — list all active passes for current user

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

/// A digital parking pass
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ParkingPass {
    pub id: Uuid,
    pub booking_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub lot_name: String,
    pub slot_number: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub verification_code: String,
    pub qr_data: String,
    pub status: PassStatus,
    pub created_at: DateTime<Utc>,
}

/// Pass status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PassStatus {
    Active,
    Expired,
    Revoked,
    Used,
}

/// Verification response (public)
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerifyPassResponse {
    pub valid: bool,
    pub pass: Option<PassSummary>,
    pub message: String,
}

/// Summary shown to verifiers (no sensitive data)
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PassSummary {
    pub user_name: String,
    pub lot_name: String,
    pub slot_number: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub status: PassStatus,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a verification code from booking ID
fn generate_verification_code(booking_id: &Uuid) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(booking_id.as_bytes());
    hasher.update(b"parkhub-pass-v1");
    let hash = hasher.finalize();
    // Take first 8 bytes as hex = 16 chars
    hex::encode(&hash[..8])
}

/// Generate QR code data as base64-encoded PNG
fn generate_qr_base64(data: &str) -> String {
    use base64::Engine;
    use image::Luma;
    use qrcode::QrCode;

    let code = match QrCode::new(data.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let img = code.render::<Luma<u8>>().quiet_zone(true).build();

    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    if image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::L8,
    )
    .is_err()
    {
        return String::new();
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    format!("data:image/png;base64,{b64}")
}

/// `GET /api/v1/bookings/:id/pass` — generate digital pass with QR code
#[utoipa::path(get, path = "/api/v1/bookings/{id}/pass", tag = "Parking Pass",
    summary = "Generate parking pass",
    description = "Generate a digital parking pass with QR code for a booking.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Pass generated"),
        (status = 404, description = "Booking not found"),
        (status = 403, description = "Not your booking"),
    )
)]
pub async fn get_booking_pass(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(booking_id): Path<Uuid>,
) -> (StatusCode, Json<ApiResponse<ParkingPass>>) {
    let state_guard = state.read().await;

    // Get booking
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

    // Get lot and slot info
    let lot_name = if let Ok(Some(lot)) = state_guard
        .db
        .get_parking_lot(&booking.lot_id.to_string())
        .await
    {
        lot.name
    } else {
        "Unknown Lot".to_string()
    };

    let slot_number = if let Ok(Some(slot)) = state_guard
        .db
        .get_parking_slot(&booking.slot_id.to_string())
        .await
    {
        slot.slot_number.to_string()
    } else {
        "?".to_string()
    };

    // Get user name
    let user_name = if let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        user.name
    } else {
        "Unknown".to_string()
    };

    // Generate verification code and QR
    let verification_code = generate_verification_code(&booking_id);
    let verify_url = format!("/api/v1/pass/verify/{}", verification_code);
    let qr_data = generate_qr_base64(&verify_url);

    // Determine status
    let status = if booking.status == BookingStatus::Cancelled {
        PassStatus::Revoked
    } else if booking.end_time < Utc::now() {
        PassStatus::Expired
    } else if booking.check_in_time.is_some() {
        PassStatus::Used
    } else {
        PassStatus::Active
    };

    let pass = ParkingPass {
        id: Uuid::new_v4(),
        booking_id,
        user_id: auth_user.user_id,
        user_name,
        lot_name,
        slot_number,
        valid_from: booking.start_time,
        valid_until: booking.end_time,
        verification_code,
        qr_data,
        status,
        created_at: Utc::now(),
    };

    (StatusCode::OK, Json(ApiResponse::success(pass)))
}

/// `GET /api/v1/pass/verify/:code` — public verification endpoint (no auth required)
#[utoipa::path(get, path = "/api/v1/pass/verify/{code}", tag = "Parking Pass",
    summary = "Verify parking pass",
    description = "Public endpoint to verify a parking pass QR code.",
    responses(
        (status = 200, description = "Verification result"),
    )
)]
pub async fn verify_pass(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> Json<ApiResponse<VerifyPassResponse>> {
    let state_guard = state.read().await;

    // Search all bookings for a matching verification code
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let matching = bookings
        .iter()
        .find(|b| generate_verification_code(&b.id) == code);

    match matching {
        Some(booking) => {
            let lot_name = if let Ok(Some(lot)) = state_guard
                .db
                .get_parking_lot(&booking.lot_id.to_string())
                .await
            {
                lot.name
            } else {
                "Unknown Lot".to_string()
            };

            let slot_number = if let Ok(Some(slot)) = state_guard
                .db
                .get_parking_slot(&booking.slot_id.to_string())
                .await
            {
                slot.slot_number.to_string()
            } else {
                "?".to_string()
            };

            let user_name = if let Ok(Some(user)) =
                state_guard.db.get_user(&booking.user_id.to_string()).await
            {
                user.name
            } else {
                "Unknown".to_string()
            };

            let status = if booking.status == BookingStatus::Cancelled {
                PassStatus::Revoked
            } else if booking.end_time < Utc::now() {
                PassStatus::Expired
            } else if booking.check_in_time.is_some() {
                PassStatus::Used
            } else {
                PassStatus::Active
            };

            let valid = status == PassStatus::Active || status == PassStatus::Used;

            Json(ApiResponse::success(VerifyPassResponse {
                valid,
                pass: Some(PassSummary {
                    user_name,
                    lot_name,
                    slot_number,
                    valid_from: booking.start_time,
                    valid_until: booking.end_time,
                    status,
                }),
                message: if valid {
                    "Pass is valid".to_string()
                } else {
                    "Pass is not valid".to_string()
                },
            }))
        }
        None => Json(ApiResponse::success(VerifyPassResponse {
            valid: false,
            pass: None,
            message: "No matching pass found".to_string(),
        })),
    }
}

/// `GET /api/v1/me/passes` — list all active passes for current user
#[utoipa::path(get, path = "/api/v1/me/passes", tag = "Parking Pass",
    summary = "List my passes",
    description = "List all parking passes for the current user's active bookings.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of passes"),
    )
)]
pub async fn list_my_passes(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<ParkingPass>>> {
    let state_guard = state.read().await;

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let user_name = if let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        user.name
    } else {
        "Unknown".to_string()
    };

    let mut passes = Vec::new();

    for booking in &bookings {
        if booking.user_id != auth_user.user_id {
            continue;
        }
        if booking.status == BookingStatus::Cancelled {
            continue;
        }
        if booking.end_time < Utc::now() {
            continue;
        }

        let lot_name = if let Ok(Some(lot)) = state_guard
            .db
            .get_parking_lot(&booking.lot_id.to_string())
            .await
        {
            lot.name
        } else {
            "Unknown Lot".to_string()
        };

        let slot_number = if let Ok(Some(slot)) = state_guard
            .db
            .get_parking_slot(&booking.slot_id.to_string())
            .await
        {
            slot.slot_number.to_string()
        } else {
            "?".to_string()
        };

        let verification_code = generate_verification_code(&booking.id);
        let verify_url = format!("/api/v1/pass/verify/{}", verification_code);
        let qr_data = generate_qr_base64(&verify_url);

        let status = if booking.check_in_time.is_some() {
            PassStatus::Used
        } else {
            PassStatus::Active
        };

        passes.push(ParkingPass {
            id: Uuid::new_v4(),
            booking_id: booking.id,
            user_id: auth_user.user_id,
            user_name: user_name.clone(),
            lot_name,
            slot_number,
            valid_from: booking.start_time,
            valid_until: booking.end_time,
            verification_code,
            qr_data,
            status,
            created_at: Utc::now(),
        });
    }

    Json(ApiResponse::success(passes))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_verification_code_deterministic() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let code1 = generate_verification_code(&id);
        let code2 = generate_verification_code(&id);
        assert_eq!(code1, code2);
        assert_eq!(code1.len(), 16); // 8 bytes hex
    }

    #[test]
    fn test_generate_verification_code_unique() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        assert_ne!(
            generate_verification_code(&id1),
            generate_verification_code(&id2)
        );
    }

    #[test]
    fn test_parking_pass_serialize() {
        let pass = ParkingPass {
            id: Uuid::nil(),
            booking_id: Uuid::nil(),
            user_id: Uuid::nil(),
            user_name: "Alice".to_string(),
            lot_name: "Garage A".to_string(),
            slot_number: "42".to_string(),
            valid_from: Utc::now(),
            valid_until: Utc::now(),
            verification_code: "abc123".to_string(),
            qr_data: "data:image/png;base64,test".to_string(),
            status: PassStatus::Active,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&pass).unwrap();
        assert!(json.contains("\"user_name\":\"Alice\""));
        assert!(json.contains("\"lot_name\":\"Garage A\""));
        assert!(json.contains("\"slot_number\":\"42\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn test_pass_status_serde() {
        assert_eq!(
            serde_json::to_string(&PassStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&PassStatus::Expired).unwrap(),
            "\"expired\""
        );
        assert_eq!(
            serde_json::to_string(&PassStatus::Revoked).unwrap(),
            "\"revoked\""
        );
        assert_eq!(
            serde_json::to_string(&PassStatus::Used).unwrap(),
            "\"used\""
        );
    }

    #[test]
    fn test_verify_pass_response_valid() {
        let resp = VerifyPassResponse {
            valid: true,
            pass: Some(PassSummary {
                user_name: "Alice".to_string(),
                lot_name: "Garage A".to_string(),
                slot_number: "42".to_string(),
                valid_from: Utc::now(),
                valid_until: Utc::now(),
                status: PassStatus::Active,
            }),
            message: "Pass is valid".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"user_name\":\"Alice\""));
    }

    #[test]
    fn test_verify_pass_response_invalid() {
        let resp = VerifyPassResponse {
            valid: false,
            pass: None,
            message: "No matching pass found".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("\"pass\":null"));
    }

    #[test]
    fn test_pass_summary_serialize() {
        let summary = PassSummary {
            user_name: "Bob".to_string(),
            lot_name: "Lot B".to_string(),
            slot_number: "7".to_string(),
            valid_from: Utc::now(),
            valid_until: Utc::now(),
            status: PassStatus::Used,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"user_name\":\"Bob\""));
        assert!(json.contains("\"status\":\"used\""));
    }

    #[test]
    fn test_generate_qr_base64() {
        let qr = generate_qr_base64("https://example.com/pass/verify/abc123");
        assert!(qr.starts_with("data:image/png;base64,"));
        assert!(qr.len() > 50);
    }

    #[test]
    fn test_pass_status_equality() {
        assert_eq!(PassStatus::Active, PassStatus::Active);
        assert_ne!(PassStatus::Active, PassStatus::Expired);
    }
}
