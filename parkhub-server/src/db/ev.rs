//! EV chargers and charging-session records.

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};
use tracing::debug;

use parkhub_common::models::{ChargingSession, EvCharger};

use super::{CHARGING_SESSIONS, Database, EV_CHARGERS};

impl Database {
    // ── EV Chargers ──

    /// Save an EV charger
    pub async fn save_charger(&self, charger: &EvCharger) -> Result<()> {
        let id = charger.id.to_string();
        let data = self.serialize(charger)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(EV_CHARGERS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved EV charger: {}", charger.id);
        Ok(())
    }

    /// Get a charger by ID
    pub async fn get_charger(&self, id: &str) -> Result<Option<EvCharger>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(EV_CHARGERS)?;
        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List chargers by lot ID
    pub async fn list_chargers_by_lot(&self, lot_id: &str) -> Result<Vec<EvCharger>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(EV_CHARGERS)?;
        let mut chargers = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let charger: EvCharger = self.deserialize(value.value())?;
            if charger.lot_id.to_string() == lot_id {
                chargers.push(charger);
            }
        }
        Ok(chargers)
    }

    /// List all chargers
    pub async fn list_all_chargers(&self) -> Result<Vec<EvCharger>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(EV_CHARGERS)?;
        let mut chargers = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            chargers.push(self.deserialize(value.value())?);
        }
        Ok(chargers)
    }

    // ── Charging Sessions ──

    /// Save a charging session
    pub async fn save_charging_session(&self, session: &ChargingSession) -> Result<()> {
        let id = session.id.to_string();
        let data = self.serialize(session)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(CHARGING_SESSIONS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved charging session: {}", session.id);
        Ok(())
    }

    /// List charging sessions by user
    pub async fn list_charging_sessions_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<ChargingSession>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(CHARGING_SESSIONS)?;
        let mut sessions = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let session: ChargingSession = self.deserialize(value.value())?;
            if session.user_id.to_string() == user_id {
                sessions.push(session);
            }
        }
        sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        Ok(sessions)
    }

    /// List all charging sessions
    pub async fn list_all_charging_sessions(&self) -> Result<Vec<ChargingSession>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(CHARGING_SESSIONS)?;
        let mut sessions = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            sessions.push(self.deserialize(value.value())?);
        }
        Ok(sessions)
    }
}
