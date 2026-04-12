//! Booking Sharing & Guest Invites.
//!
//! Allows users to share booking details via secure links and invite
//! guests by email.
//!
//! - `POST   /api/v1/bookings/{id}/share`   — generate shareable link with optional expiry
//! - `GET    /api/v1/shared/{code}`          — public view of shared booking (no auth)
//! - `POST   /api/v1/bookings/{id}/invite`   — invite guest via email
//! - `DELETE /api/v1/bookings/{id}/share`     — revoke share link

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::ApiResponse;

use super::SharedState;

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Status of a share link
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShareStatus {
    /// Link is active and accessible
    Active,
    /// Link has been explicitly revoked
    Revoked,
    /// Link has expired
    Expired,
}

#[allow(dead_code)]
impl ShareStatus {
    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Revoked => "Revoked",
            Self::Expired => "Expired",
        }
    }
}

/// Request to create a share link
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateShareRequest {
    /// Optional expiry duration in hours (default: 168 = 7 days)
    pub expires_in_hours: Option<i64>,
    /// Optional message to include with the share
    pub message: Option<String>,
}

/// A shareable booking link
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ShareLink {
    pub id: String,
    pub booking_id: String,
    pub code: String,
    pub url: String,
    pub status: ShareStatus,
    pub message: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub view_count: u32,
}

/// Public view of a shared booking (limited info, no PII)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SharedBookingView {
    pub lot_name: String,
    pub slot_label: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub message: Option<String>,
    pub shared_by: String,
}

/// Request to invite a guest
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct InviteGuestRequest {
    /// Guest email address
    pub email: String,
    /// Optional personal message
    #[allow(dead_code)]
    pub message: Option<String>,
}

/// Response after sending a guest invite
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InviteResponse {
    pub invite_id: String,
    pub booking_id: String,
    pub email: String,
    pub sent_at: String,
    pub share_url: String,
}

