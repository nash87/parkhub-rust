//! Parking lot handlers: CRUD operations and slot listing.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use parkhub_common::models::{SlotFeature, SlotPosition, SlotType};
use parkhub_common::{
    ApiResponse, LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo,
    PricingRate, SlotStatus,
};

use crate::requests::{parse_lot_status, CreateParkingLotRequest, UpdateParkingLotRequest};

use super::{AuthUser, SharedState};
use parkhub_common::UserRole;

// ─────────────────────────────────────────────────────────────────────────────
// Query params
// ─────────────────────────────────────────────────────────────────────────────

/// Optional filters for `GET /api/v1/lots/{id}/slots`.
#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
pub struct SlotFilterParams {
    /// Filter by slot type: `standard`, `compact`, `large`, `handicap`,
    /// `electric`, `motorcycle`, `reserved`, `vip`
    pub slot_type: Option<String>,
    /// Filter by slot status: `available`, `occupied`, `reserved`,
    /// `maintenance`, `disabled`
    pub status: Option<String>,
    /// Filter by feature: `near_exit`, `near_elevator`, `near_stairs`,
    /// `covered`, `security_camera`, `well_lit`, `wide_lane`, `charging_station`
    pub feature: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn parse_slot_type(s: &str) -> Option<SlotType> {
    match s.to_lowercase().as_str() {
        "standard" => Some(SlotType::Standard),
        "compact" => Some(SlotType::Compact),
        "large" => Some(SlotType::Large),
        "handicap" => Some(SlotType::Handicap),
        "electric" => Some(SlotType::Electric),
        "motorcycle" => Some(SlotType::Motorcycle),
        "reserved" => Some(SlotType::Reserved),
        "vip" => Some(SlotType::Vip),
        _ => None,
    }
}

fn parse_slot_status(s: &str) -> Option<SlotStatus> {
    match s.to_lowercase().as_str() {
        "available" => Some(SlotStatus::Available),
        "occupied" => Some(SlotStatus::Occupied),
        "reserved" => Some(SlotStatus::Reserved),
        "maintenance" => Some(SlotStatus::Maintenance),
        "disabled" => Some(SlotStatus::Disabled),
        _ => None,
    }
}

fn parse_slot_feature(s: &str) -> Option<SlotFeature> {
    match s.to_lowercase().as_str() {
        "near_exit" => Some(SlotFeature::NearExit),
        "near_elevator" => Some(SlotFeature::NearElevator),
        "near_stairs" => Some(SlotFeature::NearStairs),
        "covered" => Some(SlotFeature::Covered),
        "security_camera" => Some(SlotFeature::SecurityCamera),
        "well_lit" => Some(SlotFeature::WellLit),
        "wide_lane" => Some(SlotFeature::WideLane),
        "charging_station" => Some(SlotFeature::ChargingStation),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/lots",
    tag = "Lots",
    summary = "List all parking lots",
    description = "Returns all parking lots with their configuration and status.",
    responses(
        (status = 200, description = "List of all parking lots"),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn list_lots(State(state): State<SharedState>) -> Json<ApiResponse<Vec<ParkingLot>>> {
    let state = state.read().await;

    match state.db.list_parking_lots().await {
        Ok(lots) => {
            tracing::debug!(count = lots.len(), "Listed parking lots");
            Json(ApiResponse::success(lots))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list parking lots");
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list parking lots",
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/lots",
    tag = "Lots",
    summary = "Create a parking lot",
    description = "Create a new parking lot with auto-generated slots. Admin only.",
    request_body = CreateParkingLotRequest,
    responses(
        (status = 201, description = "Parking lot created"),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Admin access required"),
    )
)]
#[tracing::instrument(skip(state, req), fields(admin_id = %auth_user.user_id, lot_name = %req.name))]
#[allow(clippy::too_many_lines)]
pub async fn create_lot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateParkingLotRequest>,
) -> (StatusCode, Json<ApiResponse<ParkingLot>>) {
    // Validate the request DTO
    if let Err(errors) = req.validate() {
        let msg = errors
            .field_errors()
            .values()
            .flat_map(|errs| errs.iter().filter_map(|e| e.message.as_deref()))
            .collect::<Vec<_>>()
            .join("; ");
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                if msg.is_empty() {
                    "Invalid request"
                } else {
                    &msg
                },
            )),
        );
    }

    let state_guard = state.read().await;

    // Check if user is admin
    let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let now = Utc::now();
    let lot_id = Uuid::new_v4();

    // Build pricing from request fields
    let mut rates = Vec::new();
    if let Some(hourly) = req.hourly_rate {
        rates.push(PricingRate {
            duration_minutes: 60,
            price: hourly,
            label: "1 hour".to_string(),
        });
    }

    let pricing = PricingInfo {
        currency: req.currency.clone(),
        rates,
        daily_max: req.daily_max,
        monthly_pass: req.monthly_pass,
    };

    // Default to 24h operation
    let operating_hours = OperatingHours {
        is_24h: true,
        monday: None,
        tuesday: None,
        wednesday: None,
        thursday: None,
        friday: None,
        saturday: None,
        sunday: None,
    };

    // Create a default floor for the auto-generated slots
    let floor_id = Uuid::new_v4();
    let default_floor = ParkingFloor {
        id: floor_id,
        lot_id,
        name: "Ground Floor".to_string(),
        floor_number: 1,
        total_slots: req.total_slots,
        available_slots: req.total_slots,
        slots: Vec::new(),
    };

    // Build the ParkingLot
    let lot = ParkingLot {
        id: lot_id,
        name: req.name,
        address: req.address.unwrap_or_default(),
        latitude: req.latitude.unwrap_or(0.0),
        longitude: req.longitude.unwrap_or(0.0),
        total_slots: req.total_slots,
        available_slots: req.total_slots,
        floors: vec![default_floor],
        amenities: Vec::new(),
        pricing,
        operating_hours,
        images: Vec::new(),
        status: req
            .status
            .as_deref()
            .and_then(parse_lot_status)
            .unwrap_or(LotStatus::Open),
        created_at: now,
        updated_at: now,
    };

    // Persist the lot
    if let Err(e) = state_guard.db.save_parking_lot(&lot).await {
        tracing::error!("Failed to save parking lot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create parking lot",
            )),
        );
    }

    // Auto-generate parking slots in a single batch transaction
    let slots: Vec<ParkingSlot> = (1..=req.total_slots)
        .map(|i| ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: i,
            row: ((i - 1) / 10) + 1,
            column: ((i - 1) % 10) + 1,
            slot_type: SlotType::Standard,
            status: SlotStatus::Available,
            current_booking: None,
            features: Vec::new(),
            position: SlotPosition {
                #[allow(clippy::cast_precision_loss)]
                x: (((i - 1) % 10) as f32) * 3.0,
                #[allow(clippy::cast_precision_loss)]
                y: (((i - 1) / 10) as f32) * 5.0,
                width: 2.5,
                height: 5.0,
                rotation: 0.0,
            },
        })
        .collect();

    if let Err(e) = state_guard.db.save_parking_slots_batch(&slots).await {
        tracing::error!("Failed to batch-save parking slots: {}", e);
    }
    drop(state_guard);

    tracing::info!(
        "Created parking lot '{}' ({}) with {} slots",
        lot.name,
        lot.id,
        req.total_slots,
    );

    // Dispatch webhook
    {
        let state_clone = state.clone();
        let lot_json = serde_json::json!({
            "lot_id": lot.id,
            "name": lot.name,
            "total_slots": lot.total_slots,
        });
        tokio::spawn(async move {
            super::webhooks::dispatch_webhook_event(&state_clone, "lot.created", lot_json).await;
        });
    }

    (StatusCode::CREATED, Json(ApiResponse::success(lot)))
}

