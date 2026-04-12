//! Booking handlers: create, list, get, cancel, quick-book,
//! invoice generation, check-in, and booking updates.
//!
//! Extracted from mod.rs to keep the router module focused on routing.

#![allow(unused_imports)]

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use chrono::{DateTime, Datelike, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use uuid::Uuid;

use parkhub_common::{
    ApiResponse, Booking, BookingPricing, BookingStatus, CreateBookingRequest, CreditTransaction,
    CreditTransactionType, PaymentStatus, SlotStatus, User, UserRole, Vehicle, VehicleType,
};

use crate::audit::{AuditEntry, AuditEventType};
#[cfg(feature = "mod-email")]
use crate::email;
use crate::metrics;
use crate::utils::html_escape;

use super::{AuthUser, SharedState, VAT_RATE, check_admin, read_admin_setting};

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

#[utoipa::path(get, path = "/api/v1/bookings", tag = "Bookings",
    summary = "List current user's bookings",
    description = "Returns all bookings for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "List of bookings"))
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub async fn list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Booking>>> {
    let state = state.read().await;

    match state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(bookings) => {
            tracing::debug!(count = bookings.len(), "Listed bookings");
            Json(ApiResponse::success(bookings))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list bookings");
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list bookings",
            ))
        }
    }
}

