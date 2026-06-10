//! No-show release config and waitlist claim offer endpoints.
//!
//! Implements P1-1 (per-lot check-in deadline / no-show auto-release) and
//! P1-2 (waitlist auto-promotion with timed claim offers).
//!
//! # AI-Act compliance
//! Promotion order is **strict FIFO by `WaitlistEntry::created_at`**. No
//! algorithmic scoring or priority reordering is applied. `list_waitlist_by_lot`
//! already sorts ascending by `created_at` — no further sorting is done here.
//! This is documented here so it is machine-auditable.
//!
//! # Settings keys
//! - `lot_noshow_deadline:{lot_id}` — minutes after booking start before a
//!   no-show release fires (0 = disabled for this lot; default 30).
//! - `lot_claim_window:{lot_id}` — minutes the promoted user has to claim the
//!   slot before the offer passes to the next entry (default 15).

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{
    ApiResponse, Booking, BookingPricing, BookingStatus, FuelType, PaymentStatus, SlotStatus,
    Vehicle, VehicleType,
    models::{Notification, NotificationType, WaitlistEntry, WaitlistStatus},
};

use crate::AppState;
use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState, check_admin};

// ─────────────────────────────────────────────────────────────────────────────
// Settings key helpers (pub so jobs.rs can call lot_deadline_minutes /
// lot_claim_window_minutes without re-implementing the key format)
// ─────────────────────────────────────────────────────────────────────────────

/// Default minutes after booking start before a no-show release fires.
pub const DEFAULT_DEADLINE_MINUTES: i64 = 30;
/// Default minutes a promoted user has to claim their offer.
pub const DEFAULT_CLAIM_WINDOW_MINUTES: i64 = 15;

/// Settings key for per-lot no-show deadline.
pub fn lot_deadline_key(lot_id: &str) -> String {
    format!("lot_noshow_deadline:{lot_id}")
}

/// Settings key for per-lot claim window.
pub fn lot_claim_window_key(lot_id: &str) -> String {
    format!("lot_claim_window:{lot_id}")
}

/// Read per-lot deadline in minutes (0 = disabled; default `DEFAULT_DEADLINE_MINUTES`).
pub async fn lot_deadline_minutes(state: &AppState, lot_id: &str) -> i64 {
    state
        .db
        .get_setting(&lot_deadline_key(lot_id))
        .await
        .unwrap_or(None)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_DEADLINE_MINUTES)
}

/// Read per-lot claim window in minutes (default `DEFAULT_CLAIM_WINDOW_MINUTES`).
pub async fn lot_claim_window_minutes(state: &AppState, lot_id: &str) -> i64 {
    state
        .db
        .get_setting(&lot_claim_window_key(lot_id))
        .await
        .unwrap_or(None)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_CLAIM_WINDOW_MINUTES)
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-lot config types
// ─────────────────────────────────────────────────────────────────────────────

