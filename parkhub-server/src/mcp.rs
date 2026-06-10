//! MCP (Model Context Protocol) server for ParkHub.
//!
//! Exposes four tools over the stdio MCP transport so AI agents can query
//! and create parking bookings on behalf of an authenticated user.
//!
//! Auth: set `PARKHUB_API_KEY` to a valid ParkHub API key before starting.
//! The key is resolved to its owner user; all tool calls operate as that user.
//!
//! Start: `parkhub-server --mcp`

// AppState read/write guards span async db calls by design.
#![allow(clippy::significant_drop_tightening)]

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use parkhub_common::{
    Booking, BookingPricing, BookingStatus, FuelType, LotStatus, PaymentStatus, SlotStatus,
    Vehicle, VehicleType,
};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::Database;

// ── Helper ─────────────────────────────────────────────────────────────────────

fn parse_dt(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("expected RFC 3339 datetime, got: {s:?}"))
}

fn json_error(msg: &str) -> String {
    serde_json::json!({"error": msg}).to_string()
}

// ── Input types ────────────────────────────────────────────────────────────────

/// Parameters for `check_availability`
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct CheckAvailabilityInput {
    /// Optional lot UUID (string). Omit to query all lots.
    pub lot_id: Option<String>,
    /// Window start in RFC 3339 format, e.g. "2025-01-15T09:00:00Z"
    pub from: String,
    /// Window end in RFC 3339 format
    pub to: String,
}

/// Parameters for `get_occupancy`
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetOccupancyInput {
    /// Optional lot UUID (string). Omit to query all lots.
    pub lot_id: Option<String>,
}

/// Parameters for `create_booking`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBookingInput {
    /// Lot UUID (string)
    pub lot_id: String,
    /// Slot UUID (string). If omitted, the first available slot in the lot is used.
    pub slot_id: Option<String>,
    /// Booking start in RFC 3339 format
    pub from: String,
    /// Booking end in RFC 3339 format
    pub to: String,
}

// ── Output types ──────────────────────────────────────────────────────────────

/// Free-slot availability for one lot during the requested time window
#[derive(Debug, Serialize)]
pub struct LotAvailability {
    pub lot_id: Uuid,
    pub lot_name: String,
    /// Slots with no overlapping booking in the requested window
    pub free_slots: usize,
    pub total_slots: usize,
}

/// Current occupancy snapshot for one lot
#[derive(Debug, Serialize)]
pub struct LotOccupancy {
    pub lot_id: Uuid,
    pub lot_name: String,
    pub total_slots: i32,
    pub occupied_slots: i32,
    pub available_slots: i32,
    /// Lot status: "open", "closed", "full", "maintenance"
    pub status: String,
}

/// Confirmation returned after successfully creating a booking
#[derive(Debug, Serialize)]
pub struct BookingCreated {
    pub booking_id: Uuid,
    pub lot_id: Uuid,
    pub slot_id: Uuid,
    pub slot_number: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    /// Always "confirmed" on success
    pub status: String,
}

// ── Handler struct ─────────────────────────────────────────────────────────────

/// MCP server handler holding a database handle and the resolved user identity.
pub struct ParkHubMcp {
    db: Database,
    user_id: Uuid,
}

impl ParkHubMcp {
    /// Construct with an already-opened database and authenticated user.
    pub fn new(db: Database, user_id: Uuid) -> Self {
        Self { db, user_id }
    }
}

// ── Business logic (plain async, directly testable) ───────────────────────────