#[utoipa::path(post, path = "/api/v1/bookings", tag = "Bookings",
    summary = "Create a new booking",
    description = "Books a parking slot for the authenticated user.",
    security(("bearer_auth" = [])),
    request_body = CreateBookingRequest,
    responses((status = 201, description = "Booking created"), (status = 404, description = "Not found"), (status = 409, description = "Slot unavailable"))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, slot_id = %req.slot_id))]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub async fn create_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateBookingRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    // ── Input length validation (issue #115) ────────────────────────────────
    if req.license_plate.len() > 20 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "License plate must be at most 20 characters",
            )),
        );
    }
    if let Some(ref notes) = req.notes
        && notes.len() > 500
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Notes must be at most 500 characters",
            )),
        );
    }
    // ── Phase 1: reads under a read lock ──────────────────────────────────────
    // Collect all data needed to validate and price the booking.  A read lock
    // allows concurrent readers; we release it before any mutation.
    #[allow(unused_variables)]
    let (
        slot,
        vehicle,
        require_vehicle,
        plate_mode,
        duration_hours,
        min_hours,
        max_hours,
        max_per_day,
        same_day_count,
        credits_enabled,
        credits_per_booking,
        mut booking_user,
        lot_opt,
        org_name,
    ) = {
        let rg = state.read().await;

        // Check if slot exists and is available
        let slot = match rg.db.get_parking_slot(&req.slot_id.to_string()).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("NOT_FOUND", "Slot not found")),
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

        if slot.status != SlotStatus::Available {
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::error(
                    "SLOT_UNAVAILABLE",
                    "This slot is not available",
                )),
            );
        }

        // Get or create vehicle info
        let vehicle = match rg.db.get_vehicle(&req.vehicle_id.to_string()).await {
            Ok(Some(v)) => {
                if v.user_id != auth_user.user_id {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(ApiResponse::error(
                            "FORBIDDEN",
                            "Vehicle does not belong to you",
                        )),
                    );
                }
                v
            }
            _ => Vehicle {
                id: req.vehicle_id,
                user_id: auth_user.user_id,
                license_plate: req.license_plate.clone(),
                make: None,
                model: None,
                color: None,
                vehicle_type: VehicleType::Car,
                is_default: false,
                created_at: Utc::now(),
            },
        };

        // Admin settings
        let require_vehicle = read_admin_setting(&rg.db, "require_vehicle").await;
        let plate_mode = read_admin_setting(&rg.db, "license_plate_mode").await;
        let duration_hours = f64::from(req.duration_minutes) / 60.0;
        let min_hours: f64 = read_admin_setting(&rg.db, "min_booking_duration_hours")
            .await
            .parse()
            .unwrap_or(0.0);
        let max_hours: f64 = read_admin_setting(&rg.db, "max_booking_duration_hours")
            .await
            .parse()
            .unwrap_or(0.0);
        let max_per_day: i32 = rg
            .db
            .get_setting("max_bookings_per_day")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let same_day_count = if max_per_day > 0 {
            let booking_date = req.start_time.date_naive();
            rg.db
                .count_bookings_for_user_on_day(&auth_user.user_id.to_string(), booking_date)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        error = %e,
                        user_id = %auth_user.user_id,
                        booking_date = %booking_date,
                        "Failed to count same-day bookings from canonical table"
                    );
                    0
                })
        } else {
            0
        };

        // Credits settings
        let credits_enabled = rg
            .db
            .get_setting("credits_enabled")
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
            == "true";
        let credits_per_booking: i32 = rg
            .db
            .get_setting("credits_per_booking")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let Ok(Some(booking_user)) = rg.db.get_user(&auth_user.user_id.to_string()).await else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to load user")),
            );
        };

        let lot_opt = rg
            .db
            .get_parking_lot(&req.lot_id.to_string())
            .await
            .ok()
            .flatten();

        let org_name = rg.config.organization_name.clone();

        (
            slot,
            vehicle,
            require_vehicle,
            plate_mode,
            duration_hours,
            min_hours,
            max_hours,
            max_per_day,
            same_day_count,
            credits_enabled,
            credits_per_booking,
            booking_user,
            lot_opt,
            org_name,
        )
    };
    // Read lock released here.

    // ── Stateless validation (no lock needed) ─────────────────────────────────

    // Validate duration is positive before arithmetic
    if req.duration_minutes <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Duration must be positive",
            )),
        );
    }

    // Validate start_time is in the future (at least 1 minute from now)
    if req.start_time <= Utc::now() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_BOOKING_TIME",
                "Booking start time must be in the future",
            )),
        );
    }

    // ── Admin settings enforcement ─────────────────────────────────────────

    if require_vehicle == "true" && req.vehicle_id == Uuid::nil() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VEHICLE_REQUIRED",
                "A vehicle is required for booking",
            )),
        );
    }

    if plate_mode == "required" && req.license_plate.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "LICENSE_PLATE_REQUIRED",
                "A license plate is required for booking",
            )),
        );
    }

    if min_hours > 0.0 && duration_hours < min_hours {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "DURATION_TOO_SHORT",
                format!("Minimum booking duration is {min_hours} hour(s)"),
            )),
        );
    }

    if max_hours > 0.0 && duration_hours > max_hours {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "DURATION_TOO_LONG",
                format!("Maximum booking duration is {max_hours} hour(s)"),
            )),
        );
    }

    if max_per_day > 0 && same_day_count >= usize::try_from(max_per_day).unwrap_or(0) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "MAX_BOOKINGS_REACHED",
                format!("Maximum of {max_per_day} booking(s) per day reached"),
            )),
        );
    }

    // ── Operating hours validation ──────────────────────────────────────────
    #[cfg(feature = "mod-operating-hours")]
    if let Some(ref lot) = lot_opt {
        let end_time = req.start_time + TimeDelta::minutes(i64::from(req.duration_minutes));
        if let Some(msg) = super::operating_hours::validate_booking_hours(
            &lot.operating_hours,
            &req.start_time,
            &end_time,
        ) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("OUTSIDE_OPERATING_HOURS", msg)),
            );
        }
    }

    // ── End admin settings enforcement ──────────────────────────────────────

    let is_admin_user =
        booking_user.role == UserRole::Admin || booking_user.role == UserRole::SuperAdmin;

    if credits_enabled && !is_admin_user && booking_user.credits_balance < credits_per_booking {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "INSUFFICIENT_CREDITS",
                "Not enough credits for this booking",
            )),
        );
    }

    // Calculate pricing (no lock needed)
    let end_time = req.start_time + TimeDelta::minutes(i64::from(req.duration_minutes));

    let hourly_rate = lot_opt
        .as_ref()
        .and_then(|lot| lot.pricing.rates.iter().find(|r| r.duration_minutes == 60))
        .map_or(2.0, |r| r.price);
    let daily_max = lot_opt.as_ref().and_then(|lot| lot.pricing.daily_max);
    let lot_currency = lot_opt
        .as_ref()
        .map_or_else(|| "EUR".to_string(), |lot| lot.pricing.currency.clone());

    // Cap at daily_max if configured (e.g. all-day price ceiling)
    let raw_price = (f64::from(req.duration_minutes) / 60.0) * hourly_rate;
    let base_price = daily_max.map_or(raw_price, |cap| raw_price.min(cap));
    let tax = base_price * VAT_RATE;
    let total = base_price + tax;

    let floor_name = lot_opt.as_ref().map_or_else(
        || "Level 1".to_string(),
        |lot| {
            lot.floors
                .iter()
                .find(|f| f.id == slot.floor_id)
                .map_or_else(|| "Level 1".to_string(), |f| f.name.clone())
        },
    );

    let now = Utc::now();
    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        slot_number: slot.slot_number,
        floor_name,
        vehicle,
        start_time: req.start_time,
        end_time,
        status: BookingStatus::Confirmed,
        pricing: BookingPricing {
            base_price,
            discount: 0.0,
            tax,
            total,
            currency: lot_currency,
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: Some(Uuid::new_v4().to_string()),
        notes: req.notes,
        tenant_id: None,
    };

    // ── Phase 2: mutations under a write lock ──────────────────────────────────
    // Re-check slot availability and commit all mutations atomically.
    // The write lock serialises concurrent booking attempts for the same slot,
    // preventing double-booking between the availability check and the insert.
    #[allow(unused_variables)]
    let user_info_opt = {
        let state_guard = state.write().await;

        // Re-check slot availability now that we hold the write lock.
        match state_guard
            .db
            .get_parking_slot(&req.slot_id.to_string())
            .await
        {
            Ok(Some(s)) if s.status != SlotStatus::Available => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::error(
                        "SLOT_UNAVAILABLE",
                        "This slot is not available",
                    )),
                );
            }
            Err(e) => {
                tracing::error!("Database error on slot re-check: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
                );
            }
            _ => {}
        }

        if let Err(e) = state_guard.db.save_booking(&booking).await {
            tracing::error!("Failed to save booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to create booking",
                )),
            );
        }

        // Update slot status atomically within the write-lock scope.
        let mut updated_slot = slot;
        updated_slot.status = SlotStatus::Reserved;
        if let Err(e) = state_guard.db.save_parking_slot(&updated_slot).await {
            tracing::error!("Failed to update slot status after booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SLOT_UPDATE_FAILED",
                    "Booking created but slot status could not be updated. Please contact support.",
                )),
            );
        }

        tracing::info!(
            user_id = %auth_user.user_id,
            booking_id = %booking.id,
            slot_id = %booking.slot_id,
            "Booking created"
        );

        // Deduct credits if enabled and user is not admin
        if credits_enabled && !is_admin_user {
            booking_user.credits_balance -= credits_per_booking;
            if let Err(e) = state_guard.db.save_user(&booking_user).await {
                tracing::warn!("Failed to save user credit deduction: {e}");
            }
            let tx = CreditTransaction {
                id: Uuid::new_v4(),
                user_id: auth_user.user_id,
                booking_id: Some(booking.id),
                amount: -credits_per_booking,
                transaction_type: CreditTransactionType::Deduction,
                description: Some(format!("Booking {}", booking.id)),
                granted_by: None,
                created_at: Utc::now(),
            };
            if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
                tracing::warn!("Failed to save credit transaction: {e}");
            }
        }

        // Fetch user details for audit log and confirmation email
        let user_info_opt = state_guard
            .db
            .get_user(&auth_user.user_id.to_string())
            .await
            .ok()
            .flatten();

        let audit_entry = if let Some(ref u) = user_info_opt {
            crate::audit::events::booking_created(auth_user.user_id, &u.username, booking.id)
        } else {
            crate::audit::events::booking_created(auth_user.user_id, "", booking.id)
        };
        audit_entry.persist(&state_guard.db).await;

        // Write lock released at end of this block.
        user_info_opt
    };

    // Broadcast WebSocket event for real-time updates
    {
        let state_r = state.read().await;
        state_r
            .ws_events
            .broadcast(crate::api::ws::WsEvent::booking_created(
                &booking.lot_id.to_string(),
                &booking.slot_id.to_string(),
                &auth_user.user_id.to_string(),
            ));
    }

    // Dispatch webhook event (non-blocking)
    #[cfg(feature = "mod-webhooks")]
    {
        let state_clone = state.clone();
        let booking_json = serde_json::json!({
            "booking_id": booking.id,
            "user_id": auth_user.user_id,
            "lot_id": booking.lot_id,
            "slot_number": booking.slot_number,
            "start_time": booking.start_time,
            "end_time": booking.end_time,
        });
        tokio::spawn(async move {
            crate::api::webhooks::dispatch_webhook_event(
                &state_clone,
                "booking.created",
                booking_json,
            )
            .await;
        });
    }
    metrics::record_booking_event("created");

    // Send booking confirmation email (non-blocking, fire-and-forget).
    #[cfg(feature = "mod-email")]
    if let Some(u) = user_info_opt {
        let booking_id_str = booking.id.to_string();
        let floor_name = booking.floor_name.clone();
        let slot_number = booking.slot_number;
        let start_time_str = booking.start_time.format("%Y-%m-%d %H:%M UTC").to_string();
        let end_time_str = booking.end_time.format("%Y-%m-%d %H:%M UTC").to_string();
        let user_email = u.email.clone();
        let user_name = u.name;
        tokio::spawn(async move {
            let email_html = email::build_booking_confirmation_email(
                &user_name,
                &booking_id_str,
                &floor_name,
                slot_number,
                &start_time_str,
                &end_time_str,
                &org_name,
            );
            if let Err(e) =
                email::send_email(&user_email, "Booking Confirmation — ParkHub", &email_html).await
            {
                tracing::warn!("Failed to send booking confirmation email: {}", e);
            }
        });
    }

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