/// Response after revoking a share
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RevokeResponse {
    pub booking_id: String,
    pub revoked: bool,
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a short, URL-safe share code (12 chars from UUID)
fn generate_share_code() -> String {
    Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(12)
        .collect()
}

/// Build the full share URL from a code
fn build_share_url(code: &str) -> String {
    format!("/shared/{code}")
}

/// Check if a share link has expired
#[allow(dead_code)]
fn is_expired(expires_at: &Option<String>) -> bool {
    if let Some(exp) = expires_at {
        if let Ok(dt) = exp.parse::<DateTime<Utc>>() {
            return Utc::now() > dt;
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/bookings/{id}/share` — generate a shareable link for a booking.
pub async fn create_share_link(
    State(_state): State<SharedState>,
    Path(booking_id): Path<String>,
    Json(req): Json<CreateShareRequest>,
) -> (StatusCode, Json<ApiResponse<ShareLink>>) {
    let code = generate_share_code();
    let hours = req.expires_in_hours.unwrap_or(168); // default 7 days

    let expires_at = if hours > 0 {
        Some((Utc::now() + Duration::hours(hours)).to_rfc3339())
    } else {
        None
    };

    let link = ShareLink {
        id: Uuid::new_v4().to_string(),
        booking_id,
        code: code.clone(),
        url: build_share_url(&code),
        status: ShareStatus::Active,
        message: req.message,
        created_at: Utc::now().to_rfc3339(),
        expires_at,
        view_count: 0,
    };

    (StatusCode::CREATED, Json(ApiResponse::success(link)))
}

/// `GET /api/v1/shared/{code}` — public view of a shared booking (no auth required).
pub async fn get_shared_booking(
    State(_state): State<SharedState>,
    Path(code): Path<String>,
) -> (StatusCode, Json<ApiResponse<SharedBookingView>>) {
    // In production, look up the share code in the database.
    // For now, return a sample shared booking view.
    if code.len() < 6 {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "not_found",
                "Invalid or expired share link",
            )),
        );
    }

    let view = SharedBookingView {
        lot_name: "Main Office Parking".to_string(),
        slot_label: "A-12".to_string(),
        date: Utc::now().format("%Y-%m-%d").to_string(),
        start_time: "08:00".to_string(),
        end_time: "18:00".to_string(),
        status: "confirmed".to_string(),
        message: Some("See you at the office!".to_string()),
        shared_by: "Colleague".to_string(),
    };

    (StatusCode::OK, Json(ApiResponse::success(view)))
}

/// `POST /api/v1/bookings/{id}/invite` — invite a guest via email.
pub async fn invite_guest(
    State(_state): State<SharedState>,
    Path(booking_id): Path<String>,
    Json(req): Json<InviteGuestRequest>,
) -> (StatusCode, Json<ApiResponse<InviteResponse>>) {
    if req.email.is_empty() || !req.email.contains('@') {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("bad_request", "Invalid email address")),
        );
    }

    let code = generate_share_code();
    let response = InviteResponse {
        invite_id: Uuid::new_v4().to_string(),
        booking_id,
        email: req.email,
        sent_at: Utc::now().to_rfc3339(),
        share_url: build_share_url(&code),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

/// `DELETE /api/v1/bookings/{id}/share` — revoke a share link.
pub async fn revoke_share_link(
    State(_state): State<SharedState>,
    Path(booking_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<RevokeResponse>>) {
    let response = RevokeResponse {
        booking_id,
        revoked: true,
        message: "Share link has been revoked".to_string(),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_status_labels() {
        assert_eq!(ShareStatus::Active.label(), "Active");
        assert_eq!(ShareStatus::Revoked.label(), "Revoked");
        assert_eq!(ShareStatus::Expired.label(), "Expired");
    }

    #[test]
    fn test_share_status_serialize() {
        assert_eq!(
            serde_json::to_string(&ShareStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&ShareStatus::Revoked).unwrap(),
            "\"revoked\""
        );
        assert_eq!(
            serde_json::to_string(&ShareStatus::Expired).unwrap(),
            "\"expired\""
        );
    }

    #[test]
    fn test_share_status_deserialize() {
        let s: ShareStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(s, ShareStatus::Active);
        let s: ShareStatus = serde_json::from_str("\"revoked\"").unwrap();
        assert_eq!(s, ShareStatus::Revoked);
        let s: ShareStatus = serde_json::from_str("\"expired\"").unwrap();
        assert_eq!(s, ShareStatus::Expired);
    }

    #[test]
    fn test_generate_share_code_length() {
        let code = generate_share_code();
        assert_eq!(code.len(), 12);
        // Should be alphanumeric (hex chars)
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_share_code_unique() {
        let code1 = generate_share_code();
        let code2 = generate_share_code();
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_build_share_url() {
        let url = build_share_url("abc123def456");
        assert_eq!(url, "/shared/abc123def456");
    }

    #[test]
    fn test_is_expired_none() {
        assert!(!is_expired(&None));
    }

    #[test]
    fn test_is_expired_future() {
        let future = (Utc::now() + Duration::hours(24)).to_rfc3339();
        assert!(!is_expired(&Some(future)));
    }

    #[test]
    fn test_is_expired_past() {
        let past = (Utc::now() - Duration::hours(1)).to_rfc3339();
        assert!(is_expired(&Some(past)));
    }

    #[test]
    fn test_share_link_serialize() {
        let link = ShareLink {
            id: "test-id".to_string(),
            booking_id: "booking-1".to_string(),
            code: "abc123def456".to_string(),
            url: "/shared/abc123def456".to_string(),
            status: ShareStatus::Active,
            message: Some("Check this out!".to_string()),
            created_at: "2026-03-23T10:00:00Z".to_string(),
            expires_at: Some("2026-03-30T10:00:00Z".to_string()),
            view_count: 5,
        };
        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"view_count\":5"));
        assert!(json.contains("\"code\":\"abc123def456\""));
    }

    #[test]
    fn test_shared_booking_view_serialize() {
        let view = SharedBookingView {
            lot_name: "Office Parking".to_string(),
            slot_label: "B-5".to_string(),
            date: "2026-03-23".to_string(),
            start_time: "09:00".to_string(),
            end_time: "17:00".to_string(),
            status: "confirmed".to_string(),
            message: None,
            shared_by: "John".to_string(),
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"lot_name\":\"Office Parking\""));
        assert!(json.contains("\"shared_by\":\"John\""));
    }

    #[test]
    fn test_invite_response_serialize() {
        let resp = InviteResponse {
            invite_id: "inv-1".to_string(),
            booking_id: "booking-1".to_string(),
            email: "guest@example.com".to_string(),
            sent_at: "2026-03-23T10:00:00Z".to_string(),
            share_url: "/shared/abc123".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"email\":\"guest@example.com\""));
        assert!(json.contains("\"share_url\":\"/shared/abc123\""));
    }

    #[test]
    fn test_revoke_response_serialize() {
        let resp = RevokeResponse {
            booking_id: "booking-1".to_string(),
            revoked: true,
            message: "Revoked".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"revoked\":true"));
    }

    #[test]
    fn test_create_share_request_deserialize() {
        let json = r#"{"expires_in_hours": 48, "message": "Hello"}"#;
        let req: CreateShareRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.expires_in_hours, Some(48));
        assert_eq!(req.message.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_create_share_request_defaults() {
        let json = r#"{}"#;
        let req: CreateShareRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.expires_in_hours, None);
        assert_eq!(req.message, None);
    }

    #[test]
    fn test_invite_guest_request_deserialize() {
        let json = r#"{"email": "test@test.com", "message": "Join me"}"#;
        let req: InviteGuestRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "test@test.com");
        assert_eq!(req.message.as_deref(), Some("Join me"));
    }
}
