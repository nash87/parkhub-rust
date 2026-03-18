//! Parking lot handlers: CRUD operations and slot listing.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use parkhub_common::models::{SlotPosition, SlotType};
use parkhub_common::{
    ApiResponse, LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo,
    PricingRate, SlotStatus,
};

use crate::requests::{parse_lot_status, CreateParkingLotRequest, UpdateParkingLotRequest};

use super::{AuthUser, SharedState};
use parkhub_common::UserRole;

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) async fn list_lots(
    State(state): State<SharedState>,
) -> Json<ApiResponse<Vec<ParkingLot>>> {
    let state = state.read().await;

    match state.db.list_parking_lots().await {
        Ok(lots) => Json(ApiResponse::success(lots)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list parking lots",
            ))
        }
    }
}

pub(crate) async fn create_lot(
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

    let state_guard = state.write().await;

    // Check if user is admin
    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
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
                x: (((i - 1) % 10) as f32) * 3.0,
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

    tracing::info!(
        "Created parking lot '{}' ({}) with {} slots",
        lot.name,
        lot.id,
        req.total_slots,
    );

    (StatusCode::CREATED, Json(ApiResponse::success(lot)))
}

pub(crate) async fn update_lot(
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

    let state_guard = state.write().await;

    // Check if user is admin
    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
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

    tracing::info!("Updated parking lot '{}' ({})", lot.name, lot.id);

    (StatusCode::OK, Json(ApiResponse::success(lot)))
}

pub(crate) async fn delete_lot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;

    // Check if user is admin
    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    match state_guard.db.delete_parking_lot(&id).await {
        Ok(true) => {
            // Cascade-delete orphaned slots belonging to this lot
            if let Err(e) = state_guard.db.delete_slots_by_lot(&id).await {
                tracing::error!("Failed to cascade-delete slots for lot {}: {}", id, e);
            }
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

pub(crate) async fn get_lot(
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

pub(crate) async fn get_lot_slots(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<ParkingSlot>>> {
    let state = state.read().await;

    match state.db.list_slots_by_lot(&id).await {
        Ok(slots) => Json(ApiResponse::success(slots)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error("SERVER_ERROR", "Failed to list slots"))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Slot CRUD (admin only)
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/lots/{lot_id}/slots` — create a new slot in a lot
pub(crate) async fn create_slot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<ParkingSlot>>) {
    let state_guard = state.write().await;

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

    let floor_id = lot
        .floors
        .first()
        .map(|f| f.id)
        .unwrap_or_else(Uuid::new_v4);
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

    let slot = ParkingSlot {
        id: Uuid::new_v4(),
        lot_id: lot.id,
        floor_id,
        slot_number: req
            .get("slot_number")
            .and_then(|v| v.as_i64())
            .unwrap_or(next_number as i64) as i32,
        row: ((next_number - 1) / 10) + 1,
        column: ((next_number - 1) % 10) + 1,
        slot_type,
        status: SlotStatus::Available,
        current_booking: None,
        features: Vec::new(),
        position: SlotPosition {
            x: (((next_number - 1) % 10) as f32) * 3.0,
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

    (StatusCode::CREATED, Json(ApiResponse::success(slot)))
}

/// `PUT /api/v1/lots/{lot_id}/slots/{slot_id}` — update a slot
pub(crate) async fn update_slot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, slot_id)): Path<(String, String)>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<ParkingSlot>>) {
    let state_guard = state.write().await;

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

    if let Some(number) = req.get("slot_number").and_then(|v| v.as_i64()) {
        slot.slot_number = number as i32;
    }

    if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
        tracing::error!("Failed to update slot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update slot")),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(slot)))
}

/// `DELETE /api/v1/lots/{lot_id}/slots/{slot_id}` — delete a slot
pub(crate) async fn delete_slot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((lot_id, slot_id)): Path<(String, String)>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;

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

    (StatusCode::OK, Json(ApiResponse::success(())))
}