#[utoipa::path(get, path = "/api/v1/bookings/{id}", tag = "Bookings",
    summary = "Get booking by ID",
    description = "Returns a single booking. Only the owner can access it.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Booking UUID")),
    responses((status = 200, description = "Booking found"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id, booking_id = %id))]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub async fn get_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state = state.read().await;

    match state.db.get_booking(&id).await {
        Ok(Some(booking)) => {
            if booking.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
            (StatusCode::OK, Json(ApiResponse::success(booking)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

#[utoipa::path(delete, path = "/api/v1/bookings/{id}", tag = "Bookings",
    summary = "Cancel a booking",
    description = "Cancels an active booking and releases the slot.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Booking UUID")),
    responses((status = 200, description = "Cancelled"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id, booking_id = %id))]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub async fn cancel_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Use write lock so the booking status update and slot status update are
    // made while no other booking creation can interleave.
    let state_guard = state.write().await;

    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
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

    if booking.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    // Only Confirmed or Pending bookings can be cancelled.
    if booking.status == BookingStatus::Cancelled {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "ALREADY_CANCELLED",
                "Booking is already cancelled",
            )),
        );
    }

    let mut updated_booking = booking.clone();
    updated_booking.status = BookingStatus::Cancelled;
    updated_booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&updated_booking).await {
        tracing::error!("Failed to update booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel booking",
            )),
        );
    }

    // Free up the slot — only restore to Available if it was Reserved.
    // Slots in Maintenance or Disabled state must remain as-is.
    if let Ok(Some(mut slot)) = state_guard
        .db
        .get_parking_slot(&booking.slot_id.to_string())
        .await
        && slot.status == SlotStatus::Reserved
    {
        slot.status = SlotStatus::Available;
        if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
            tracing::error!("Failed to restore slot status after cancellation: {}", e);
        }
    }

    // Refund credits if credits system is enabled
    let credits_enabled = state_guard
        .db
        .get_setting("credits_enabled")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        == "true";
    if credits_enabled {
        let credits_per_booking: i32 = state_guard
            .db
            .get_setting("credits_per_booking")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        if let Ok(Some(mut user)) = state_guard
            .db
            .get_user(&auth_user.user_id.to_string())
            .await
            && user.role != UserRole::Admin
            && user.role != UserRole::SuperAdmin
        {
            user.credits_balance += credits_per_booking;
            if let Err(e) = state_guard.db.save_user(&user).await {
                tracing::warn!("Failed to save user credit refund: {e}");
            }
            let tx = CreditTransaction {
                id: Uuid::new_v4(),
                user_id: auth_user.user_id,
                booking_id: Some(booking.id),
                amount: credits_per_booking,
                transaction_type: CreditTransactionType::Refund,
                description: Some(format!("Cancelled booking {}", booking.id)),
                granted_by: None,
                created_at: Utc::now(),
            };
            if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
                tracing::warn!("Failed to save credit transaction: {e}");
            }
        }
    }

    // Fetch user for audit log + cancellation email
    let user = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();

    AuditEntry::new(AuditEventType::BookingCancelled)
        .user(auth_user.user_id, &username)
        .resource("booking", &id)
        .log();

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %id,
        "Booking cancelled"
    );

    // Send cancellation confirmation email (async, best-effort)
    #[cfg(feature = "mod-email")]
    if let Some(ref user) = user {
        let user_email = user.email.clone();
        let user_name = user.name.clone();
        let booking_id_str = booking.id.to_string();
        let org_name = state_guard.config.organization_name.clone();
        let start_time = booking.start_time.format("%Y-%m-%d %H:%M").to_string();
        let end_time = booking.end_time.format("%Y-%m-%d %H:%M").to_string();
        let floor = booking.floor_name.clone();
        let slot = booking.slot_number;
        tokio::spawn(async move {
            let email_html = email::build_booking_cancellation_email(
                &user_name,
                &booking_id_str,
                &floor,
                slot,
                &start_time,
                &end_time,
                &org_name,
            );
            if let Err(e) =
                email::send_email(&user_email, "Booking Cancelled — ParkHub", &email_html).await
            {
                tracing::warn!("Failed to send cancellation email: {}", e);
            }
        });
    }

    // Notify the first waitlist member that a slot is now available (async, best-effort)
    #[cfg(feature = "mod-email")]
    {
        let state_clone = state.clone();
        let lot_id_str = booking.lot_id.to_string();
        let org_name_wl = state_guard.config.organization_name.clone();
        tokio::spawn(async move {
            let state_r = state_clone.read().await;
            let lot_name = state_r
                .db
                .get_parking_lot(&lot_id_str)
                .await
                .ok()
                .flatten()
                .map_or_else(|| lot_id_str.clone(), |l| l.name);

            let waitlist = state_r
                .db
                .list_waitlist_by_lot(&lot_id_str)
                .await
                .unwrap_or_default();

            // Notify the earliest-queued user who has not yet been notified
            if let Some(entry) = waitlist.iter().find(|e| e.notified_at.is_none())
                && let Ok(Some(wl_user)) =
                    state_r.db.get_user(&entry.user_id.to_string()).await
            {
                let email_html = email::build_waitlist_slot_available_email(
                    &wl_user.name,
                    &lot_name,
                    &org_name_wl,
                );
                let subject = format!("Parking slot available at {lot_name} — ParkHub");
                if let Err(e) = email::send_email(&wl_user.email, &subject, &email_html).await {
                    tracing::warn!("Failed to send waitlist notification: {}", e);
                } else {
                    // Mark the entry as notified
                    let mut updated = entry.clone();
                    updated.notified_at = Some(Utc::now());
                    if let Err(e) = state_r.db.save_waitlist_entry(&updated).await {
                        tracing::warn!("Failed to update waitlist notified_at: {}", e);
                    }
                    tracing::info!(
                        user_id = %wl_user.id,
                        lot_id = %lot_id_str,
                        "Waitlist slot-available notification sent"
                    );
                }
            }
        });
    }

    // Broadcast WebSocket event for real-time updates
    state_guard
        .ws_events
        .broadcast(crate::api::ws::WsEvent::booking_cancelled(
            &booking.lot_id.to_string(),
            &booking.slot_id.to_string(),
        ));

    // Dispatch webhook event
    #[cfg(feature = "mod-webhooks")]
    {
        let state_clone = state.clone();
        let payload = serde_json::json!({
            "booking_id": id,
            "user_id": auth_user.user_id,
            "action": "cancelled",
        });
        tokio::spawn(async move {
            crate::api::webhooks::dispatch_webhook_event(
                &state_clone,
                "booking.cancelled",
                payload,
            )
            .await;
        });
    }
    metrics::record_booking_event("cancelled");

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// INVOICE
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/bookings/{id}/invoice`
///
/// Returns an HTML invoice for the given booking.  The authenticated user must
/// own the booking (admin users may retrieve any invoice).
///
/// The invoice includes:
/// - Company/organisation name from server config
/// - Booking reference (booking UUID)
/// - User name and email
/// - Parking lot name and slot number
/// - Start / end time and duration
/// - Itemised pricing: base price, VAT at 19% (German standard), total
#[allow(clippy::format_in_format_args)]
#[utoipa::path(get, path = "/api/v1/bookings/{id}/invoice", tag = "Bookings",
    summary = "Download booking invoice",
    description = "Generates a text invoice for a booking.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub async fn get_booking_invoice(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_guard = state.read().await;

    // Fetch the booking
    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Booking not found".to_string(),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching booking for invoice: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error".to_string(),
            );
        }
    };

    // Ownership check — only the booking owner (or admin) may fetch the invoice
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Access denied".to_string(),
        );
    };

    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Access denied".to_string(),
        );
    }

    // Fetch user details for the invoice
    let booking_user = match state_guard.db.get_user(&booking.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => caller.clone(),
    };

    // Fetch parking lot name
    let lot_name = match state_guard
        .db
        .get_parking_lot(&booking.lot_id.to_string())
        .await
    {
        Ok(Some(lot)) => lot.name,
        _ => "Unknown Parking Lot".to_string(),
    };

    let org_name = state_guard.config.organization_name.clone();
    let company = if org_name.is_empty() {
        "ParkHub".to_string()
    } else {
        org_name
    };

    // Calculate duration in minutes
    let duration_minutes = (booking.end_time - booking.start_time).num_minutes();
    let duration_hours = duration_minutes / 60;
    let duration_mins_part = duration_minutes % 60;

    // VAT breakdown (19% German standard — Umsatzsteuergesetz § 12 Abs. 1)
    let net_price = booking.pricing.base_price;
    let vat_amount = net_price * VAT_RATE;
    let gross_total = net_price + vat_amount;

    let invoice_date = booking.created_at.format("%d.%m.%Y").to_string();
    let start_str = booking.start_time.format("%d.%m.%Y %H:%M").to_string();
    let end_str = booking.end_time.format("%d.%m.%Y %H:%M").to_string();

    let invoice_number = format!(
        "INV-{}",
        booking
            .id
            .to_string()
            .to_uppercase()
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>()
    );

    // HTML-escape all user-controlled values to prevent stored XSS
    let company = html_escape(&company);
    let user_name = html_escape(&booking_user.name);
    let user_email = html_escape(&booking_user.email);
    let lot_name = html_escape(&lot_name);
    let floor_name = html_escape(&booking.floor_name);
    let license_plate = html_escape(&booking.vehicle.license_plate);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Rechnung {invoice_number}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{ font-family: 'Helvetica Neue', Arial, sans-serif; color: #1a1a2e; background: #f8f9fa; }}
    .page {{ max-width: 800px; margin: 40px auto; background: #ffffff; padding: 60px;
             box-shadow: 0 4px 20px rgba(0,0,0,0.08); border-radius: 4px; }}
    .header {{ display: flex; justify-content: space-between; align-items: flex-start;
               border-bottom: 3px solid #1a73e8; padding-bottom: 24px; margin-bottom: 40px; }}
    .company-name {{ font-size: 28px; font-weight: 700; color: #1a73e8; }}
    .company-sub {{ font-size: 12px; color: #666; margin-top: 4px; }}
    .invoice-meta {{ text-align: right; }}
    .invoice-meta h2 {{ font-size: 22px; color: #333; }}
    .invoice-meta p {{ font-size: 13px; color: #666; margin-top: 4px; }}
    .section {{ margin-bottom: 32px; }}
    .section-title {{ font-size: 11px; font-weight: 700; color: #999; text-transform: uppercase;
                      letter-spacing: 0.1em; margin-bottom: 8px; }}
    .bill-to {{ background: #f8f9fa; padding: 16px 20px; border-radius: 4px; border-left: 3px solid #1a73e8; }}
    .bill-to p {{ font-size: 14px; line-height: 1.6; color: #333; }}
    table {{ width: 100%; border-collapse: collapse; margin-bottom: 0; }}
    thead tr {{ background: #1a73e8; color: white; }}
    thead th {{ padding: 12px 16px; text-align: left; font-size: 13px; font-weight: 600; }}
    tbody tr {{ border-bottom: 1px solid #e8ecf0; }}
    tbody tr:hover {{ background: #f8f9fa; }}
    tbody td {{ padding: 14px 16px; font-size: 14px; color: #333; }}
    .text-right {{ text-align: right; }}
    .totals {{ margin-top: 0; border-top: 2px solid #e8ecf0; }}
    .totals tr td {{ padding: 10px 16px; font-size: 14px; }}
    .totals .total-row td {{ font-size: 16px; font-weight: 700; color: #1a73e8;
                              border-top: 2px solid #1a73e8; padding-top: 14px; }}
    .badge {{ display: inline-block; padding: 4px 10px; border-radius: 20px; font-size: 12px;
              font-weight: 600; }}
    .badge-confirmed {{ background: #e8f5e9; color: #2e7d32; }}
    .footer {{ margin-top: 48px; padding-top: 24px; border-top: 1px solid #e8ecf0;
               font-size: 11px; color: #999; text-align: center; line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="page">

    <!-- Header -->
    <div class="header">
      <div>
        <div class="company-name">{company}</div>
        <div class="company-sub">Parkverwaltungssystem</div>
      </div>
      <div class="invoice-meta">
        <h2>RECHNUNG</h2>
        <p><strong>{invoice_number}</strong></p>
        <p>Datum: {invoice_date}</p>
      </div>
    </div>

    <!-- Bill To -->
    <div class="section">
      <div class="section-title">Rechnungsempfänger</div>
      <div class="bill-to">
        <p><strong>{user_name}</strong></p>
        <p>{user_email}</p>
      </div>
    </div>

    <!-- Booking Details -->
    <div class="section">
      <div class="section-title">Buchungsdetails</div>
      <table>
        <thead>
          <tr>
            <th>Beschreibung</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Buchungsnummer</td>
            <td>{booking_id}</td>
          </tr>
          <tr>
            <td>Parkhaus</td>
            <td>{lot_name}</td>
          </tr>
          <tr>
            <td>Stellplatz</td>
            <td>Nr. {slot_number} &nbsp;·&nbsp; {floor_name}</td>
          </tr>
          <tr>
            <td>Fahrzeug (Kennzeichen)</td>
            <td>{license_plate}</td>
          </tr>
          <tr>
            <td>Beginn</td>
            <td>{start_str}</td>
          </tr>
          <tr>
            <td>Ende</td>
            <td>{end_str}</td>
          </tr>
          <tr>
            <td>Dauer</td>
            <td>{duration_hours} Std. {duration_mins_part} Min.</td>
          </tr>
          <tr>
            <td>Status</td>
            <td><span class="badge badge-confirmed">{status}</span></td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pricing -->
    <div class="section">
      <div class="section-title">Rechnungsbetrag</div>
      <table>
        <thead>
          <tr>
            <th>Position</th>
            <th class="text-right">Betrag ({currency})</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Parkgebühr (Netto)</td>
            <td class="text-right">{net_price:.2}</td>
          </tr>
        </tbody>
        <tbody class="totals">
          <tr>
            <td>Zwischensumme (Netto)</td>
            <td class="text-right">{net_price:.2}</td>
          </tr>
          <tr>
            <td>MwSt. 19% (§ 12 UStG)</td>
            <td class="text-right">{vat_amount:.2}</td>
          </tr>
          <tr class="total-row">
            <td>Gesamtbetrag (Brutto)</td>
            <td class="text-right">{gross_total:.2}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Footer -->
    <div class="footer">
      <p>{company} · Parkverwaltungssystem · Automatisch generierte Rechnung</p>
      <p>Diese Rechnung wurde automatisch erstellt und ist ohne Unterschrift gültig.</p>
    </div>

  </div>
</body>
</html>"#,
        invoice_number = invoice_number,
        invoice_date = invoice_date,
        company = company,
        user_name = user_name,
        user_email = user_email,
        booking_id = booking.id,
        lot_name = lot_name,
        slot_number = booking.slot_number,
        floor_name = floor_name,
        license_plate = license_plate,
        start_str = start_str,
        end_str = end_str,
        duration_hours = duration_hours,
        duration_mins_part = duration_mins_part,
        status = format!("{:?}", booking.status),
        currency = booking.pricing.currency,
        net_price = net_price,
        vat_amount = vat_amount,
        gross_total = gross_total,
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUICK BOOK
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for quick booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct QuickBookRequest {
    lot_id: Uuid,
    date: Option<String>,
    booking_type: Option<String>,
}

/// `POST /api/v1/bookings/quick` — quick book with auto-assigned slot
#[utoipa::path(post, path = "/api/v1/bookings/quick", tag = "Bookings",
    summary = "Quick book (auto-assign slot)",
    description = "Auto-picks an available slot and creates a booking.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, lot_id = %req.lot_id))]
pub async fn quick_book(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<QuickBookRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.write().await;

    // Find first available slot in the lot
    let slots = match state_guard
        .db
        .list_slots_by_lot(&req.lot_id.to_string())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list slots: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list slots")),
            );
        }
    };

    let available_slot = match slots.iter().find(|s| s.status == SlotStatus::Available) {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::error(
                    "NO_SLOTS_AVAILABLE",
                    "No available slots in this lot",
                )),
            );
        }
    };

    // Get user's default vehicle (or first vehicle)
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
            license_plate: String::new(),
            make: None,
            model: None,
            color: None,
            vehicle_type: VehicleType::Car,
            is_default: false,
            created_at: Utc::now(),
        });

    // Determine booking times based on type
    let booking_type = req.booking_type.as_deref().unwrap_or("full_day");
    let now = Utc::now();
    let (start_time, end_time) = match booking_type {
        "half_day_am" | "half_day_pm" => {
            let start = now + TimeDelta::minutes(1);
            let end = start + TimeDelta::hours(4);
            (start, end)
        }
        _ => {
            // full_day default: 8 hours
            let start = now + TimeDelta::minutes(1);
            let end = start + TimeDelta::hours(8);
            (start, end)
        }
    };

    // Look up floor name and pricing from the lot
    let lot_opt = state_guard
        .db
        .get_parking_lot(&req.lot_id.to_string())
        .await
        .ok()
        .flatten();

    let floor_name = lot_opt.as_ref().map_or_else(
        || "Level 1".to_string(),
        |lot| {
            lot.floors
                .iter()
                .find(|f| f.id == available_slot.floor_id)
                .map_or_else(|| "Level 1".to_string(), |f| f.name.clone())
        },
    );

    let hourly_rate = lot_opt
        .as_ref()
        .and_then(|lot| lot.pricing.rates.iter().find(|r| r.duration_minutes == 60))
        .map_or(2.0, |r| r.price);
    let daily_max_gs = lot_opt.as_ref().and_then(|lot| lot.pricing.daily_max);
    let lot_currency_gs = lot_opt
        .as_ref()
        .map_or_else(|| "EUR".to_string(), |lot| lot.pricing.currency.clone());

    #[allow(clippy::cast_precision_loss)]
    let raw_price_gs = ((end_time - start_time).num_minutes() as f64 / 60.0) * hourly_rate;
    let base_price = daily_max_gs.map_or(raw_price_gs, |cap| raw_price_gs.min(cap));
    let tax = base_price * VAT_RATE;
    let total = base_price + tax;

    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: available_slot.id,
        slot_number: available_slot.slot_number,
        floor_name,
        vehicle,
        start_time,
        end_time,
        status: BookingStatus::Confirmed,
        pricing: BookingPricing {
            base_price,
            discount: 0.0,
            tax,
            total,
            currency: lot_currency_gs,
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: Some(Uuid::new_v4().to_string()),
        notes: Some(format!("Quick book ({booking_type})")),
        tenant_id: None,
    };

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save quick booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create booking",
            )),
        );
    }

    // Update slot status — fail the booking if slot update fails to prevent double-booking
    let mut updated_slot = available_slot;
    updated_slot.status = SlotStatus::Reserved;
    if let Err(e) = state_guard.db.save_parking_slot(&updated_slot).await {
        tracing::error!("Failed to update slot status after quick booking: {}", e);
        // Roll back the booking to avoid inconsistent state
        let _ = state_guard.db.delete_booking(&booking.id.to_string()).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SLOT_UPDATE_FAILED",
                "Failed to reserve slot",
            )),
        );
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %booking.id,
        slot_id = %booking.slot_id,
        "Quick booking created"
    );

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING CHECKIN
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/bookings/{id}/checkin` — mark booking as checked in
#[utoipa::path(post, path = "/api/v1/bookings/{id}/checkin", tag = "Bookings",
    summary = "Check in to a booking",
    description = "Marks a booking as checked-in.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn booking_checkin(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.write().await;

    let mut booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
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

    // Only booking owner or admin can check in
    if booking.user_id != auth_user.user_id
        && let Err((status, msg)) = check_admin(&state_guard, &auth_user).await
    {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Only Confirmed or Pending bookings can be checked in
    if booking.status != BookingStatus::Confirmed && booking.status != BookingStatus::Pending {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "INVALID_STATUS",
                "Only confirmed or pending bookings can be checked in",
            )),
        );
    }

    booking.status = BookingStatus::Active;
    booking.check_in_time = Some(Utc::now());
    booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save booking checkin: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to check in booking",
            )),
        );
    }

    AuditEntry::new(AuditEventType::BookingUpdated)
        .user(auth_user.user_id, "")
        .resource("booking", &id)
        .details(serde_json::json!({"action": "checkin"}))
        .log();

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

