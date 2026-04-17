//! Parking lot, parking slot, and zone CRUD with slot-by-lot secondary index.

use anyhow::Result;
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use parkhub_common::models::{ParkingLot, ParkingSlot};

use super::{Database, PARKING_LOTS, PARKING_SLOTS, SLOTS_BY_LOT, ZONES};

/// A zone within a parking lot (e.g., "Level A", "VIP Section")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Database {
    // ── Parking Lot CRUD ──

    /// Save a parking lot
    pub async fn save_parking_lot(&self, lot: &ParkingLot) -> Result<()> {
        let id = lot.id.to_string();
        let data = self.serialize(lot)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(PARKING_LOTS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved parking lot: {} ({})", lot.name, lot.id);
        Ok(())
    }

    /// Get a parking lot by ID (string)
    pub async fn get_parking_lot(&self, id: &str) -> Result<Option<ParkingLot>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PARKING_LOTS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all parking lots
    pub async fn list_parking_lots(&self) -> Result<Vec<ParkingLot>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PARKING_LOTS)?;

        let mut lots = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            lots.push(self.deserialize(value.value())?);
        }
        Ok(lots)
    }

    /// Delete a parking lot
    pub async fn delete_parking_lot(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(PARKING_LOTS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted parking lot: {}", id);
        }
        Ok(existed)
    }

    // ── Parking Slot CRUD ──

    /// Save a parking slot
    pub async fn save_parking_slot(&self, slot: &ParkingSlot) -> Result<()> {
        let id = slot.id.to_string();
        let lot_id = slot.lot_id.to_string();
        let data = self.serialize(slot)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            // Save main slot data
            let mut table = write_txn.open_table(PARKING_SLOTS)?;
            table.insert(id.as_str(), data.as_slice())?;

            // Update lot->slots index
            let mut idx = write_txn.open_table(SLOTS_BY_LOT)?;
            let key = format!("{lot_id}:{id}");
            idx.insert(key.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved parking slot: {} (lot: {})", slot.id, slot.lot_id);
        Ok(())
    }

    /// Get a parking slot by ID (string)
    pub async fn get_parking_slot(&self, id: &str) -> Result<Option<ParkingSlot>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PARKING_SLOTS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get all parking slots for a lot (`list_slots_by_lot`)
    pub async fn list_slots_by_lot(&self, lot_id: &str) -> Result<Vec<ParkingSlot>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SLOTS_BY_LOT)?;

        let prefix = format!("{lot_id}:");
        let mut slots = Vec::new();

        for entry in table.iter()? {
            let (key, value) = entry?;
            if key.value().starts_with(&prefix) {
                slots.push(self.deserialize(value.value())?);
            }
        }
        Ok(slots)
    }

    /// Delete all parking slots belonging to a lot (cascade delete).
    /// Removes entries from both `PARKING_SLOTS` and `SLOTS_BY_LOT` index.
    pub async fn delete_slots_by_lot(&self, lot_id: &str) -> Result<()> {
        let prefix = format!("{lot_id}:");

        let db = self.inner.write().await;

        // First, collect all slot IDs and index keys from SLOTS_BY_LOT
        let keys_to_delete: Vec<(String, String)> = {
            let read_txn = db.begin_read()?;
            let table = read_txn.open_table(SLOTS_BY_LOT)?;
            let mut keys = Vec::new();
            for entry in table.iter()? {
                let (key, _value) = entry?;
                let key_str = key.value().to_string();
                if key_str.starts_with(&prefix) {
                    // key format is "lot_id:slot_id"
                    let slot_id = key_str[prefix.len()..].to_string();
                    keys.push((key_str, slot_id));
                }
            }
            keys
        };

        if keys_to_delete.is_empty() {
            return Ok(());
        }

        // Delete all matching entries in a single write transaction
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut slots_table = write_txn.open_table(PARKING_SLOTS)?;
            let mut idx_table = write_txn.open_table(SLOTS_BY_LOT)?;
            for (idx_key, slot_id) in &keys_to_delete {
                slots_table.remove(slot_id.as_str())?;
                idx_table.remove(idx_key.as_str())?;
            }
        }
        write_txn.commit()?;
        debug!(
            "Cascade-deleted {} slots for lot {}",
            keys_to_delete.len(),
            lot_id
        );
        Ok(())
    }

    /// Delete a single parking slot by ID.
    pub async fn delete_parking_slot(&self, id: &str) -> Result<bool> {
        let id_suffix = format!(":{id}");
        let db = self.inner.write().await;

        // First collect index keys to remove (read pass)
        let keys_to_remove: Vec<String> = {
            let read_txn = db.begin_read()?;
            let idx_table = read_txn.open_table(SLOTS_BY_LOT)?;
            let mut keys = Vec::new();
            for entry in idx_table.iter()? {
                let (key, _) = entry?;
                if key.value().ends_with(&id_suffix) {
                    keys.push(key.value().to_string());
                }
            }
            keys
        };

        // Write pass: remove slot + index entries
        let write_txn = db.begin_write()?;
        drop(db);
        let removed = {
            let mut table = write_txn.open_table(PARKING_SLOTS)?;

            table.remove(id)?.is_some()
        };
        if removed && !keys_to_remove.is_empty() {
            let mut idx_table = write_txn.open_table(SLOTS_BY_LOT)?;
            for key in &keys_to_remove {
                idx_table.remove(key.as_str())?;
            }
        }
        write_txn.commit()?;
        Ok(removed)
    }

    /// Save multiple parking slots in a single write transaction (batch insert).
    pub async fn save_parking_slots_batch(&self, slots: &[ParkingSlot]) -> Result<()> {
        if slots.is_empty() {
            return Ok(());
        }

        // Pre-serialize all slots before acquiring the write lock
        let serialized: Vec<(String, String, Vec<u8>)> = slots
            .iter()
            .map(|slot| {
                let id = slot.id.to_string();
                let lot_id = slot.lot_id.to_string();
                let data = self.serialize(slot)?;
                Ok((id, lot_id, data))
            })
            .collect::<Result<Vec<_>>>()?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(PARKING_SLOTS)?;
            let mut idx = write_txn.open_table(SLOTS_BY_LOT)?;
            for (id, lot_id, data) in &serialized {
                table.insert(id.as_str(), data.as_slice())?;
                let key = format!("{lot_id}:{id}");
                idx.insert(key.as_str(), data.as_slice())?;
            }
        }
        write_txn.commit()?;
        debug!("Batch-saved {} parking slots", slots.len());
        Ok(())
    }

    /// Update slot status
    pub async fn update_slot_status(
        &self,
        slot_id: &str,
        status: parkhub_common::models::SlotStatus,
    ) -> Result<bool> {
        let Some(mut slot) = self.get_parking_slot(slot_id).await? else {
            return Ok(false);
        };

        slot.status = status;
        self.save_parking_slot(&slot).await?;
        Ok(true)
    }

    // ── Zone CRUD ──

    /// Save a zone
    pub async fn save_zone(&self, zone: &Zone) -> Result<()> {
        let key = format!("{}:{}", zone.lot_id, zone.id);
        let data = self.serialize(zone)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(ZONES)?;
            table.insert(key.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved zone: {} (lot: {})", zone.id, zone.lot_id);
        Ok(())
    }

    /// List all zones for a parking lot
    pub async fn list_zones_by_lot(&self, lot_id: &str) -> Result<Vec<Zone>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(ZONES)?;

        let prefix = format!("{lot_id}:");
        let mut zones = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            if key.value().starts_with(&prefix) {
                zones.push(self.deserialize(value.value())?);
            }
        }
        Ok(zones)
    }

    /// Delete a zone by `lot_id` and `zone_id`
    pub async fn delete_zone(&self, lot_id: &str, zone_id: &str) -> Result<bool> {
        let key = format!("{lot_id}:{zone_id}");

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(ZONES)?;
            let result = table.remove(key.as_str())?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted zone {} from lot {}", zone_id, lot_id);
        }
        Ok(existed)
    }
}
