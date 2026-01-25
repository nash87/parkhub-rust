//! Database Module
//!
//! Uses redb for pure-Rust embedded database storage.

use anyhow::{Context, Result};
use redb::{Database as RedbDatabase, ReadableTable, TableDefinition};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

// Table definitions
const USERS: TableDefinition<&str, &[u8]> = TableDefinition::new("users");
const BOOKINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("bookings");
const PARKING_LOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("parking_lots");
const SLOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("slots");
const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");

/// Database wrapper
pub struct Database {
    inner: Arc<RwLock<RedbDatabase>>,
}

impl Database {
    /// Open or create a database
    pub fn open(path: &Path) -> Result<Self> {
        let db = RedbDatabase::create(path).context("Failed to create/open database")?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(USERS)?;
            let _ = write_txn.open_table(BOOKINGS)?;
            let _ = write_txn.open_table(PARKING_LOTS)?;
            let _ = write_txn.open_table(SLOTS)?;
            let _ = write_txn.open_table(SETTINGS)?;
        }
        write_txn.commit()?;

        Ok(Self {
            inner: Arc::new(RwLock::new(db)),
        })
    }

    /// Get a setting value
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(SETTINGS)?;

        match table.get(key)? {
            Some(value) => Ok(Some(value.value().to_string())),
            None => Ok(None),
        }
    }

    /// Set a setting value
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(SETTINGS)?;
            table.insert(key, value)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Store a user (JSON serialized)
    pub async fn save_user(&self, id: &str, data: &[u8]) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(USERS)?;
            table.insert(id, data)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Get a user by ID
    pub async fn get_user(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(USERS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(value.value().to_vec())),
            None => Ok(None),
        }
    }

    /// Store a booking
    pub async fn save_booking(&self, id: &str, data: &[u8]) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(BOOKINGS)?;
            table.insert(id, data)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Get a booking by ID
    pub async fn get_booking(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(BOOKINGS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(value.value().to_vec())),
            None => Ok(None),
        }
    }

    /// List all bookings
    pub async fn list_bookings(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(BOOKINGS)?;

        let mut results = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            results.push((key.value().to_string(), value.value().to_vec()));
        }
        Ok(results)
    }

    /// Store a parking lot
    pub async fn save_parking_lot(&self, id: &str, data: &[u8]) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(PARKING_LOTS)?;
            table.insert(id, data)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Get database statistics
    pub async fn stats(&self) -> Result<DbStats> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;

        let users = read_txn.open_table(USERS)?.len()?;
        let bookings = read_txn.open_table(BOOKINGS)?.len()?;
        let lots = read_txn.open_table(PARKING_LOTS)?.len()?;

        Ok(DbStats {
            users: users as u64,
            bookings: bookings as u64,
            parking_lots: lots as u64,
        })
    }
}

/// Database statistics
pub struct DbStats {
    pub users: u64,
    pub bookings: u64,
    pub parking_lots: u64,
}
