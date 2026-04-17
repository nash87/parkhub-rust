//! Visitor registrations (guests hosted by internal users).

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};
use tracing::debug;

use parkhub_common::models::Visitor;

use super::{Database, VISITORS};

impl Database {
    /// Save a visitor registration
    pub async fn save_visitor(&self, visitor: &Visitor) -> Result<()> {
        let id = visitor.id.to_string();
        let data = self.serialize(visitor)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(VISITORS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved visitor: {}", visitor.id);
        Ok(())
    }

    /// Get a visitor by ID
    pub async fn get_visitor(&self, id: &str) -> Result<Option<Visitor>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(VISITORS)?;
        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List visitors by host user ID
    pub async fn list_visitors_by_host(&self, host_user_id: &str) -> Result<Vec<Visitor>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(VISITORS)?;

        let mut visitors = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let visitor: Visitor = self.deserialize(value.value())?;
            if visitor.host_user_id.to_string() == host_user_id {
                visitors.push(visitor);
            }
        }
        visitors.sort_by(|a: &Visitor, b: &Visitor| b.created_at.cmp(&a.created_at));
        Ok(visitors)
    }

    /// List all visitors (admin)
    pub async fn list_all_visitors(&self) -> Result<Vec<Visitor>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(VISITORS)?;

        let mut visitors = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            visitors.push(self.deserialize(value.value())?);
        }
        visitors.sort_by(|a: &Visitor, b: &Visitor| b.created_at.cmp(&a.created_at));
        Ok(visitors)
    }

    /// Delete a visitor by ID
    pub async fn delete_visitor(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(VISITORS)?;
            table.remove(id)?;
        }
        let committed = write_txn.commit().is_ok();
        drop(db);
        Ok(committed)
    }
}