impl ParkHubMcp {
    /// Return free-slot counts per lot for the given time window.
    ///
    /// A slot counts as "free" when:
    /// - it has no booking whose interval `[start, end)` overlaps `[from, to)`, AND
    /// - its status is not `Maintenance` or `Disabled`.
    pub async fn check_availability_impl(
        &self,
        lot_id: Option<Uuid>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<LotAvailability>> {
        let lots = self.db.list_parking_lots().await?;
        let bookings = self.db.list_bookings().await?;

        let lots: Vec<_> = match lot_id {
            Some(lid) => lots.into_iter().filter(|l| l.id == lid).collect(),
            None => lots,
        };

        let mut results = Vec::new();
        for lot in &lots {
            let slots = self.db.list_slots_by_lot(&lot.id.to_string()).await?;
            let total = slots.len();
            let free = slots
                .iter()
                .filter(|slot| {
                    if slot.status == SlotStatus::Maintenance || slot.status == SlotStatus::Disabled
                    {
                        return false;
                    }
                    !bookings.iter().any(|b| {
                        b.slot_id == slot.id
                            && !matches!(
                                b.status,
                                BookingStatus::Cancelled | BookingStatus::Expired
                            )
                            && b.start_time < to
                            && b.end_time > from
                    })
                })
                .count();
            results.push(LotAvailability {
                lot_id: lot.id,
                lot_name: lot.name.clone(),
                free_slots: free,
                total_slots: total,
            });
        }
        Ok(results)
    }

    /// Return the current occupancy snapshot (derived from lot-level counters).
    pub async fn get_occupancy_impl(&self, lot_id: Option<Uuid>) -> Result<Vec<LotOccupancy>> {
        let lots = self.db.list_parking_lots().await?;
        let lots: Vec<_> = match lot_id {
            Some(lid) => lots.into_iter().filter(|l| l.id == lid).collect(),
            None => lots,
        };

        let results = lots
            .iter()
            .map(|lot| {
                let status = match lot.status {
                    LotStatus::Open => "open",
                    LotStatus::Closed => "closed",
                    LotStatus::Full => "full",
                    LotStatus::Maintenance => "maintenance",
                };
                LotOccupancy {
                    lot_id: lot.id,
                    lot_name: lot.name.clone(),
                    total_slots: lot.total_slots,
                    occupied_slots: lot.total_slots - lot.available_slots,
                    available_slots: lot.available_slots,
                    status: status.to_string(),
                }
            })
            .collect();
        Ok(results)
    }

    /// List bookings owned by the API-key user.
    pub async fn list_my_bookings_impl(&self) -> Result<Vec<Booking>> {
        self.db
            .list_bookings_by_user(&self.user_id.to_string())
            .await
            .context("failed to load bookings")
    }

    /// Create a booking for the API-key user.
    ///
    /// Validation mirrors the REST `POST /api/v1/bookings` path:
    /// - `from` must be in the future
    /// - `to` must be after `from`
    /// - lot must exist and be `Open`
    /// - slot (explicit or auto-selected) must be `Available`
    pub async fn create_booking_impl(
        &self,
        lot_id: Uuid,
        slot_id: Option<Uuid>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<BookingCreated> {
        if to <= from {
            anyhow::bail!("INVALID_INPUT: end time must be after start time");
        }
        let duration_minutes = (to - from).num_minutes();
        if duration_minutes <= 0 {
            anyhow::bail!("INVALID_INPUT: duration must be positive");
        }
        if from <= Utc::now() {
            anyhow::bail!("INVALID_BOOKING_TIME: booking start time must be in the future");
        }

        let lot = self
            .db
            .get_parking_lot(&lot_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("NOT_FOUND: lot not found"))?;

        if lot.status != LotStatus::Open {
            anyhow::bail!(
                "LOT_UNAVAILABLE: lot is not accepting bookings (status: {:?})",
                lot.status
            );
        }

        let slot = match slot_id {
            Some(sid) => self
                .db
                .get_parking_slot(&sid.to_string())
                .await?
                .ok_or_else(|| anyhow::anyhow!("NOT_FOUND: slot not found"))?,
            None => {
                let slots = self.db.list_slots_by_lot(&lot_id.to_string()).await?;
                slots
                    .into_iter()
                    .find(|s| s.status == SlotStatus::Available)
                    .ok_or_else(|| anyhow::anyhow!("SLOT_UNAVAILABLE: no available slots in lot"))?
            }
        };

        if slot.status != SlotStatus::Available {
            anyhow::bail!("SLOT_UNAVAILABLE: this slot is not available");
        }

        let now = Utc::now();
        let floor_name = lot
            .floors
            .iter()
            .find(|f| f.id == slot.floor_id)
            .map_or_else(|| "Level 1".to_string(), |f| f.name.clone());

        let booking = Booking {
            id: Uuid::new_v4(),
            user_id: self.user_id,
            lot_id,
            slot_id: slot.id,
            slot_number: slot.slot_number,
            floor_name,
            vehicle: Vehicle {
                id: Uuid::new_v4(),
                user_id: self.user_id,
                license_plate: String::new(),
                make: None,
                model: None,
                color: None,
                vehicle_type: VehicleType::Car,
                fuel_type: FuelType::Unknown,
                is_default: false,
                created_at: now,
            },
            start_time: from,
            end_time: to,
            status: BookingStatus::Confirmed,
            pricing: BookingPricing {
                base_price: 0.0,
                discount: 0.0,
                tax: 0.0,
                total: 0.0,
                currency: lot.pricing.currency.clone(),
                payment_status: PaymentStatus::Pending,
                payment_method: None,
            },
            created_at: now,
            updated_at: now,
            check_in_time: None,
            check_out_time: None,
            qr_code: Some(Uuid::new_v4().to_string()),
            notes: None,
            tenant_id: None,
        };

        let mut updated_slot = slot.clone();
        updated_slot.status = SlotStatus::Reserved;

        self.db.save_booking(&booking).await?;
        self.db.save_parking_slot(&updated_slot).await?;

        Ok(BookingCreated {
            booking_id: booking.id,
            lot_id,
            slot_id: slot.id,
            slot_number: slot.slot_number,
            start_time: from,
            end_time: to,
            status: "confirmed".to_string(),
        })
    }
}

// ── MCP tool surface ──────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl ParkHubMcp {
    /// Check parking availability for a time window.
    ///
    /// Returns free-slot counts per lot. A slot is "free" when it has no
    /// confirmed/pending booking overlapping the requested window AND it is
    /// not in Maintenance or Disabled status.
    #[tool(
        description = "Check parking lot availability for a time window. Returns free slot counts per lot (and all lots if lot_id is omitted)."
    )]
    async fn check_availability(
        &self,
        Parameters(p): Parameters<CheckAvailabilityInput>,
    ) -> String {
        let lot_id = p.lot_id.as_deref().and_then(|s| s.parse::<Uuid>().ok());
        let from = match parse_dt(&p.from) {
            Ok(t) => t,
            Err(e) => return json_error(&format!("invalid 'from': {e}")),
        };
        let to = match parse_dt(&p.to) {
            Ok(t) => t,
            Err(e) => return json_error(&format!("invalid 'to': {e}")),
        };
        match self.check_availability_impl(lot_id, from, to).await {
            Ok(result) => {
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| json_error(&e.to_string()))
            }
            Err(e) => json_error(&e.to_string()),
        }
    }

    /// Get current occupancy snapshot for parking lots.
    ///
    /// Returns total / occupied / available counts per lot along with lot status.
    #[tool(
        description = "Get current parking lot occupancy. Returns total, occupied, and available slot counts. Optionally filter by lot_id."
    )]
    async fn get_occupancy(&self, Parameters(p): Parameters<GetOccupancyInput>) -> String {
        let lot_id = p.lot_id.as_deref().and_then(|s| s.parse::<Uuid>().ok());
        match self.get_occupancy_impl(lot_id).await {
            Ok(result) => {
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| json_error(&e.to_string()))
            }
            Err(e) => json_error(&e.to_string()),
        }
    }

    /// List bookings for the authenticated user.
    ///
    /// Returns all bookings owned by the API-key user, including past and upcoming.
    #[tool(
        description = "List all parking bookings for the authenticated user (identified by PARKHUB_API_KEY)."
    )]
    async fn list_my_bookings(&self) -> String {
        match self.list_my_bookings_impl().await {
            Ok(result) => {
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| json_error(&e.to_string()))
            }
            Err(e) => json_error(&e.to_string()),
        }
    }

    /// Create a parking booking for the authenticated user.
    ///
    /// Applies the same validation as the REST API: start time must be in the
    /// future, lot must be open, slot must be available. If `slot_id` is omitted
    /// the first available slot in the lot is used automatically.
    #[tool(
        description = "Create a parking booking for the authenticated user. lot_id and time window (from/to in RFC 3339) are required. slot_id is optional — omit to auto-select."
    )]
    async fn create_booking(&self, Parameters(p): Parameters<CreateBookingInput>) -> String {
        let lot_id = match p.lot_id.parse::<Uuid>() {
            Ok(id) => id,
            Err(e) => return json_error(&format!("invalid lot_id: {e}")),
        };
        let slot_id = match p.slot_id.as_deref() {
            Some(s) => match s.parse::<Uuid>() {
                Ok(id) => Some(id),
                Err(e) => return json_error(&format!("invalid slot_id: {e}")),
            },
            None => None,
        };
        let from = match parse_dt(&p.from) {
            Ok(t) => t,
            Err(e) => return json_error(&format!("invalid 'from': {e}")),
        };
        let to = match parse_dt(&p.to) {
            Ok(t) => t,
            Err(e) => return json_error(&format!("invalid 'to': {e}")),
        };
        match self.create_booking_impl(lot_id, slot_id, from, to).await {
            Ok(result) => {
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| json_error(&e.to_string()))
            }
            Err(e) => json_error(&e.to_string()),
        }
    }
}

