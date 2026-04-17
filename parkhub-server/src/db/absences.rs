//! Absence CRUD: per-user absence records and team-wide listings.

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};
use tracing::debug;

use parkhub_common::models::Absence;

use super::{ABSENCES, Database};

impl Database {
    /// Save an absence record
    pub async fn save_absence(&self, absence: &Absence) -> Result<()> {
        let id = absence.id.to_string();
        let data = self.serialize(absence)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(ABSENCES)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved absence: {}", absence.id);
        Ok(())
    }

    /// Get an absence by ID
    pub async fn get_absence(&self, id: &str) -> Result<Option<Absence>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(ABSENCES)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List absences for a specific user
    pub async fn list_absences_by_user(&self, user_id: &str) -> Result<Vec<Absence>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(ABSENCES)?;

        let mut absences = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let absence: Absence = self.deserialize(value.value())?;
            if absence.user_id.to_string() == user_id {
                absences.push(absence);
            }
        }
        Ok(absences)
    }

    /// List all absences (team view)
    pub async fn list_absences_team(&self) -> Result<Vec<Absence>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(ABSENCES)?;

        let mut absences = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            absences.push(self.deserialize(value.value())?);
        }
        Ok(absences)
    }

    /// Delete an absence
    pub async fn delete_absence(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(ABSENCES)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted absence: {}", id);
        }
        Ok(existed)
    }
}