/// Request body for patching a booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub struct PatchBookingRequest {
    pub notes: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// `PATCH /api/v1/bookings/{id}` — update notes/times on an existing booking
#[utoipa::path(
    patch,
    path = "/api/v1/bookings/{id}",
    tag = "Bookings",
    summary = "Update a booking",
    description = "Update notes and/or times on a booking. Only the booking owner or an admin may update.",
    security(("bearer_auth" = []))
)]
#[cfg_attr(not(feature = "mod-bookings"), allow(dead_code))]
pub async fn update_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<PatchBookingRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.read().await;

    let mut booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Check ownership or admin
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
    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    if let Some(notes) = req.notes {
        booking.notes = Some(notes);
    }
    if let Some(start_time) = req.start_time {
        booking.start_time = start_time;
    }
    if let Some(end_time) = req.end_time {
        booking.end_time = end_time;
    }
    booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to update booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update booking",
            )),
        );
    }

    AuditEntry::new(AuditEventType::BookingUpdated)
        .user(auth_user.user_id, &caller.username)
        .resource("booking", &id)
        .details(serde_json::json!({"action": "patch"}))
        .log();

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

#[cfg(test)]
mod tests {
    use parkhub_common::{
        Booking, BookingPricing, BookingStatus, GuestBooking, PaymentStatus, Vehicle, VehicleType,
    };
    use uuid::Uuid;

