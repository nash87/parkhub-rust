//! Database Module
//!
//! Provides persistent storage using redb (pure Rust embedded database).
//! Supports optional AES-256-GCM encryption for data at rest.
//!
//! The public `Database` type is split across domain-oriented sub-modules.
//! `mod.rs` owns the struct definition, lifecycle (open / clear / stats /
//! setup) and JSON (de)serialization plumbing; each sub-module adds
//! `impl Database { ... }` blocks for its domain's CRUD.

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use rand::Rng;
use redb::{
    Database as RedbDatabase, ReadableDatabase, ReadableTable, ReadableTableMetadata,
    TableDefinition,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

mod absences;
mod audit_log;
mod bookings;
mod communications;
mod encryption;
mod ev;
mod favorites;
mod invoice_counters;
mod lots;
mod sessions;
mod settings;
mod stripe_events;
mod translations;
mod users;
mod vehicles;
mod visitors;

#[cfg(test)]
mod tests;

use encryption::Encryptor;

pub use favorites::Favorite;
pub use lots::Zone;
pub use sessions::Session;

// ═══════════════════════════════════════════════════════════════════════════════
// TABLE DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════════

pub(crate) const USERS: TableDefinition<&str, &[u8]> = TableDefinition::new("users");
pub(crate) const USERS_BY_USERNAME: TableDefinition<&str, &str> =
    TableDefinition::new("users_by_username");
pub(crate) const USERS_BY_EMAIL: TableDefinition<&str, &str> =
    TableDefinition::new("users_by_email");
pub(crate) const SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");
pub(crate) const BOOKINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("bookings");
pub(crate) const BOOKINGS_BY_USER: TableDefinition<&str, &str> =
    TableDefinition::new("bookings_by_user");
pub(crate) const PARKING_LOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("parking_lots");
pub(crate) const PARKING_SLOTS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("parking_slots");
pub(crate) const SLOTS_BY_LOT: TableDefinition<&str, &[u8]> = TableDefinition::new("slots_by_lot");
pub(crate) const VEHICLES: TableDefinition<&str, &[u8]> = TableDefinition::new("vehicles");
pub(crate) const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");
pub(crate) const CREDIT_TRANSACTIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("credit_transactions");
pub(crate) const ABSENCES: TableDefinition<&str, &[u8]> = TableDefinition::new("absences");
pub(crate) const WAITLIST: TableDefinition<&str, &[u8]> = TableDefinition::new("waitlist");
pub(crate) const GUEST_BOOKINGS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("guest_bookings");
pub(crate) const SWAP_REQUESTS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("swap_requests");
pub(crate) const RECURRING_BOOKINGS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("recurring_bookings");
pub(crate) const ANNOUNCEMENTS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("announcements");
pub(crate) const NOTIFICATIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("notifications");
pub(crate) const WEBHOOKS: TableDefinition<&str, &[u8]> = TableDefinition::new("webhooks");
pub(crate) const PUSH_SUBSCRIPTIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("push_subscriptions");
pub(crate) const ZONES: TableDefinition<&str, &[u8]> = TableDefinition::new("zones");
pub(crate) const FAVORITES: TableDefinition<&str, &[u8]> = TableDefinition::new("favorites");
pub(crate) const AUDIT_LOG: TableDefinition<&str, &[u8]> = TableDefinition::new("audit_log");
pub(crate) const TRANSLATION_PROPOSALS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("translation_proposals");
pub(crate) const TRANSLATION_VOTES: TableDefinition<&str, &[u8]> =
    TableDefinition::new("translation_votes");
pub(crate) const TRANSLATION_OVERRIDES: TableDefinition<&str, &[u8]> =
    TableDefinition::new("translation_overrides");
pub(crate) const VISITORS: TableDefinition<&str, &[u8]> = TableDefinition::new("visitors");
pub(crate) const EV_CHARGERS: TableDefinition<&str, &[u8]> = TableDefinition::new("ev_chargers");
pub(crate) const CHARGING_SESSIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("charging_sessions");
/// Stripe webhook event log (idempotency). Key: Stripe `evt_...` id.
/// Value: event type (e.g. `checkout.session.completed`). Presence of the key
/// means the event was already processed — retries short-circuit to 200 OK
/// before any credit mutation, preventing double-credit.
pub(crate) const STRIPE_EVENTS: TableDefinition<&str, &str> = TableDefinition::new("stripe_events");

// Settings keys
const SETTING_SETUP_COMPLETED: &str = "setup_completed";
const SETTING_DB_VERSION: &str = "db_version";
const SETTING_ENCRYPTION_SALT: &str = "encryption_salt";

const CURRENT_DB_VERSION: &str = "1";

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for database initialization
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the data directory
    pub path: PathBuf,
    /// Enable encryption for stored data
    pub encryption_enabled: bool,
    /// Passphrase for encryption (required if `encryption_enabled`)
    pub passphrase: Option<String>,
    /// Create database if it doesn't exist
    pub create_if_missing: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// WEBHOOK
// ═══════════════════════════════════════════════════════════════════════════════

/// Webhook configuration for event delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUSH SUBSCRIPTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Web Push subscription stored per user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// AUDIT LOG
// ═══════════════════════════════════════════════════════════════════════════════

/// Persistent audit log entry (stored in DB, exposed via API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub details: Option<String>,
    /// Target resource type (e.g. "booking", "user", "lot")
    #[serde(default)]
    pub target_type: Option<String>,
    /// Target resource ID
    #[serde(default)]
    pub target_id: Option<String>,
    /// Client IP address
    #[serde(default)]
    pub ip_address: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE STATISTICS
// ═══════════════════════════════════════════════════════════════════════════════

/// Database statistics
#[derive(Debug, Clone, Default)]
pub struct DatabaseStats {
    pub users: u64,
    pub bookings: u64,
    pub parking_lots: u64,
    pub slots: u64,
    pub sessions: u64,
    pub vehicles: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Compute (skip, per_page_usize) from 1-based page and per_page inputs.
/// Returns (number of items to skip, items per page).
pub(crate) fn pagination_offset(page: i32, per_page: i32) -> (usize, usize) {
    let per_page = per_page.max(1) as usize;
    let skip = (page.max(1) as usize - 1) * per_page;
    (skip, per_page)
}

/// Main database wrapper with optional encryption support
#[derive(Clone)]
pub struct Database {
    pub(crate) inner: Arc<RwLock<RedbDatabase>>,
    encryptor: Option<Encryptor>,
    encryption_enabled: bool,
}

impl Database {
    /// Open or create a database with the given configuration
    pub fn open(config: &DatabaseConfig) -> Result<Self> {
        let db_path = config.path.join("parkhub.redb");

        // Check if database exists
        let db_exists = db_path.exists();
        if !db_exists && !config.create_if_missing {
            return Err(anyhow!(
                "Database not found at {} and create_if_missing is false",
                db_path.display()
            ));
        }

        // Create parent directories if needed
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create data directory")?;
        }

        info!("Opening database at {:?}", db_path);
        let db = RedbDatabase::create(&db_path).context("Failed to create/open database")?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(USERS)?;
            let _ = write_txn.open_table(USERS_BY_USERNAME)?;
            let _ = write_txn.open_table(USERS_BY_EMAIL)?;
            let _ = write_txn.open_table(SESSIONS)?;
            let _ = write_txn.open_table(BOOKINGS)?;
            let _ = write_txn.open_table(BOOKINGS_BY_USER)?;
            let _ = write_txn.open_table(PARKING_LOTS)?;
            let _ = write_txn.open_table(PARKING_SLOTS)?;
            let _ = write_txn.open_table(SLOTS_BY_LOT)?;
            let _ = write_txn.open_table(VEHICLES)?;
            let _ = write_txn.open_table(SETTINGS)?;
            let _ = write_txn.open_table(CREDIT_TRANSACTIONS)?;
            let _ = write_txn.open_table(ABSENCES)?;
            let _ = write_txn.open_table(WAITLIST)?;
            let _ = write_txn.open_table(GUEST_BOOKINGS)?;
            let _ = write_txn.open_table(SWAP_REQUESTS)?;
            let _ = write_txn.open_table(RECURRING_BOOKINGS)?;
            let _ = write_txn.open_table(ANNOUNCEMENTS)?;
            let _ = write_txn.open_table(NOTIFICATIONS)?;
            let _ = write_txn.open_table(WEBHOOKS)?;
            let _ = write_txn.open_table(PUSH_SUBSCRIPTIONS)?;
            let _ = write_txn.open_table(ZONES)?;
            let _ = write_txn.open_table(FAVORITES)?;
            let _ = write_txn.open_table(AUDIT_LOG)?;
            let _ = write_txn.open_table(TRANSLATION_PROPOSALS)?;
            let _ = write_txn.open_table(TRANSLATION_VOTES)?;
            let _ = write_txn.open_table(TRANSLATION_OVERRIDES)?;
            let _ = write_txn.open_table(VISITORS)?;
            let _ = write_txn.open_table(EV_CHARGERS)?;
            let _ = write_txn.open_table(CHARGING_SESSIONS)?;
            let _ = write_txn.open_table(STRIPE_EVENTS)?;
        }
        write_txn.commit()?;

        // Set up encryption if enabled
        let encryptor = if config.encryption_enabled {
            let passphrase = config
                .passphrase
                .as_ref()
                .ok_or_else(|| anyhow!("Encryption enabled but no passphrase provided"))?;

            // Get or create salt
            let salt = {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(SETTINGS)?;
                if let Some(value) = table.get(SETTING_ENCRYPTION_SALT)? {
                    hex::decode(value.value()).context("Invalid salt in database")?
                } else {
                    // Generate new salt
                    let mut salt = [0u8; 32];
                    rand::rng().fill_bytes(&mut salt);

                    // Store salt
                    let write_txn = db.begin_write()?;
                    {
                        let mut table = write_txn.open_table(SETTINGS)?;
                        table.insert(SETTING_ENCRYPTION_SALT, hex::encode(salt).as_str())?;
                    }
                    write_txn.commit()?;

                    salt.to_vec()
                }
            };

            Some(Encryptor::new(passphrase, &salt)?)
        } else {
            None
        };

        // Set database version if new
        if !db_exists {
            let write_txn = db.begin_write()?;
            {
                let mut table = write_txn.open_table(SETTINGS)?;
                table.insert(SETTING_DB_VERSION, CURRENT_DB_VERSION)?;
            }
            write_txn.commit()?;
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(db)),
            encryptor,
            encryption_enabled: config.encryption_enabled,
        })
    }

    /// Check if encryption is enabled
    pub const fn is_encrypted(&self) -> bool {
        self.encryption_enabled
    }

    /// Clear all data tables for demo reset. Preserves DB structure and settings.
    /// Admin user must be re-created after calling this.
    pub async fn clear_all_data(&self) -> Result<()> {
        // Helper: drain a table by collecting keys first, then removing them.
        // redb's borrow rules prevent removing while iterating.
        macro_rules! drain_table {
            ($txn:expr, $table:expr) => {{
                let mut t = $txn.open_table($table)?;
                let keys: Vec<String> = {
                    let mut keys = Vec::new();
                    let mut iter = t.iter()?;
                    while let Some(entry) = iter.next() {
                        let entry = entry?;
                        keys.push(entry.0.value().to_string());
                    }
                    keys
                };
                for key in &keys {
                    t.remove(key.as_str())?;
                }
            }};
        }

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        drain_table!(write_txn, USERS);
        drain_table!(write_txn, USERS_BY_USERNAME);
        drain_table!(write_txn, USERS_BY_EMAIL);
        drain_table!(write_txn, SESSIONS);
        drain_table!(write_txn, BOOKINGS);
        drain_table!(write_txn, BOOKINGS_BY_USER);
        drain_table!(write_txn, PARKING_LOTS);
        drain_table!(write_txn, PARKING_SLOTS);
        drain_table!(write_txn, SLOTS_BY_LOT);
        drain_table!(write_txn, VEHICLES);
        drain_table!(write_txn, CREDIT_TRANSACTIONS);
        drain_table!(write_txn, ABSENCES);
        drain_table!(write_txn, WAITLIST);
        drain_table!(write_txn, GUEST_BOOKINGS);
        drain_table!(write_txn, SWAP_REQUESTS);
        drain_table!(write_txn, RECURRING_BOOKINGS);
        drain_table!(write_txn, ANNOUNCEMENTS);
        drain_table!(write_txn, NOTIFICATIONS);
        drain_table!(write_txn, WEBHOOKS);
        drain_table!(write_txn, PUSH_SUBSCRIPTIONS);
        drain_table!(write_txn, ZONES);
        drain_table!(write_txn, FAVORITES);
        drain_table!(write_txn, AUDIT_LOG);
        drain_table!(write_txn, TRANSLATION_PROPOSALS);
        drain_table!(write_txn, TRANSLATION_VOTES);
        drain_table!(write_txn, TRANSLATION_OVERRIDES);
        drain_table!(write_txn, VISITORS);
        drain_table!(write_txn, EV_CHARGERS);
        drain_table!(write_txn, CHARGING_SESSIONS);
        drain_table!(write_txn, STRIPE_EVENTS);
        // Preserve SETTINGS table (encryption salt, setup status, etc.)
        write_txn.commit()?;
        info!("All data tables cleared for demo reset");
        Ok(())
    }

    /// Check if the database is fresh (no setup completed)
    pub async fn is_fresh(&self) -> Result<bool> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SETTINGS)?;

        Ok(table
            .get(SETTING_SETUP_COMPLETED)?
            .is_none_or(|value| value.value() != "true"))
    }

    /// Mark the initial setup as completed
    pub async fn mark_setup_completed(&self) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(SETTINGS)?;
            table.insert(SETTING_SETUP_COMPLETED, "true")?;
        }
        write_txn.commit()?;
        info!("Database setup marked as completed");
        Ok(())
    }

    /// Get database statistics
    pub async fn stats(&self) -> Result<DatabaseStats> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        Ok(DatabaseStats {
            users: read_txn.open_table(USERS)?.len()?,
            bookings: read_txn.open_table(BOOKINGS)?.len()?,
            parking_lots: read_txn.open_table(PARKING_LOTS)?.len()?,
            slots: read_txn.open_table(PARKING_SLOTS)?.len()?,
            sessions: read_txn.open_table(SESSIONS)?.len()?,
            vehicles: read_txn.open_table(VEHICLES)?.len()?,
        })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // INTERNAL HELPERS
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(value).context("Failed to serialize")?;
        if let Some(ref enc) = self.encryptor {
            enc.encrypt(&json)
        } else {
            Ok(json)
        }
    }

    pub(crate) fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        let json = if let Some(ref enc) = self.encryptor {
            enc.decrypt(data)?
        } else {
            data.to_vec()
        };
        serde_json::from_slice(&json).context("Failed to deserialize")
    }
}
