//! Persistent audit log CRUD + export listings (all / limit).

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};

use super::{AUDIT_LOG, AuditLogEntry, Database};

impl Database {
    /// Save an audit log entry
    pub async fn save_audit_log(&self, entry: &AuditLogEntry) -> Result<()> {
        let id = entry.id.to_string();
        let data = self.serialize(entry)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(AUDIT_LOG)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// List recent audit log entries (most recent first, limited)
    pub async fn list_audit_log(&self, limit: usize) -> Result<Vec<AuditLogEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(AUDIT_LOG)?;

        let mut entries: Vec<AuditLogEntry> = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            entries.push(self.deserialize(value.value())?);
        }
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        entries.truncate(limit);
        Ok(entries)
    }

    /// List all audit log entries (no limit) for export and filtered queries.
    pub async fn list_all_audit_log(&self) -> Result<Vec<AuditLogEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(AUDIT_LOG)?;

        let mut entries: Vec<AuditLogEntry> = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            entries.push(self.deserialize(value.value())?);
        }
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(entries)
    }
}