    fn make_vehicle() -> Vehicle {
        Vehicle {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            license_plate: "AB-CD-1234".to_string(),
            make: Some("BMW".to_string()),
            model: Some("X5".to_string()),
            color: Some("Black".to_string()),
            vehicle_type: VehicleType::Car,
            is_default: true,
            created_at: chrono::Utc::now(),
        }
    }

    fn make_pricing() -> BookingPricing {
        BookingPricing {
            base_price: 5.0,
            discount: 0.0,
            tax: 0.5,
            total: 5.5,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        }
    }

    // ── BookingStatus serde ──────────────────────────────────────────────────

    #[test]
    fn test_booking_status_serde_all_variants() {
        let cases = [
            (BookingStatus::Pending, "\"pending\""),
            (BookingStatus::Confirmed, "\"confirmed\""),
            (BookingStatus::Active, "\"active\""),
            (BookingStatus::Completed, "\"completed\""),
            (BookingStatus::Cancelled, "\"cancelled\""),
            (BookingStatus::Expired, "\"expired\""),
            (BookingStatus::NoShow, "\"no_show\""),
        ];
        for (variant, expected_json) in &cases {
            let serialized = serde_json::to_string(variant).unwrap();
            assert_eq!(&serialized, expected_json, "Variant {:?} failed", variant);
            let deserialized: BookingStatus = serde_json::from_str(expected_json).unwrap();
            assert_eq!(&deserialized, variant);
        }
    }