#[utoipa::path(
    put,
    path = "/api/v1/lots/{id}",
    tag = "Lots",
    summary = "Update a parking lot",
    description = "Update parking lot properties. Admin only.",
    params(("id" = String, Path, description = "Parking lot ID")),
    request_body = UpdateParkingLotRequest,
    responses(
        (status = 200, description = "Parking lot updated"),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
#[allow(clippy::too_many_lines)]
pub async fn update_lot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateParkingLotRequest>,
) -> (StatusCode, Json<ApiResponse<ParkingLot>>) {
    // Validate the request DTO
    if let Err(errors) = req.validate() {
        let msg = errors
            .field_errors()
            .values()
            .flat_map(|errs| errs.iter().filter_map(|e| e.message.as_deref()))
            .collect::<Vec<_>>()
            .join("; ");
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                if msg.is_empty() {
                    "Invalid request"
                } else {
                    &msg
                },
            )),
        );
    }

    let state_guard = state.read().await;

    // Check if user is admin
    let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    // Fetch existing lot
    let mut lot = match state_guard.db.get_parking_lot(&id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
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

    // Apply partial updates
    if let Some(name) = req.name {
        lot.name = name;
    }
    if let Some(address) = req.address {
        lot.address = address;
    }
    if let Some(lat) = req.latitude {
        lot.latitude = lat;
    }
    if let Some(lng) = req.longitude {
        lot.longitude = lng;
    }
    if let Some(total_slots) = req.total_slots {
        lot.total_slots = total_slots;
    }
    if let Some(ref status_str) = req.status {
        if let Some(parsed) = parse_lot_status(status_str) {
            lot.status = parsed;
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "Invalid status. Valid: open, closed, full, maintenance",
                )),
            );
        }
    }

    // Update pricing fields
    if let Some(hourly_rate) = req.hourly_rate {
        if let Some(rate) = lot
            .pricing
            .rates
            .iter_mut()
            .find(|r| r.duration_minutes == 60)
        {
            rate.price = hourly_rate;
        } else {
            lot.pricing.rates.push(PricingRate {
                duration_minutes: 60,
                price: hourly_rate,
                label: "1 hour".to_string(),
            });
        }
    }
    if let Some(daily_max) = req.daily_max {
        lot.pricing.daily_max = Some(daily_max);
    }
    if let Some(monthly_pass) = req.monthly_pass {
        lot.pricing.monthly_pass = Some(monthly_pass);
    }
    if let Some(currency) = req.currency {
        lot.pricing.currency = currency;
    }

    lot.updated_at = Utc::now();

    // Persist
    if let Err(e) = state_guard.db.save_parking_lot(&lot).await {
        tracing::error!("Failed to update parking lot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update parking lot",
            )),
        );
    }
    drop(state_guard);

    tracing::info!("Updated parking lot '{}' ({})", lot.name, lot.id);

    (StatusCode::OK, Json(ApiResponse::success(lot)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/lots/{id}",
    tag = "Lots",
    summary = "Delete a parking lot",
    description = "Permanently remove a parking lot and all its slots. Admin only.",
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Parking lot deleted"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn delete_lot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Check if user is admin
    let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let result = state_guard.db.delete_parking_lot(&id).await;
    match result {
        Ok(true) => {
            // Cascade-delete orphaned slots belonging to this lot
            if let Err(e) = state_guard.db.delete_slots_by_lot(&id).await {
                tracing::error!("Failed to cascade-delete slots for lot {}: {}", id, e);
            }
            drop(state_guard);
            tracing::info!("Deleted parking lot: {}", id);
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete parking lot: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete parking lot",
                )),
            )
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}",
    tag = "Lots",
    summary = "Get parking lot details",
    description = "Returns full details of a single parking lot.",
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Parking lot details"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn get_lot(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ParkingLot>>) {
    let state = state.read().await;

    match state.db.get_parking_lot(&id).await {
        Ok(Some(lot)) => (StatusCode::OK, Json(ApiResponse::success(lot))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
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

#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}/slots",
    tag = "Lots",
    summary = "List slots in a parking lot",
    description = "Returns parking slots in the specified lot. Optionally filter by \
        `slot_type` (standard, compact, large, handicap, electric, motorcycle, reserved, vip), \
        `status` (available, occupied, reserved, maintenance, disabled), or \
        `feature` (near_exit, near_elevator, near_stairs, covered, security_camera, \
        well_lit, wide_lane, charging_station).",
    params(
        ("id" = String, Path, description = "Parking lot ID"),
        SlotFilterParams,
    ),
    responses(
        (status = 200, description = "List of slots in the parking lot"),
        (status = 400, description = "Invalid filter value"),
    )
)]
pub async fn get_lot_slots(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(filters): Query<SlotFilterParams>,
) -> (StatusCode, Json<ApiResponse<Vec<ParkingSlot>>>) {
    // Validate filter params upfront so we can return 400 on unknown values
    let type_filter = if let Some(ref t) = filters.slot_type {
        match parse_slot_type(t) {
            Some(v) => Some(v),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "VALIDATION_ERROR",
                        "Invalid slot_type. Valid: standard, compact, large, handicap, electric, motorcycle, reserved, vip",
                    )),
                );
            }
        }
    } else {
        None
    };

    let status_filter = if let Some(ref s) = filters.status {
        match parse_slot_status(s) {
            Some(v) => Some(v),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "VALIDATION_ERROR",
                        "Invalid status. Valid: available, occupied, reserved, maintenance, disabled",
                    )),
                );
            }
        }
    } else {
        None
    };

    let feature_filter = if let Some(ref f) = filters.feature {
        match parse_slot_feature(f) {
            Some(v) => Some(v),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "VALIDATION_ERROR",
                        "Invalid feature. Valid: near_exit, near_elevator, near_stairs, covered, security_camera, well_lit, wide_lane, charging_station",
                    )),
                );
            }
        }
    } else {
        None
    };

    let state = state.read().await;

    let slots = match state.db.list_slots_by_lot(&id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list slots")),
            );
        }
    };

    // Apply in-memory filters
    let filtered: Vec<ParkingSlot> = slots
        .into_iter()
        .filter(|s| type_filter.as_ref().map_or(true, |t| &s.slot_type == t))
        .filter(|s| status_filter.as_ref().map_or(true, |st| &s.status == st))
        .filter(|s| {
            feature_filter
                .as_ref()
                .map_or(true, |f| s.features.contains(f))
        })
        .collect();

    tracing::debug!(
        lot_id = %id,
        total = filtered.len(),
        slot_type = ?filters.slot_type,
        status = ?filters.status,
        feature = ?filters.feature,
        "Listed slots with filters"
    );

    (StatusCode::OK, Json(ApiResponse::success(filtered)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Slot CRUD (admin only)
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/lots/{lot_id}/slots` — create a new slot in a lot
#[utoipa::path(
    post,
    path = "/api/v1/lots/{lot_id}/slots",
    tag = "Lots",
    summary = "Create a parking slot",
    description = "Add a new slot to a parking lot. Admin only.",
    params(("lot_id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 201, description = "Slot created"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn create_slot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<ParkingSlot>>) {
    let state_guard = state.read().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    // Verify lot exists
    let lot = match state_guard.db.get_parking_lot(&lot_id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
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

    let floor_id = lot.floors.first().map_or_else(Uuid::new_v4, |f| f.id);
    let existing_slots = state_guard
        .db
        .list_slots_by_lot(&lot_id)
        .await
        .unwrap_or_default();
    let next_number = existing_slots
        .iter()
        .map(|s| s.slot_number)
        .max()
        .unwrap_or(0)
        + 1;

    let slot_type_str = req
        .get("slot_type")
        .and_then(|v| v.as_str())
        .unwrap_or("standard");
    let slot_type = match slot_type_str {
        "compact" => SlotType::Compact,
        "large" => SlotType::Large,
        "handicap" => SlotType::Handicap,
        "electric" => SlotType::Electric,
        "motorcycle" => SlotType::Motorcycle,
        "vip" => SlotType::Vip,
        _ => SlotType::Standard,
    };

    let raw_slot_number = req
        .get("slot_number")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(|| i64::from(next_number));
    #[allow(clippy::cast_possible_truncation)]
    let slot_number = raw_slot_number as i32;

    let slot = ParkingSlot {
        id: Uuid::new_v4(),
        lot_id: lot.id,
        floor_id,
        slot_number,
        row: ((next_number - 1) / 10) + 1,
        column: ((next_number - 1) % 10) + 1,
        slot_type,
        status: SlotStatus::Available,
        current_booking: None,
        features: Vec::new(),
        position: SlotPosition {
            #[allow(clippy::cast_precision_loss)]
            x: (((next_number - 1) % 10) as f32) * 3.0,
            #[allow(clippy::cast_precision_loss)]
            y: (((next_number - 1) / 10) as f32) * 5.0,
            width: 2.5,
            height: 5.0,
            rotation: 0.0,
        },
    };

    if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
        tracing::error!("Failed to save slot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create slot")),
        );
    }
    drop(state_guard);

    (StatusCode::CREATED, Json(ApiResponse::success(slot)))
}

/// `PUT /api/v1/lots/{lot_id}/slots/{slot_id}` — update a slot
#[utoipa::path(
    put,
    path = "/api/v1/lots/{lot_id}/slots/{slot_id}",
    tag = "Lots",
    summary = "Update a parking slot",
    description = "Update slot properties (status, type, label, etc.). Admin only.",
    params(
        ("lot_id" = String, Path, description = "Parking lot ID"),
        ("slot_id" = String, Path, description = "Slot ID"),
    ),
    responses(
        (status = 200, description = "Slot updated"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Slot not found"),
    )
)]
pub async fn update_slot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, slot_id)): Path<(String, String)>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<ParkingSlot>>) {
    let state_guard = state.read().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    let mut slot = match state_guard.db.get_parking_slot(&slot_id).await {
        Ok(Some(s)) if s.lot_id.to_string() == lot_id => s,
        Ok(Some(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "NOT_FOUND",
                    "Slot not found in this lot",
                )),
            );
        }
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

    // Update fields if provided
    if let Some(status) = req.get("status").and_then(|v| v.as_str()) {
        slot.status = match status {
            "available" => SlotStatus::Available,
            "occupied" => SlotStatus::Occupied,
            "reserved" => SlotStatus::Reserved,
            "maintenance" => SlotStatus::Maintenance,
            "disabled" => SlotStatus::Disabled,
            _ => slot.status,
        };
    }

    if let Some(slot_type) = req.get("slot_type").and_then(|v| v.as_str()) {
        slot.slot_type = match slot_type {
            "compact" => SlotType::Compact,
            "large" => SlotType::Large,
            "handicap" => SlotType::Handicap,
            "electric" => SlotType::Electric,
            "motorcycle" => SlotType::Motorcycle,
            "vip" => SlotType::Vip,
            _ => SlotType::Standard,
        };
    }

    if let Some(number) = req.get("slot_number").and_then(serde_json::Value::as_i64) {
        #[allow(clippy::cast_possible_truncation)]
        let num = number as i32;
        slot.slot_number = num;
    }

    if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
        tracing::error!("Failed to update slot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update slot")),
        );
    }
    drop(state_guard);

    (StatusCode::OK, Json(ApiResponse::success(slot)))
}

/// `DELETE /api/v1/lots/{lot_id}/slots/{slot_id}` — delete a slot
#[utoipa::path(
    delete,
    path = "/api/v1/lots/{lot_id}/slots/{slot_id}",
    tag = "Lots",
    summary = "Delete a parking slot",
    description = "Remove a slot from a parking lot. Admin only.",
    params(
        ("lot_id" = String, Path, description = "Parking lot ID"),
        ("slot_id" = String, Path, description = "Slot ID"),
    ),
    responses(
        (status = 200, description = "Slot deleted"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Slot not found"),
    )
)]
pub async fn delete_slot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, slot_id)): Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    // Verify slot belongs to lot
    match state_guard.db.get_parking_slot(&slot_id).await {
        Ok(Some(s)) if s.lot_id.to_string() == lot_id => {}
        Ok(Some(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "NOT_FOUND",
                    "Slot not found in this lot",
                )),
            );
        }
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
    }

    if let Err(e) = state_guard.db.delete_parking_slot(&slot_id).await {
        tracing::error!("Failed to delete slot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to delete slot")),
        );
    }
    drop(state_guard);

    (StatusCode::OK, Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use parkhub_common::models::{LotStatus, SlotFeature, SlotStatus, SlotType};
    use parkhub_common::{PricingInfo, PricingRate};

    use crate::requests::{parse_lot_status, CreateParkingLotRequest, UpdateParkingLotRequest};
    use validator::Validate;

    use super::{parse_slot_feature, parse_slot_status, parse_slot_type};

    // ── parse_slot_type ─────────────────────────────────────────────────────

    #[test]
    fn test_parse_slot_type_all_variants() {
        assert_eq!(parse_slot_type("standard"), Some(SlotType::Standard));
        assert_eq!(parse_slot_type("compact"), Some(SlotType::Compact));
        assert_eq!(parse_slot_type("large"), Some(SlotType::Large));
        assert_eq!(parse_slot_type("handicap"), Some(SlotType::Handicap));
        assert_eq!(parse_slot_type("electric"), Some(SlotType::Electric));
        assert_eq!(parse_slot_type("motorcycle"), Some(SlotType::Motorcycle));
        assert_eq!(parse_slot_type("reserved"), Some(SlotType::Reserved));
        assert_eq!(parse_slot_type("vip"), Some(SlotType::Vip));
        assert_eq!(parse_slot_type("unknown"), None);
        assert_eq!(parse_slot_type(""), None);
    }

    #[test]
    fn test_parse_slot_type_case_insensitive() {
        assert_eq!(parse_slot_type("ELECTRIC"), Some(SlotType::Electric));
        assert_eq!(parse_slot_type("Compact"), Some(SlotType::Compact));
    }

    // ── parse_slot_status ───────────────────────────────────────────────────

    #[test]
    fn test_parse_slot_status_all_variants() {
        assert_eq!(parse_slot_status("available"), Some(SlotStatus::Available));
        assert_eq!(parse_slot_status("occupied"), Some(SlotStatus::Occupied));
        assert_eq!(parse_slot_status("reserved"), Some(SlotStatus::Reserved));
        assert_eq!(parse_slot_status("maintenance"), Some(SlotStatus::Maintenance));
        assert_eq!(parse_slot_status("disabled"), Some(SlotStatus::Disabled));
        assert_eq!(parse_slot_status("unknown"), None);
    }

    #[test]
    fn test_parse_slot_status_case_insensitive() {
        assert_eq!(parse_slot_status("AVAILABLE"), Some(SlotStatus::Available));
        assert_eq!(parse_slot_status("Occupied"), Some(SlotStatus::Occupied));
    }

    // ── parse_slot_feature ──────────────────────────────────────────────────

    #[test]
    fn test_parse_slot_feature_all_variants() {
        assert_eq!(parse_slot_feature("near_exit"), Some(SlotFeature::NearExit));
        assert_eq!(parse_slot_feature("near_elevator"), Some(SlotFeature::NearElevator));
        assert_eq!(parse_slot_feature("near_stairs"), Some(SlotFeature::NearStairs));
        assert_eq!(parse_slot_feature("covered"), Some(SlotFeature::Covered));
        assert_eq!(parse_slot_feature("security_camera"), Some(SlotFeature::SecurityCamera));
        assert_eq!(parse_slot_feature("well_lit"), Some(SlotFeature::WellLit));
        assert_eq!(parse_slot_feature("wide_lane"), Some(SlotFeature::WideLane));
        assert_eq!(parse_slot_feature("charging_station"), Some(SlotFeature::ChargingStation));
        assert_eq!(parse_slot_feature("unknown"), None);
    }

    // ── parse_lot_status ────────────────────────────────────────────────────

    #[test]
    fn test_parse_lot_status_all_variants() {
        assert_eq!(parse_lot_status("open"), Some(LotStatus::Open));
        assert_eq!(parse_lot_status("closed"), Some(LotStatus::Closed));
        assert_eq!(parse_lot_status("full"), Some(LotStatus::Full));
        assert_eq!(
            parse_lot_status("maintenance"),
            Some(LotStatus::Maintenance)
        );
    }

    #[test]
    fn test_parse_lot_status_case_sensitive() {
        assert!(parse_lot_status("Open").is_none());
        assert!(parse_lot_status("OPEN").is_none());
        assert!(parse_lot_status("Full").is_none());
        assert!(parse_lot_status("MAINTENANCE").is_none());
    }

    #[test]
    fn test_parse_lot_status_empty_and_whitespace() {
        assert!(parse_lot_status("").is_none());
        assert!(parse_lot_status(" open").is_none());
        assert!(parse_lot_status("open ").is_none());
        assert!(parse_lot_status(" ").is_none());
    }

    #[test]
    fn test_parse_lot_status_garbage_values() {
        assert!(parse_lot_status("unknown").is_none());
        assert!(parse_lot_status("123").is_none());
        assert!(parse_lot_status("open;closed").is_none());
    }

    // ── CreateParkingLotRequest: serde edge cases ───────────────────────────

    #[test]
    fn test_create_lot_request_missing_name_fails() {
        let json = r#"{"total_slots": 10}"#;
        let result: Result<CreateParkingLotRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_lot_request_zero_hourly_rate() {
        let json = r#"{"name": "Free Lot", "hourly_rate": 0.0}"#;
        let req: CreateParkingLotRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.hourly_rate, Some(0.0));
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_lot_request_max_hourly_rate() {
        let json = r#"{"name": "VIP Lot", "hourly_rate": 1000.0}"#;
        let req: CreateParkingLotRequest = serde_json::from_str(json).unwrap();
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_lot_request_hourly_rate_over_max() {
        let req = CreateParkingLotRequest {
            name: "X".to_string(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 1,
            hourly_rate: Some(1001.0),
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_negative_rates_fail_validation() {
        let req_daily_max = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 5,
            hourly_rate: None,
            daily_max: Some(-1.0),
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req_daily_max.validate().is_err());

        let req_monthly = CreateParkingLotRequest {
            name: "Lot".to_string(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 5,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: Some(-50.0),
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req_monthly.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_name_exact_min_length() {
        let req = CreateParkingLotRequest {
            name: "A".to_string(), // exactly 1 char — min boundary
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 1,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_lot_request_name_empty_fails() {
        let req = CreateParkingLotRequest {
            name: String::new(),
            address: None,
            latitude: None,
            longitude: None,
            total_slots: 1,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_lot_request_extreme_coords() {
        let south_pole = CreateParkingLotRequest {
            name: "South Pole".to_string(),
            address: None,
            latitude: Some(-90.0),
            longitude: Some(0.0),
            total_slots: 1,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(south_pole.validate().is_ok());

        let invalid_lat = CreateParkingLotRequest {
            name: "Beyond".to_string(),
            address: None,
            latitude: Some(-90.1),
            longitude: None,
            total_slots: 1,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: "EUR".to_string(),
            status: None,
        };
        assert!(invalid_lat.validate().is_err());
    }

    // ── UpdateParkingLotRequest: serde edge cases ───────────────────────────

    #[test]
    fn test_update_lot_request_currency_four_chars_fails() {
        let req = UpdateParkingLotRequest {
            name: None,
            address: None,
            latitude: None,
            longitude: None,
            total_slots: None,
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: Some("EURO".to_string()),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_lot_request_zero_slots_fails() {
        let req = UpdateParkingLotRequest {
            name: None,
            address: None,
            latitude: None,
            longitude: None,
            total_slots: Some(0),
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: None,
            status: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_lot_request_exactly_max_slots() {
        let req = UpdateParkingLotRequest {
            name: None,
            address: None,
            latitude: None,
            longitude: None,
            total_slots: Some(10000),
            hourly_rate: None,
            daily_max: None,
            monthly_pass: None,
            currency: None,
            status: None,
        };
        assert!(req.validate().is_ok());
    }

    // ── SlotType / SlotStatus model serde ────────────────────────────────────

    #[test]
    fn test_slot_type_serde_roundtrip() {
        let types = [
            SlotType::Standard,
            SlotType::Compact,
            SlotType::Large,
            SlotType::Handicap,
            SlotType::Electric,
            SlotType::Motorcycle,
            SlotType::Vip,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: SlotType = serde_json::from_str(&json).unwrap();
            // Both should serialize to the same JSON string
            assert_eq!(serde_json::to_string(&back).unwrap(), json);
        }
    }

    #[test]
    fn test_slot_status_serde_roundtrip() {
        let statuses = [
            SlotStatus::Available,
            SlotStatus::Occupied,
            SlotStatus::Reserved,
            SlotStatus::Maintenance,
            SlotStatus::Disabled,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: SlotStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(serde_json::to_string(&back).unwrap(), json);
        }
    }

    #[test]
    fn test_pricing_info_serde() {
        let pricing = PricingInfo {
            currency: "EUR".to_string(),
            rates: vec![PricingRate {
                duration_minutes: 60,
                price: 2.50,
                label: "1 hour".to_string(),
            }],
            daily_max: Some(20.0),
            monthly_pass: None,
        };

        let json = serde_json::to_string(&pricing).unwrap();
        let back: PricingInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(back.currency, "EUR");
        assert_eq!(back.rates.len(), 1);
        assert!((back.rates[0].price - 2.50).abs() < 1e-9);
        assert_eq!(back.daily_max, Some(20.0));
        assert!(back.monthly_pass.is_none());
    }
}
