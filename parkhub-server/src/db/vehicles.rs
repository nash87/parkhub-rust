//! Vehicle CRUD with per-user listing.

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};
use tracing::debug;

use parkhub_common::models::Vehicle;

use super::{Database, VEHICLES};

impl Database {
    /// Save a vehicle
    pub async fn save_vehicle(&self, vehicle: &Vehicle) -> Result<()> {
        let id = vehicle.id.to_string();
        let data = self.serialize(vehicle)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(VEHICLES)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved vehicle: {} ({})", vehicle.license_plate, vehicle.id);
        Ok(())
    }

    /// Get a vehicle by ID (string)
    pub async fn get_vehicle(&self, id: &str) -> Result<Option<Vehicle>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(VEHICLES)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get vehicles for a user (`list_vehicles_by_user`)
    pub async fn list_vehicles_by_user(&self, user_id: &str) -> Result<Vec<Vehicle>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(VEHICLES)?;

        let mut vehicles = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let vehicle: Vehicle = self.deserialize(value.value())?;
            if vehicle.user_id.to_string() == user_id {
                vehicles.push(vehicle);
            }
        }
        Ok(vehicles)
    }

    /// List all vehicles across all users.
    pub async fn list_all_vehicles(&self) -> Result<Vec<Vehicle>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(VEHICLES)?;

        let mut vehicles = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            vehicles.push(self.deserialize(value.value())?);
        }
        Ok(vehicles)
    }

    /// Delete a vehicle by ID
    pub async fn delete_vehicle(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(VEHICLES)?;
            let removed = table.remove(id)?.is_some();
            if !removed {
                return Ok(false);
            }
        }
        write_txn.commit()?;
        debug!("Deleted vehicle: {}", id);
        Ok(true)
    }
}
