//! Favorites: per-user pinned parking slots.

use anyhow::Result;
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use super::{Database, FAVORITES};

/// A user's favorite parking slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub user_id: Uuid,
    pub slot_id: Uuid,
    pub lot_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl Database {
    /// Save a favorite (user pins a parking slot)
    pub async fn save_favorite(&self, fav: &Favorite) -> Result<()> {
        let key = format!("{}:{}", fav.user_id, fav.slot_id);
        let data = self.serialize(fav)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(FAVORITES)?;
            table.insert(key.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved favorite: user={}, slot={}", fav.user_id, fav.slot_id);
        Ok(())
    }

    /// List all favorites for a user
    pub async fn list_favorites_by_user(&self, user_id: &str) -> Result<Vec<Favorite>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(FAVORITES)?;

        let prefix = format!("{user_id}:");
        let mut favs = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            if key.value().starts_with(&prefix) {
                favs.push(self.deserialize(value.value())?);
            }
        }
        Ok(favs)
    }

    /// Delete a favorite by `user_id` and `slot_id`
    pub async fn delete_favorite(&self, user_id: &str, slot_id: &str) -> Result<bool> {
        let key = format!("{user_id}:{slot_id}");

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(FAVORITES)?;
            let result = table.remove(key.as_str())?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted favorite: user={}, slot={}", user_id, slot_id);
        }
        Ok(existed)
    }
}
