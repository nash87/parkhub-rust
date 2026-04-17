//! Free-form string-keyed settings (admin config, feature flags, etc.).

use anyhow::Result;
use redb::ReadableDatabase;

use super::{Database, SETTINGS};

impl Database {
    /// Get a setting value
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SETTINGS)?;

        Ok(table.get(key)?.map(|value| value.value().to_string()))
    }

    /// Set a setting value
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(SETTINGS)?;
            table.insert(key, value)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