/// Per-lot no-show and claim-window configuration.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LotNoshowConfig {
    /// Minutes after booking start that a missing check-in triggers release.
    /// `0` disables auto-release for this lot.
    pub check_in_deadline_minutes: i64,
    /// Minutes the promoted waitlist user has to claim their offer before it
    /// passes to the next entry.
    pub claim_window_minutes: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-lot config API
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/{id}/noshow-config` — read per-lot no-show configuration
#[utoipa::path(
    get, path = "/api/v1/lots/{id}/noshow-config", tag = "Waitlist",
    summary = "Get per-lot no-show config",
    description = "Returns per-lot check-in deadline and claim window minutes. \
                   `check_in_deadline_minutes = 0` means auto-release is disabled for this lot.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Config values", body = LotNoshowConfig),
        (status = 404, description = "Lot not found"),
    )
)]
pub async fn get_lot_noshow_config(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<LotNoshowConfig>>) {
    let state_guard = state.read().await;
    if state_guard
        .db
        .get_parking_lot(&lot_id)
        .await
        .unwrap_or(None)
        .is_none()
    {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
        );
    }
    let config = LotNoshowConfig {
        check_in_deadline_minutes: lot_deadline_minutes(&state_guard, &lot_id).await,
        claim_window_minutes: lot_claim_window_minutes(&state_guard, &lot_id).await,
    };
    (StatusCode::OK, Json(ApiResponse::success(config)))
}

/// Request body for updating per-lot no-show config
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateLotNoshowConfigRequest {
    /// Minutes after booking start before auto-release. `0` = disabled for this lot.
    pub check_in_deadline_minutes: i64,
    /// Minutes the promoted user has to claim. Must be ≥ 1.
    pub claim_window_minutes: i64,
}

/// `PUT /api/v1/lots/{id}/noshow-config` — update per-lot no-show configuration (admin only)
#[utoipa::path(
    put, path = "/api/v1/lots/{id}/noshow-config", tag = "Waitlist",
    summary = "Update per-lot no-show config",
    description = "Admin-only. Sets per-lot check-in deadline and claim window.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Parking lot ID")),
    request_body = UpdateLotNoshowConfigRequest,
    responses(
        (status = 200, description = "Updated config", body = LotNoshowConfig),
        (status = 400, description = "Invalid values"),
        (status = 403, description = "Forbidden — admin only"),
        (status = 404, description = "Lot not found"),
    )
)]
pub async fn update_lot_noshow_config(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
    Json(req): Json<UpdateLotNoshowConfigRequest>,
) -> (StatusCode, Json<ApiResponse<LotNoshowConfig>>) {
    let state_guard = state.read().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if state_guard
        .db
        .get_parking_lot(&lot_id)
        .await
        .unwrap_or(None)
        .is_none()
    {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Lot not found")),
        );
    }

    if req.check_in_deadline_minutes < 0 || req.claim_window_minutes < 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "check_in_deadline_minutes must be >= 0; claim_window_minutes must be >= 1",
            )),
        );
    }

    if let Err(e) = state_guard
        .db
        .set_setting(
            &lot_deadline_key(&lot_id),
            &req.check_in_deadline_minutes.to_string(),
        )
        .await
    {
        tracing::error!("Failed to save noshow deadline: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update config",
            )),
        );
    }

    if let Err(e) = state_guard
        .db
        .set_setting(
            &lot_claim_window_key(&lot_id),
            &req.claim_window_minutes.to_string(),
        )
        .await
    {
        tracing::error!("Failed to save claim window: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update config",
            )),
        );
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "")
        .resource("lot_noshow_config", &lot_id)
        .details(serde_json::json!({
            "check_in_deadline_minutes": req.check_in_deadline_minutes,
            "claim_window_minutes": req.claim_window_minutes,
        }))
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(LotNoshowConfig {
            check_in_deadline_minutes: req.check_in_deadline_minutes,
            claim_window_minutes: req.claim_window_minutes,
        })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Waitlist offers: list + claim
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/waitlist/offers` — list active claim offers for the current user
#[utoipa::path(
    get, path = "/api/v1/waitlist/offers", tag = "Waitlist",
    summary = "List my waitlist offers",
    description = "Returns waitlist entries with status `offered` belonging to the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Active offers"))
)]
pub async fn list_my_offers(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<WaitlistEntry>>> {
    let state_guard = state.read().await;
    let entries = state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();
    let offers: Vec<WaitlistEntry> = entries
        .into_iter()
        .filter(|e| e.status == WaitlistStatus::Offered)
        .collect();
    Json(ApiResponse::success(offers))
}

/// Request body for claiming a waitlist offer
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ClaimOfferRequest {
    /// Booking start time (defaults to now if omitted).
    #[serde(default)]
    pub start_time: Option<DateTime<Utc>>,
    /// Booking end time (defaults to `start_time + 2h` if omitted).
    #[serde(default)]
    pub end_time: Option<DateTime<Utc>>,
}

/// `POST /api/v1/waitlist/offers/{id}/claim` — convert a waitlist offer into a booking
///
/// Finds the first available slot in the offered lot and creates a Confirmed
/// booking. The claim is atomic under a write lock — double-claim is rejected
/// with 409 (entry no longer in `offered` state).
///
/// Promotion order is strict FIFO (AI-Act compliant).
#[utoipa::path(
    post, path = "/api/v1/waitlist/offers/{id}/claim", tag = "Waitlist",
    summary = "Claim a waitlist offer",
    description = "Converts an active waitlist offer into a confirmed booking. \
                   Finds the first available slot in the offered lot. \
                   Promotion order is strict FIFO by `created_at` (AI-Act compliant). \
                   Idempotent: if the entry is already Accepted the request is rejected \
                   with 409 so the caller must use the booking referenced in \
                   `accepted_booking_id`.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Waitlist entry ID")),
    request_body = ClaimOfferRequest,
    responses(
        (status = 201, description = "Booking created from offer"),
        (status = 404, description = "Offer not found"),
        (status = 403, description = "Not your offer"),
        (status = 409, description = "No available slot or entry not in offered state"),
        (status = 410, description = "Offer expired"),
    )
)]
pub async fn claim_offer(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(entry_id): Path<String>,
    Json(req): Json<ClaimOfferRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    // Hold the write lock for the entire transaction so no concurrent claim
    // can race on the same offer or slot.
    let state_guard = state.write().await;

    let entry = match state_guard.db.get_waitlist_entry(&entry_id).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Waitlist offer not found")),
            );
        }
        Err(e) => {
            tracing::error!("DB error fetching waitlist entry: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if entry.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Not your offer")),
        );
    }

    if entry.status != WaitlistStatus::Offered {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "NOT_OFFERED",
                "This entry is not in offered state",
            )),
        );
    }

    let now = Utc::now();

    // Reject expired offers.
    if let Some(expires) = entry.offer_expires_at
        && now > expires
    {
        let mut expired = entry.clone();
        expired.status = WaitlistStatus::Expired;
        let _ = state_guard.db.save_waitlist_entry(&expired).await;
        return (
            StatusCode::GONE,
            Json(ApiResponse::error("OFFER_EXPIRED", "The offer has expired")),
        );
    }

    // Find an available slot in the lot (write lock held → race-safe).
    let slots = state_guard
        .db
        .list_slots_by_lot(&entry.lot_id.to_string())
        .await
        .unwrap_or_default();

    let Some(slot) = slots
        .into_iter()
        .find(|s| s.status == SlotStatus::Available)
    else {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "NO_SLOT",
                "No available slot in lot right now",
            )),
        );
    };

    // Find the user's default vehicle (or first vehicle; stub if none).
    let vehicles = state_guard
        .db
        .list_vehicles_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();
    let vehicle = vehicles
        .iter()
        .find(|v| v.is_default)
        .or_else(|| vehicles.first())
        .cloned()
        .unwrap_or_else(|| Vehicle {
            id: Uuid::new_v4(),
            user_id: auth_user.user_id,
            license_plate: "UNKNOWN".to_string(),
            make: None,
            model: None,
            color: None,
            vehicle_type: VehicleType::Car,
            fuel_type: FuelType::Unknown,
            is_default: false,
            created_at: now,
        });

    // Resolve floor name from the lot's floor list.
    let floor_name = state_guard
        .db
        .get_parking_lot(&entry.lot_id.to_string())
        .await
        .unwrap_or(None)
        .as_ref()
        .and_then(|l| {
            l.floors
                .iter()
                .find(|f| f.id == slot.floor_id)
                .map(|f| f.name.clone())
        })
        .unwrap_or_else(|| "Level 1".to_string());

    let start_time = req.start_time.unwrap_or(now);
    let end_time = req
        .end_time
        .unwrap_or_else(|| start_time + Duration::hours(2));

    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: entry.lot_id,
        slot_id: slot.id,
        slot_number: slot.slot_number,
        floor_name,
        vehicle,
        start_time,
        end_time,
        status: BookingStatus::Confirmed,
        pricing: BookingPricing {
            base_price: 0.0,
            discount: 0.0,
            tax: 0.0,
            total: 0.0,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: None,
        notes: Some(format!("Claimed via waitlist offer {entry_id}")),
        tenant_id: None,
    };

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save claimed booking: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create booking",
            )),
        );
    }

    // Mark slot as Occupied.
    if let Err(e) = state_guard
        .db
        .update_slot_status(&slot.id.to_string(), SlotStatus::Occupied)
        .await
    {
        tracing::warn!("Failed to mark slot {} occupied: {e}", slot.id);
    }

    // Mark entry as Accepted with the new booking ID.
    let mut accepted = entry;
    accepted.status = WaitlistStatus::Accepted;
    accepted.accepted_booking_id = Some(booking.id);
    if let Err(e) = state_guard.db.save_waitlist_entry(&accepted).await {
        tracing::warn!("Failed to update waitlist entry to Accepted: {e}");
    }

    AuditEntry::new(AuditEventType::BookingCreated)
        .user(auth_user.user_id, "")
        .resource("booking", &booking.id.to_string())
        .details(serde_json::json!({"source": "waitlist_claim", "entry_id": entry_id}))
        .log();

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %booking.id,
        lot_id = %booking.lot_id,
        "Waitlist offer claimed, booking created"
    );

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared promotion logic (called by jobs + cancel flow)
// ─────────────────────────────────────────────────────────────────────────────