    #[test]
    fn test_booking_status_unknown_fails() {
        let result: Result<BookingStatus, _> = serde_json::from_str(r#""unknown_status""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_booking_status_default_is_pending() {
        let status = BookingStatus::default();
        assert_eq!(status, BookingStatus::Pending);
    }

    // ── PaymentStatus serde ──────────────────────────────────────────────────

    #[test]
    fn test_payment_status_serde() {
        let pending: PaymentStatus = serde_json::from_str(r#""pending""#).unwrap();
        assert_eq!(pending, PaymentStatus::Pending);

        let paid: PaymentStatus = serde_json::from_str(r#""paid""#).unwrap();
        assert_eq!(paid, PaymentStatus::Paid);

        let refunded: PaymentStatus = serde_json::from_str(r#""refunded""#).unwrap();
        assert_eq!(refunded, PaymentStatus::Refunded);
    }

    #[test]
    fn test_payment_status_default_is_pending() {
        assert_eq!(PaymentStatus::default(), PaymentStatus::Pending);
    }

    // ── BookingPricing serde ─────────────────────────────────────────────────

    #[test]
    fn test_booking_pricing_serde_roundtrip() {
        let pricing = make_pricing();
        let json = serde_json::to_string(&pricing).unwrap();
        let back: BookingPricing = serde_json::from_str(&json).unwrap();
        assert!((back.base_price - 5.0).abs() < 1e-9);
        assert!((back.total - 5.5).abs() < 1e-9);
        assert_eq!(back.currency, "EUR");
        assert!(back.payment_method.is_none());
    }

    #[test]
    fn test_booking_pricing_zero_discount() {
        let json = serde_json::json!({
            "base_price": 10.0,
            "discount": 0.0,
            "tax": 1.0,
            "total": 11.0,
            "currency": "USD",
            "payment_status": "pending",
            "payment_method": null
        });
        let pricing: BookingPricing = serde_json::from_value(json).unwrap();
        assert_eq!(pricing.discount, 0.0);
        assert_eq!(pricing.total, 11.0);
    }

    #[test]
    fn test_booking_pricing_with_payment_method() {
        let json = serde_json::json!({
            "base_price": 8.0,
            "discount": 1.0,
            "tax": 0.7,
            "total": 7.7,
            "currency": "EUR",
            "payment_status": "paid",
            "payment_method": "credit_card"
        });
        let pricing: BookingPricing = serde_json::from_value(json).unwrap();
        assert_eq!(pricing.payment_method.as_deref(), Some("credit_card"));
        assert_eq!(pricing.payment_status, PaymentStatus::Paid);
    }

    // ── Booking full model serde ─────────────────────────────────────────────

    #[test]
    fn test_booking_serde_roundtrip() {
        let now = chrono::Utc::now();
        let booking = Booking {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            slot_number: 42,
            floor_name: "Ground Floor".to_string(),
            vehicle: make_vehicle(),
            start_time: now,
            end_time: now + chrono::Duration::hours(2),
            status: BookingStatus::Confirmed,
            pricing: make_pricing(),
            created_at: now,
            updated_at: now,
            check_in_time: None,
            check_out_time: None,
            qr_code: Some("QR_DATA".to_string()),
            notes: None,
            tenant_id: None,
        };

        let json = serde_json::to_string(&booking).unwrap();
        let back: Booking = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, booking.id);
        assert_eq!(back.slot_number, 42);
        assert_eq!(back.status, BookingStatus::Confirmed);
        assert!(back.check_in_time.is_none());
        assert!(back.notes.is_none());
        assert_eq!(back.qr_code.as_deref(), Some("QR_DATA"));
    }

    #[test]
    fn test_booking_with_check_in_out_times() {
        let now = chrono::Utc::now();
        let booking = Booking {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            slot_number: 1,
            floor_name: "Level 1".to_string(),
            vehicle: make_vehicle(),
            start_time: now,
            end_time: now + chrono::Duration::hours(1),
            status: BookingStatus::Completed,
            pricing: make_pricing(),
            created_at: now,
            updated_at: now,
            check_in_time: Some(now + chrono::Duration::minutes(5)),
            check_out_time: Some(now + chrono::Duration::minutes(65)),
            qr_code: None,
            notes: Some("late arrival".to_string()),
            tenant_id: None,
        };

        let json = serde_json::to_string(&booking).unwrap();
        let back: Booking = serde_json::from_str(&json).unwrap();
        assert!(back.check_in_time.is_some());
        assert!(back.check_out_time.is_some());
        assert_eq!(back.notes.as_deref(), Some("late arrival"));
        assert_eq!(back.status, BookingStatus::Completed);
    }

    // ── GuestBooking serde ───────────────────────────────────────────────────

    #[test]
    fn test_guest_booking_serde_roundtrip() {
        let now = chrono::Utc::now();
        let guest = GuestBooking {
            id: Uuid::new_v4(),
            created_by: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            guest_name: "Max Muster".to_string(),
            guest_email: Some("max@example.com".to_string()),
            vehicle_plate: None,
            start_time: now,
            end_time: now + chrono::Duration::hours(3),
            guest_code: "ABCD1234".to_string(),
            status: BookingStatus::Confirmed,
            created_at: now,
        };

        let json = serde_json::to_string(&guest).unwrap();
        let back: GuestBooking = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, guest.id);
        assert_eq!(back.guest_name, "Max Muster");
        assert_eq!(back.guest_code, "ABCD1234");
        assert_eq!(back.guest_email.as_deref(), Some("max@example.com"));
        assert!(back.vehicle_plate.is_none());
        assert_eq!(back.status, BookingStatus::Confirmed);
    }

    #[test]
    fn test_guest_booking_no_email() {
        let now = chrono::Utc::now();
        let guest = GuestBooking {
            id: Uuid::new_v4(),
            created_by: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            guest_name: "Anonymous".to_string(),
            guest_email: None,
            vehicle_plate: Some("MUC-AB-123".to_string()),
            start_time: now,
            end_time: now + chrono::Duration::hours(1),
            guest_code: "ZZZZZZZZ".to_string(),
            status: BookingStatus::Pending,
            created_at: now,
        };

        let json = serde_json::to_string(&guest).unwrap();
        let back: GuestBooking = serde_json::from_str(&json).unwrap();
        assert!(back.guest_email.is_none());
        assert_eq!(back.vehicle_plate.as_deref(), Some("MUC-AB-123"));
        assert_eq!(back.status, BookingStatus::Pending);
    }
}