// ── Entrypoint ─────────────────────────────────────────────────────────────────

/// Run the MCP server over stdio until the transport closes.
///
/// Reads `PARKHUB_API_KEY` from the environment, validates it against the DB,
/// and then hands off to the rmcp stdio loop.
pub async fn run(db: Database) -> Result<()> {
    let api_key =
        std::env::var("PARKHUB_API_KEY").context("PARKHUB_API_KEY environment variable not set")?;

    let (user_id, _key_id) = crate::api::security::validate_api_key_detailed(&db, &api_key)
        .await
        .ok_or_else(|| anyhow::anyhow!("Invalid or unknown PARKHUB_API_KEY — key rejected"))?;

    tracing::info!(user_id = %user_id, "MCP server authenticated");

    let handler = ParkHubMcp::new(db, user_id);

    handler
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await
        .context("MCP server initialization failed")?
        .waiting()
        .await
        .context("MCP server transport error")?;

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, DatabaseConfig};
    use chrono::TimeDelta;
    use parkhub_common::models::SlotPosition;
    use parkhub_common::{
        LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo, SlotStatus,
        SlotType,
    };
    use tempfile::TempDir;

    // ── Test helpers ─────────────────────────────────────────────────────────

    fn open_test_db() -> (Database, TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Database::open(&DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        })
        .expect("open test db");
        (db, dir)
    }

    fn make_lot(status: LotStatus) -> ParkingLot {
        let floor_id = Uuid::new_v4();
        let lot_id = Uuid::new_v4();
        ParkingLot {
            id: lot_id,
            name: "Test Lot".to_string(),
            address: "1 Test St".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            total_slots: 5,
            available_slots: 5,
            floors: vec![ParkingFloor {
                id: floor_id,
                lot_id,
                name: "Ground Floor".to_string(),
                floor_number: 0,
                total_slots: 5,
                available_slots: 5,
                slots: vec![],
            }],
            amenities: vec![],
            pricing: PricingInfo {
                currency: "EUR".to_string(),
                rates: vec![],
                daily_max: None,
                monthly_pass: None,
            },
            operating_hours: OperatingHours {
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
            status,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tenant_id: None,
        }
    }

    fn make_slot(lot_id: Uuid, floor_id: Uuid, number: i32, status: SlotStatus) -> ParkingSlot {
        ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: number,
            row: 0,
            column: number - 1,
            slot_type: SlotType::Standard,
            status,
            current_booking: None,
            features: vec![],
            position: SlotPosition {
                x: 0.0,
                y: 0.0,
                width: 70.0,
                height: 90.0,
                rotation: 0.0,
            },
            is_accessible: false,
        }
    }

    async fn handler_with_lot(lot: ParkingLot, slots: &[ParkingSlot]) -> (ParkHubMcp, TempDir) {
        let (db, dir) = open_test_db();
        let user_id = Uuid::new_v4();

        db.save_parking_lot(&lot).await.unwrap();
        for s in slots {
            db.save_parking_slot(s).await.unwrap();
        }

        let handler = ParkHubMcp::new(db, user_id);
        (handler, dir)
    }

    fn future(mins: i64) -> DateTime<Utc> {
        Utc::now() + TimeDelta::minutes(mins)
    }

    // ── check_availability_impl ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_availability_empty_db_returns_empty() {
        let (db, _dir) = open_test_db();
        let handler = ParkHubMcp::new(db, Uuid::new_v4());
        let result = handler
            .check_availability_impl(None, future(60), future(120))
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_availability_counts_free_slots() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let s1 = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let s2 = make_slot(lot_id, floor_id, 2, SlotStatus::Available);
        let s3 = make_slot(lot_id, floor_id, 3, SlotStatus::Maintenance);
        let (handler, _dir) = handler_with_lot(lot, &[s1, s2, s3]).await;

        let result = handler
            .check_availability_impl(None, future(60), future(120))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].free_slots, 2,
            "maintenance slot not counted as free"
        );
        assert_eq!(result[0].total_slots, 3);
    }

    #[tokio::test]
    async fn test_availability_slot_with_overlapping_booking_not_free() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let slot_id = slot.id;
        let user_id = Uuid::new_v4();

        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        // Insert a confirmed booking that overlaps the query window
        let booking = parkhub_common::Booking {
            id: Uuid::new_v4(),
            user_id,
            lot_id,
            slot_id,
            slot_number: 1,
            floor_name: "Ground Floor".to_string(),
            vehicle: Vehicle {
                id: Uuid::new_v4(),
                user_id,
                license_plate: "AB-123".to_string(),
                make: None,
                model: None,
                color: None,
                vehicle_type: VehicleType::Car,
                fuel_type: FuelType::Unknown,
                is_default: false,
                created_at: Utc::now(),
            },
            start_time: future(70),
            end_time: future(130),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            check_in_time: None,
            check_out_time: None,
            qr_code: None,
            notes: None,
            tenant_id: None,
        };
        handler.db.save_booking(&booking).await.unwrap();

        let result = handler
            .check_availability_impl(None, future(60), future(120))
            .await
            .unwrap();

        assert_eq!(
            result[0].free_slots, 0,
            "slot with overlapping booking must not be free"
        );
    }

    #[tokio::test]
    async fn test_availability_cancelled_booking_slot_is_free() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let slot_id = slot.id;
        let user_id = Uuid::new_v4();
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let mut booking = parkhub_common::Booking {
            id: Uuid::new_v4(),
            user_id,
            lot_id,
            slot_id,
            slot_number: 1,
            floor_name: "Ground Floor".to_string(),
            vehicle: Vehicle {
                id: Uuid::new_v4(),
                user_id,
                license_plate: String::new(),
                make: None,
                model: None,
                color: None,
                vehicle_type: VehicleType::Car,
                fuel_type: FuelType::Unknown,
                is_default: false,
                created_at: Utc::now(),
            },
            start_time: future(70),
            end_time: future(130),
            status: BookingStatus::Cancelled, // cancelled — slot remains free
            pricing: BookingPricing {
                base_price: 0.0,
                discount: 0.0,
                tax: 0.0,
                total: 0.0,
                currency: "EUR".to_string(),
                payment_status: PaymentStatus::Pending,
                payment_method: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            check_in_time: None,
            check_out_time: None,
            qr_code: None,
            notes: None,
            tenant_id: None,
        };
        handler.db.save_booking(&booking).await.unwrap();

        let result = handler
            .check_availability_impl(None, future(60), future(120))
            .await
            .unwrap();
        assert_eq!(
            result[0].free_slots, 1,
            "cancelled booking must not block slot"
        );

        // Also verify expired bookings don't block
        booking.id = Uuid::new_v4();
        booking.status = BookingStatus::Expired;
        handler.db.save_booking(&booking).await.unwrap();

        let result = handler
            .check_availability_impl(None, future(60), future(120))
            .await
            .unwrap();
        assert_eq!(
            result[0].free_slots, 1,
            "expired booking must not block slot"
        );
    }

    #[tokio::test]
    async fn test_availability_lot_id_filter() {
        let lot_a = make_lot(LotStatus::Open);
        let lot_b = make_lot(LotStatus::Open);
        let floor_a = lot_a.floors[0].id;
        let floor_b = lot_b.floors[0].id;
        let (db, _dir) = open_test_db();
        let uid = Uuid::new_v4();

        db.save_parking_lot(&lot_a).await.unwrap();
        db.save_parking_lot(&lot_b).await.unwrap();
        db.save_parking_slot(&make_slot(lot_a.id, floor_a, 1, SlotStatus::Available))
            .await
            .unwrap();
        db.save_parking_slot(&make_slot(lot_b.id, floor_b, 1, SlotStatus::Available))
            .await
            .unwrap();

        let handler = ParkHubMcp::new(db, uid);
        let result = handler
            .check_availability_impl(Some(lot_a.id), future(60), future(120))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].lot_id, lot_a.id);
    }

    // ── get_occupancy_impl ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_occupancy_snapshot() {
        let mut lot = make_lot(LotStatus::Open);
        lot.total_slots = 10;
        lot.available_slots = 7; // 3 occupied
        let (handler, _dir) = handler_with_lot(lot.clone(), &[]).await;

        let result = handler.get_occupancy_impl(None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].total_slots, 10);
        assert_eq!(result[0].occupied_slots, 3);
        assert_eq!(result[0].available_slots, 7);
        assert_eq!(result[0].status, "open");
    }

    #[tokio::test]
    async fn test_occupancy_lot_filter() {
        let lot_a = make_lot(LotStatus::Open);
        let lot_b = make_lot(LotStatus::Closed);
        let (db, _dir) = open_test_db();
        db.save_parking_lot(&lot_a).await.unwrap();
        db.save_parking_lot(&lot_b).await.unwrap();
        let handler = ParkHubMcp::new(db, Uuid::new_v4());

        let result = handler.get_occupancy_impl(Some(lot_b.id)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "closed");
    }

    // ── list_my_bookings_impl ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_bookings_scoped_to_key_user() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let slot_id = slot.id;
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let (db, _dir) = open_test_db();
        db.save_parking_lot(&lot).await.unwrap();
        db.save_parking_slot(&slot).await.unwrap();

        let make_booking = |uid: Uuid, sid: Uuid| parkhub_common::Booking {
            id: Uuid::new_v4(),
            user_id: uid,
            lot_id,
            slot_id: sid,
            slot_number: 1,
            floor_name: "Ground Floor".to_string(),
            vehicle: Vehicle {
                id: Uuid::new_v4(),
                user_id: uid,
                license_plate: String::new(),
                make: None,
                model: None,
                color: None,
                vehicle_type: VehicleType::Car,
                fuel_type: FuelType::Unknown,
                is_default: false,
                created_at: Utc::now(),
            },
            start_time: future(60),
            end_time: future(120),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            check_in_time: None,
            check_out_time: None,
            qr_code: None,
            notes: None,
            tenant_id: None,
        };

        db.save_booking(&make_booking(user_a, slot_id))
            .await
            .unwrap();
        db.save_booking(&make_booking(user_a, slot_id))
            .await
            .unwrap();
        db.save_booking(&make_booking(user_b, slot_id))
            .await
            .unwrap(); // different user

        let handler_a = ParkHubMcp::new(db.clone(), user_a);
        let bookings_a = handler_a.list_my_bookings_impl().await.unwrap();
        assert_eq!(bookings_a.len(), 2, "user A should see exactly 2 bookings");
        assert!(
            bookings_a.iter().all(|b| b.user_id == user_a),
            "all returned bookings must belong to user A"
        );

        let handler_b = ParkHubMcp::new(db, user_b);
        let bookings_b = handler_b.list_my_bookings_impl().await.unwrap();
        assert_eq!(bookings_b.len(), 1, "user B should see exactly 1 booking");
    }

    // ── create_booking_impl ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_booking_success() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let slot_id = slot.id;
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let result = handler
            .create_booking_impl(lot_id, Some(slot_id), future(60), future(120))
            .await
            .unwrap();

        assert_eq!(result.lot_id, lot_id);
        assert_eq!(result.slot_id, slot_id);
        assert_eq!(result.status, "confirmed");
    }

    #[tokio::test]
    async fn test_create_booking_auto_selects_slot() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let s1 = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let (handler, _dir) = handler_with_lot(lot, &[s1]).await;

        let result = handler
            .create_booking_impl(lot_id, None, future(60), future(120))
            .await
            .unwrap();

        assert_eq!(result.status, "confirmed");
        assert_eq!(result.lot_id, lot_id);
    }

    #[tokio::test]
    async fn test_create_booking_rejects_past_start_time() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let err = handler
            .create_booking_impl(
                lot_id,
                None,
                Utc::now() - TimeDelta::minutes(30), // past
                future(60),
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("INVALID_BOOKING_TIME"),
            "expected INVALID_BOOKING_TIME, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_booking_rejects_end_before_start() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let err = handler
            .create_booking_impl(lot_id, None, future(120), future(60))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("INVALID_INPUT"),
            "expected INVALID_INPUT, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_booking_rejects_disabled_lot() {
        let lot = make_lot(LotStatus::Closed); // disabled
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let err = handler
            .create_booking_impl(lot_id, None, future(60), future(120))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("LOT_UNAVAILABLE"),
            "expected LOT_UNAVAILABLE, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_booking_rejects_maintenance_lot() {
        let lot = make_lot(LotStatus::Maintenance);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Available);
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let err = handler
            .create_booking_impl(lot_id, None, future(60), future(120))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("LOT_UNAVAILABLE"),
            "expected LOT_UNAVAILABLE for maintenance lot, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_booking_rejects_occupied_slot() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        let slot = make_slot(lot_id, floor_id, 1, SlotStatus::Reserved); // already reserved
        let slot_id = slot.id;
        let (handler, _dir) = handler_with_lot(lot, &[slot]).await;

        let err = handler
            .create_booking_impl(lot_id, Some(slot_id), future(60), future(120))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("SLOT_UNAVAILABLE"),
            "expected SLOT_UNAVAILABLE, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_booking_no_available_slot_in_lot() {
        let lot = make_lot(LotStatus::Open);
        let floor_id = lot.floors[0].id;
        let lot_id = lot.id;
        // All slots occupied
        let s1 = make_slot(lot_id, floor_id, 1, SlotStatus::Occupied);
        let s2 = make_slot(lot_id, floor_id, 2, SlotStatus::Reserved);
        let (handler, _dir) = handler_with_lot(lot, &[s1, s2]).await;

        let err = handler
            .create_booking_impl(lot_id, None, future(60), future(120))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("SLOT_UNAVAILABLE"),
            "expected SLOT_UNAVAILABLE when no slots free, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_booking_unknown_lot_returns_not_found() {
        let (db, _dir) = open_test_db();
        let handler = ParkHubMcp::new(db, Uuid::new_v4());

        let err = handler
            .create_booking_impl(Uuid::new_v4(), None, future(60), future(120))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("NOT_FOUND"),
            "expected NOT_FOUND, got: {err}"
        );
    }

    // ── auth: unknown API key fails closed ────────────────────────────────────

    #[tokio::test]
    async fn test_unknown_api_key_fails_closed() {
        let (db, _dir) = open_test_db();
        // No API keys stored — validate_api_key_detailed must return None
        let result =
            crate::api::security::validate_api_key_detailed(&db, "ph_unknown_key_xyz").await;
        assert!(
            result.is_none(),
            "unknown API key must be rejected (returns None)"
        );
    }
}