/// Promote the next FIFO-ordered Waiting entry for `lot_id` to Offered status.
///
/// Sets `status = Offered`, `notified_at = now`, `offer_expires_at = now +
/// claim_window_minutes`, and creates an in-app notification.
///
/// Returns `true` if an entry was promoted, `false` if no Waiting entry exists.
///
/// **AI-Act compliance**: promotion is strict FIFO by `created_at`. The DB
/// layer (`list_waitlist_by_lot`) sorts ascending by `created_at` — no further
/// reordering is done here.
pub async fn promote_next_waitlist_offer(
    state: &AppState,
    lot_id: Uuid,
    claim_window_minutes: i64,
) -> bool {
    let entries = state
        .db
        .list_waitlist_by_lot(&lot_id.to_string())
        .await
        .unwrap_or_default();

    // FIFO: take the first entry still Waiting (list is sorted by created_at asc).
    let Some(next) = entries.iter().find(|e| e.status == WaitlistStatus::Waiting) else {
        return false;
    };

    let now = Utc::now();
    let mut offered = next.clone();
    offered.status = WaitlistStatus::Offered;
    offered.notified_at = Some(now);
    offered.offer_expires_at = Some(now + Duration::minutes(claim_window_minutes));

    if let Err(e) = state.db.save_waitlist_entry(&offered).await {
        tracing::warn!(entry_id = %offered.id, "promote_next_waitlist_offer: save failed: {e}");
        return false;
    }

    let notification = Notification {
        id: Uuid::new_v4(),
        user_id: offered.user_id,
        notification_type: NotificationType::WaitlistOffer,
        title: "Parking spot available!".to_string(),
        message: format!(
            "A spot has opened up. You have {claim_window_minutes} minutes to claim it."
        ),
        data: Some(serde_json::json!({
            "lot_id": lot_id,
            "entry_id": offered.id,
            "expires_at": offered.offer_expires_at,
        })),
        read: false,
        created_at: now,
    };
    let _ = state.db.save_notification(&notification).await;

    tracing::info!(
        entry_id = %offered.id,
        user_id = %offered.user_id,
        lot_id = %lot_id,
        expires_in_minutes = claim_window_minutes,
        "Waitlist offer promoted (FIFO)"
    );

    true
}

/// Expire all outstanding offers whose `offer_expires_at` has passed, then
/// promote the next Waiting entry in each affected lot.
///
/// Called every 5 minutes by the background scheduler.
pub async fn expire_outstanding_offers(state: &AppState) -> anyhow::Result<()> {
    let all_lots = state.db.list_parking_lots().await?;
    let now = Utc::now();
    let mut expired_count = 0u32;

    for lot in &all_lots {
        let entries = state
            .db
            .list_waitlist_by_lot(&lot.id.to_string())
            .await
            .unwrap_or_default();

        let claim_window = lot_claim_window_minutes(state, &lot.id.to_string()).await;

        for entry in entries
            .iter()
            .filter(|e| e.status == WaitlistStatus::Offered)
        {
            if entry.offer_expires_at.is_none_or(|exp| now <= exp) {
                continue;
            }

            let mut expired_entry = entry.clone();
            expired_entry.status = WaitlistStatus::Expired;
            if let Err(e) = state.db.save_waitlist_entry(&expired_entry).await {
                tracing::warn!(
                    entry_id = %entry.id,
                    "expire_outstanding_offers: failed to expire entry: {e}"
                );
                continue;
            }
            expired_count += 1;
            tracing::info!(
                entry_id = %entry.id,
                lot_id = %lot.id,
                "Waitlist offer expired — promoting next in line"
            );

            promote_next_waitlist_offer(state, lot.id, claim_window).await;
        }
    }

    if expired_count > 0 {
        tracing::info!("expire_outstanding_offers: expired {expired_count} offer(s)");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::db::{Database, DatabaseConfig};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn make_test_state() -> (Arc<RwLock<AppState>>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_config = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        let db = Database::open(&db_config).expect("open test db");
        let config = ServerConfig::default();
        let state = Arc::new(RwLock::new(AppState {
            config,
            db,
            mdns: None,
            scheduler: None,
            ws_events: crate::api::ws::EventBroadcaster::new(),
            fleet_events: crate::api::sse::FleetEventBroadcaster::new(),
            revocation_store: crate::jwt::TokenRevocationList::new(),
        }));
        (state, dir)
    }

    fn waiting_entry(lot_id: Uuid, user_id: Uuid, age_minutes: i64) -> WaitlistEntry {
        WaitlistEntry {
            id: Uuid::new_v4(),
            user_id,
            lot_id,
            created_at: Utc::now() - Duration::minutes(age_minutes),
            notified_at: None,
            status: WaitlistStatus::Waiting,
            offer_expires_at: None,
            accepted_booking_id: None,
        }
    }

    fn make_test_lot(lot_id: Uuid) -> parkhub_common::ParkingLot {
        parkhub_common::ParkingLot {
            id: lot_id,
            name: "Test Lot".to_string(),
            address: "1 Test St".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            total_slots: 10,
            available_slots: 10,
            floors: vec![],
            amenities: vec![],
            pricing: parkhub_common::PricingInfo {
                currency: "EUR".to_string(),
                rates: vec![],
                daily_max: None,
                monthly_pass: None,
            },
            operating_hours: parkhub_common::OperatingHours {
                is_24h: true,
                monday: None,
                tuesday: None,
                wednesday: None,
                thursday: None,
                friday: None,
                saturday: None,
                sunday: None,
            },
            images: vec![],
            status: parkhub_common::LotStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tenant_id: None,
        }
    }

    // ── promote_next_waitlist_offer ────────────────────────────────────────

    #[tokio::test]
    async fn promote_creates_offer_for_first_waiting() {
        let (state, _dir) = make_test_state();
        let lot_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let entry = waiting_entry(lot_id, user_id, 5);
        let entry_id = entry.id;

        {
            let guard = state.read().await;
            guard.db.save_waitlist_entry(&entry).await.unwrap();
        }

        let guard = state.read().await;
        let promoted = promote_next_waitlist_offer(&guard, lot_id, 15).await;

        assert!(promoted, "should promote the waiting entry");
        let updated = guard
            .db
            .get_waitlist_entry(&entry_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, WaitlistStatus::Offered);
        assert!(
            updated.offer_expires_at.is_some(),
            "offer_expires_at must be set"
        );
        assert!(updated.notified_at.is_some(), "notified_at must be set");
    }

    #[tokio::test]
    async fn promote_returns_false_when_no_waiting_entries() {
        let (state, _dir) = make_test_state();
        let lot_id = Uuid::new_v4();

        let guard = state.read().await;
        let promoted = promote_next_waitlist_offer(&guard, lot_id, 15).await;
        assert!(!promoted, "empty waitlist should return false");
    }

    #[tokio::test]
    async fn promote_skips_offered_and_promotes_next_waiting() {
        let (state, _dir) = make_test_state();
        let lot_id = Uuid::new_v4();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // entry1 is already Offered (older entry).
        let mut entry1 = waiting_entry(lot_id, user1, 20);
        entry1.status = WaitlistStatus::Offered;
        entry1.offer_expires_at = Some(Utc::now() + Duration::minutes(10));

        // entry2 is Waiting (newer, but first Waiting in FIFO after entry1).
        let entry2 = waiting_entry(lot_id, user2, 10);
        let entry2_id = entry2.id;

        {
            let guard = state.read().await;
            guard.db.save_waitlist_entry(&entry1).await.unwrap();
            guard.db.save_waitlist_entry(&entry2).await.unwrap();
        }

        let guard = state.read().await;
        let promoted = promote_next_waitlist_offer(&guard, lot_id, 15).await;
        assert!(promoted);

        let updated2 = guard
            .db
            .get_waitlist_entry(&entry2_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated2.status,
            WaitlistStatus::Offered,
            "first Waiting entry must be promoted, not the already-Offered one"
        );
    }

    #[tokio::test]
    async fn promote_offer_expires_at_uses_claim_window() {
        let (state, _dir) = make_test_state();
        let lot_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let entry = waiting_entry(lot_id, user_id, 5);
        let entry_id = entry.id;

        {
            let guard = state.read().await;
            guard.db.save_waitlist_entry(&entry).await.unwrap();
        }

        let before = Utc::now();
        let guard = state.read().await;
        promote_next_waitlist_offer(&guard, lot_id, 20).await;

        let updated = guard
            .db
            .get_waitlist_entry(&entry_id.to_string())
            .await
            .unwrap()
            .unwrap();
        let expires = updated.offer_expires_at.unwrap();
        // expires_at must be approximately now + 20 minutes.
        assert!(expires > before + Duration::minutes(19));
        assert!(expires < before + Duration::minutes(21));
    }

    // ── expire_outstanding_offers ──────────────────────────────────────────

    #[tokio::test]
    async fn expire_marks_expired_and_promotes_next() {
        let (state, _dir) = make_test_state();
        let lot_id = Uuid::new_v4();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        {
            let guard = state.read().await;
            guard
                .db
                .save_parking_lot(&make_test_lot(lot_id))
                .await
                .unwrap();

            // entry1: Offered but already expired.
            let mut entry1 = waiting_entry(lot_id, user1, 30);
            entry1.status = WaitlistStatus::Offered;
            entry1.offer_expires_at = Some(Utc::now() - Duration::minutes(1));
            guard.db.save_waitlist_entry(&entry1).await.unwrap();

            // entry2: Waiting — should be promoted after entry1 expires.
            let entry2 = waiting_entry(lot_id, user2, 20);
            guard.db.save_waitlist_entry(&entry2).await.unwrap();
        }

        let entries_before: Vec<_> = {
            let guard = state.read().await;
            guard
                .db
                .list_waitlist_by_lot(&lot_id.to_string())
                .await
                .unwrap()
        };
        let entry1_id = entries_before
            .iter()
            .find(|e| e.status == WaitlistStatus::Offered)
            .unwrap()
            .id;
        let entry2_id = entries_before
            .iter()
            .find(|e| e.status == WaitlistStatus::Waiting)
            .unwrap()
            .id;

        {
            let guard = state.read().await;
            expire_outstanding_offers(&guard).await.unwrap();
        }

        let guard = state.read().await;
        let e1 = guard
            .db
            .get_waitlist_entry(&entry1_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            e1.status,
            WaitlistStatus::Expired,
            "expired offer must be marked Expired"
        );

        let e2 = guard
            .db
            .get_waitlist_entry(&entry2_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            e2.status,
            WaitlistStatus::Offered,
            "next Waiting entry must be promoted after expiry"
        );
    }

    #[tokio::test]
    async fn expire_does_not_touch_non_expired_offers() {
        let (state, _dir) = make_test_state();
        let lot_id = Uuid::new_v4();
        let user1 = Uuid::new_v4();

        {
            let guard = state.read().await;
            guard
                .db
                .save_parking_lot(&make_test_lot(lot_id))
                .await
                .unwrap();

            let mut entry1 = waiting_entry(lot_id, user1, 5);
            entry1.status = WaitlistStatus::Offered;
            entry1.offer_expires_at = Some(Utc::now() + Duration::minutes(10)); // NOT expired
            guard.db.save_waitlist_entry(&entry1).await.unwrap();
        }

        let entry1_id = {
            let guard = state.read().await;
            guard
                .db
                .list_waitlist_by_lot(&lot_id.to_string())
                .await
                .unwrap()[0]
                .id
        };

        {
            let guard = state.read().await;
            expire_outstanding_offers(&guard).await.unwrap();
        }

        let guard = state.read().await;
        let e1 = guard
            .db
            .get_waitlist_entry(&entry1_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            e1.status,
            WaitlistStatus::Offered,
            "non-expired offer must not be touched"
        );
    }

    // ── Settings key helpers ───────────────────────────────────────────────

    #[test]
    fn lot_deadline_key_has_correct_format() {
        assert_eq!(lot_deadline_key("abc-123"), "lot_noshow_deadline:abc-123");
    }

    #[test]
    fn lot_claim_window_key_has_correct_format() {
        assert_eq!(lot_claim_window_key("abc-123"), "lot_claim_window:abc-123");
    }

    #[tokio::test]
    async fn lot_deadline_defaults_to_30_when_not_set() {
        let (state, _dir) = make_test_state();
        let guard = state.read().await;
        let mins = lot_deadline_minutes(&guard, "nonexistent-lot").await;
        assert_eq!(mins, DEFAULT_DEADLINE_MINUTES);
    }

    #[tokio::test]
    async fn lot_claim_window_defaults_to_15_when_not_set() {
        let (state, _dir) = make_test_state();
        let guard = state.read().await;
        let mins = lot_claim_window_minutes(&guard, "nonexistent-lot").await;
        assert_eq!(mins, DEFAULT_CLAIM_WINDOW_MINUTES);
    }

    #[tokio::test]
    async fn lot_deadline_reads_per_lot_setting() {
        let (state, _dir) = make_test_state();
        let lot_id = "test-lot-42";
        {
            let guard = state.read().await;
            guard
                .db
                .set_setting(&lot_deadline_key(lot_id), "45")
                .await
                .unwrap();
        }
        let guard = state.read().await;
        let mins = lot_deadline_minutes(&guard, lot_id).await;
        assert_eq!(mins, 45);
    }

    #[tokio::test]
    async fn lot_claim_window_reads_per_lot_setting() {
        let (state, _dir) = make_test_state();
        let lot_id = "test-lot-99";
        {
            let guard = state.read().await;
            guard
                .db
                .set_setting(&lot_claim_window_key(lot_id), "20")
                .await
                .unwrap();
        }
        let guard = state.read().await;
        let mins = lot_claim_window_minutes(&guard, lot_id).await;
        assert_eq!(mins, 20);
    }
}
