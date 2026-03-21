//! Database Module
//!
//! Provides persistent storage using redb (pure Rust embedded database).
//! Supports optional AES-256-GCM encryption for data at rest.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use redb::{Database as RedbDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use parkhub_common::models::{
    Absence, Announcement, Booking, GuestBooking, Notification, ParkingLot, ParkingSlot,
    ProposalStatus, RecurringBooking, SwapRequest, TranslationOverride, TranslationProposal,
    TranslationVote, User, Vehicle, WaitlistEntry,
};

// ═══════════════════════════════════════════════════════════════════════════════
// TABLE DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════════

const USERS: TableDefinition<&str, &[u8]> = TableDefinition::new("users");
const USERS_BY_USERNAME: TableDefinition<&str, &str> = TableDefinition::new("users_by_username");
const USERS_BY_EMAIL: TableDefinition<&str, &str> = TableDefinition::new("users_by_email");
const SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");
const BOOKINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("bookings");
const BOOKINGS_BY_USER: TableDefinition<&str, &str> = TableDefinition::new("bookings_by_user");
const PARKING_LOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("parking_lots");
const PARKING_SLOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("parking_slots");
const SLOTS_BY_LOT: TableDefinition<&str, &[u8]> = TableDefinition::new("slots_by_lot");
const VEHICLES: TableDefinition<&str, &[u8]> = TableDefinition::new("vehicles");
const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");
const CREDIT_TRANSACTIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("credit_transactions");
const ABSENCES: TableDefinition<&str, &[u8]> = TableDefinition::new("absences");
const WAITLIST: TableDefinition<&str, &[u8]> = TableDefinition::new("waitlist");
const GUEST_BOOKINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("guest_bookings");
const SWAP_REQUESTS: TableDefinition<&str, &[u8]> = TableDefinition::new("swap_requests");
const RECURRING_BOOKINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("recurring_bookings");
const ANNOUNCEMENTS: TableDefinition<&str, &[u8]> = TableDefinition::new("announcements");
const NOTIFICATIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("notifications");
const WEBHOOKS: TableDefinition<&str, &[u8]> = TableDefinition::new("webhooks");
const PUSH_SUBSCRIPTIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("push_subscriptions");
const ZONES: TableDefinition<&str, &[u8]> = TableDefinition::new("zones");
const FAVORITES: TableDefinition<&str, &[u8]> = TableDefinition::new("favorites");
const AUDIT_LOG: TableDefinition<&str, &[u8]> = TableDefinition::new("audit_log");
const TRANSLATION_PROPOSALS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("translation_proposals");
const TRANSLATION_VOTES: TableDefinition<&str, &[u8]> = TableDefinition::new("translation_votes");
const TRANSLATION_OVERRIDES: TableDefinition<&str, &[u8]> =
    TableDefinition::new("translation_overrides");

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
// SESSION
// ═══════════════════════════════════════════════════════════════════════════════

/// User session for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
    pub refresh_token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with the given duration in hours.
    ///
    /// `username` and `role` are stored for audit/logging purposes.
    pub fn new(user_id: Uuid, duration_hours: i64, username: &str, role: &str) -> Self {
        let now = Utc::now();
        // Use cryptographically random refresh token (not a UUID — UUIDs have
        // a fixed structure that reduces effective entropy).
        let mut rng_bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rng(), &mut rng_bytes);
        let refresh_token = format!("rt_{}", hex::encode(rng_bytes));
        Self {
            user_id,
            username: username.to_string(),
            role: role.to_string(),
            refresh_token,
            created_at: now,
            expires_at: now + chrono::Duration::hours(duration_hours),
        }
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
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
// ZONE
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// FAVORITE
// ═══════════════════════════════════════════════════════════════════════════════

/// A user's favorite parking slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub user_id: Uuid,
    pub slot_id: Uuid,
    pub lot_id: Uuid,
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
// ENCRYPTION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

struct Encryptor {
    cipher: Aes256Gcm,
}

/// PBKDF2 iteration count for key derivation.
///
/// 600 000 iterations with HMAC-SHA-256 meets the NIST SP 800-132 (2023)
/// recommendation. This is applied once at database open time, not on every
/// request, so the cost is paid only once per process start.
const PBKDF2_ITERATIONS: u32 = 600_000;

impl Encryptor {
    fn new(passphrase: &str, salt: &[u8]) -> Result<Self> {
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow!("Failed to create cipher: {e}"))?;
        Ok(Self { cipher })
    }

    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {e}"))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data: too short"));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {e}"))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Main database wrapper with optional encryption support
pub struct Database {
    inner: Arc<RwLock<RedbDatabase>>,
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

    fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(value).context("Failed to serialize")?;
        if let Some(ref enc) = self.encryptor {
            enc.encrypt(&json)
        } else {
            Ok(json)
        }
    }

    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        let json = if let Some(ref enc) = self.encryptor {
            enc.decrypt(data)?
        } else {
            data.to_vec()
        };
        serde_json::from_slice(&json).context("Failed to deserialize")
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a session (access token -> session data)
    pub async fn save_session(&self, token: &str, session: &Session) -> Result<()> {
        let data = self.serialize(session)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(SESSIONS)?;
            table.insert(token, data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved session for user: {}", session.username);
        Ok(())
    }

    /// Get a session by token
    pub async fn get_session(&self, token: &str) -> Result<Option<Session>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SESSIONS)?;

        match table.get(token)? {
            Some(value) => {
                let session: Session = self.deserialize(value.value())?;
                // Check if expired
                if session.expires_at < Utc::now() {
                    Ok(None)
                } else {
                    Ok(Some(session))
                }
            }
            None => Ok(None),
        }
    }

    /// Find a session by its refresh token (scans all sessions)
    ///
    /// Returns a tuple of (`access_token`, session) if found and not expired.
    pub async fn get_session_by_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<Option<(String, Session)>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SESSIONS)?;

        for entry in table.iter()? {
            let (key, value) = entry?;
            let access_token = key.value().to_string();
            let session: Session = self.deserialize(value.value())?;
            if session.refresh_token == refresh_token {
                if session.is_expired() {
                    return Ok(None);
                }
                return Ok(Some((access_token, session)));
            }
        }
        Ok(None)
    }

    /// Delete all sessions belonging to a specific user.
    ///
    /// Scans every session, deserializes it, and removes entries whose
    /// `user_id` matches the given ID. Returns the number of deleted sessions.
    pub async fn delete_sessions_by_user(&self, user_id: Uuid) -> Result<u64> {
        let db = self.inner.write().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(SESSIONS)?;

        // Collect tokens to delete (cannot mutate while iterating)
        let mut tokens_to_delete = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            let session: Session = self.deserialize(value.value())?;
            if session.user_id == user_id {
                tokens_to_delete.push(key.value().to_string());
            }
        }
        drop(table);
        drop(read_txn);

        let count = tokens_to_delete.len() as u64;
        if count > 0 {
            let write_txn = db.begin_write()?;
            drop(db);
            {
                let mut table = write_txn.open_table(SESSIONS)?;
                for token in &tokens_to_delete {
                    table.remove(token.as_str())?;
                }
            }
            write_txn.commit()?;
            debug!("Deleted {} session(s) for user {}", count, user_id);
        }
        Ok(count)
    }

    /// Delete a session
    pub async fn delete_session(&self, token: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(SESSIONS)?;
            let result = table.remove(token)?;
            result.is_some()
        };
        write_txn.commit()?;
        Ok(existed)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // USER OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a user to the database
    pub async fn save_user(&self, user: &User) -> Result<()> {
        let id = user.id.to_string();
        let data = self.serialize(user)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(USERS)?;
            table.insert(id.as_str(), data.as_slice())?;

            // Update username index
            let mut idx = write_txn.open_table(USERS_BY_USERNAME)?;
            idx.insert(user.username.as_str(), id.as_str())?;

            // Update email index
            let mut email_idx = write_txn.open_table(USERS_BY_EMAIL)?;
            email_idx.insert(user.email.as_str(), id.as_str())?;
        }
        write_txn.commit()?;
        debug!("Saved user: {} ({})", user.username, user.id);
        Ok(())
    }

    /// Get a user by ID (string)
    pub async fn get_user(&self, id: &str) -> Result<Option<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(USERS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get a user by username
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        // Look up user ID from username index
        let idx = read_txn.open_table(USERS_BY_USERNAME)?;
        let user_id = match idx.get(username)? {
            Some(id) => id.value().to_string(),
            None => return Ok(None),
        };

        // Get user data
        let table = read_txn.open_table(USERS)?;
        match table.get(user_id.as_str())? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get a user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        // Look up user ID from email index
        let idx = read_txn.open_table(USERS_BY_EMAIL)?;
        let user_id = match idx.get(email)? {
            Some(id) => id.value().to_string(),
            None => return Ok(None),
        };

        // Get user data
        let table = read_txn.open_table(USERS)?;
        match table.get(user_id.as_str())? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all users
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(USERS)?;

        let mut users = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            users.push(self.deserialize(value.value())?);
        }
        Ok(users)
    }

    /// Delete a user
    pub async fn delete_user(&self, id: &str) -> Result<bool> {
        // First get the user to find the username/email
        let Some(user) = self.get_user(id).await? else {
            return Ok(false);
        };

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(USERS)?;
            table.remove(id)?;

            let mut idx = write_txn.open_table(USERS_BY_USERNAME)?;
            idx.remove(user.username.as_str())?;

            let mut email_idx = write_txn.open_table(USERS_BY_EMAIL)?;
            email_idx.remove(user.email.as_str())?;
        }
        write_txn.commit()?;
        debug!("Deleted user: {}", id);
        Ok(true)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PARKING LOT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // PARKING SLOT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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
            let r = table.remove(id)?.is_some();
            r
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

    // ═══════════════════════════════════════════════════════════════════════════
    // BOOKING OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a booking
    pub async fn save_booking(&self, booking: &Booking) -> Result<()> {
        let id = booking.id.to_string();
        let user_id = booking.user_id.to_string();
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;

            // Maintain user → booking secondary index
            let mut idx = write_txn.open_table(BOOKINGS_BY_USER)?;
            let idx_key = format!("{user_id}:{id}");
            idx.insert(idx_key.as_str(), id.as_str())?;
        }
        write_txn.commit()?;
        debug!("Saved booking: {}", booking.id);
        Ok(())
    }

    /// Get a booking by ID (string)
    pub async fn get_booking(&self, id: &str) -> Result<Option<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(BOOKINGS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all bookings
    pub async fn list_bookings(&self) -> Result<Vec<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            bookings.push(self.deserialize(value.value())?);
        }
        Ok(bookings)
    }

    /// Get bookings for a user using the BOOKINGS_BY_USER secondary index.
    ///
    /// O(k) where k = number of bookings for this user, instead of O(n) over
    /// all bookings.
    pub async fn list_bookings_by_user(&self, user_id: &str) -> Result<Vec<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        let idx = read_txn.open_table(BOOKINGS_BY_USER)?;
        let bookings_table = read_txn.open_table(BOOKINGS)?;

        let prefix = format!("{user_id}:");
        let mut bookings = Vec::new();

        for entry in idx.iter()? {
            let (key, booking_id_val) = entry?;
            if !key.value().starts_with(&prefix) {
                continue;
            }
            let booking_id = booking_id_val.value();
            if let Some(data) = bookings_table.get(booking_id)? {
                bookings.push(self.deserialize(data.value())?);
            }
        }
        Ok(bookings)
    }

    /// Delete a booking
    pub async fn delete_booking(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;

        // Read pass: find the user_id to remove the secondary-index entry
        let user_id_opt: Option<String> = {
            let read_txn = db.begin_read()?;
            let table = read_txn.open_table(BOOKINGS)?;
            match table.get(id)? {
                Some(value) => {
                    let booking: Booking = self.deserialize(value.value())?;
                    Some(booking.user_id.to_string())
                }
                None => None,
            }
        };

        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(BOOKINGS)?;
            let result = table.remove(id)?;
            // Remove secondary index entry if booking was found
            if result.is_some() {
                if let Some(ref uid) = user_id_opt {
                    let mut idx = write_txn.open_table(BOOKINGS_BY_USER)?;
                    let idx_key = format!("{uid}:{id}");
                    idx.remove(idx_key.as_str())?;
                }
            }
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted booking: {}", id);
        }
        Ok(existed)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VEHICLE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    /// GDPR Art. 17 — Anonymize a user: scrub PII while keeping booking records.
    /// Atomically replaces user's name/email/username/password with placeholder values,
    /// removes old index entries, and deletes all linked vehicle records.
    pub async fn anonymize_user(&self, user_id: &str) -> Result<bool> {
        let Some(user) = self.get_user(user_id).await? else {
            return Ok(false);
        };

        let old_username = user.username.clone();
        let old_email = user.email.clone();
        let anon_id = format!("deleted-{}", Uuid::new_v4());
        let anon_email = format!("{anon_id}@deleted.invalid");
        let anon_password = format!("DELETED_{}", Uuid::new_v4());

        // Anonymize user record + clean indexes atomically
        let mut anon_user = user;
        anon_user.name = "[Deleted User]".to_string();
        anon_user.email = anon_email.clone();
        anon_user.username = anon_id.clone();
        anon_user.password_hash = anon_password;

        let user_data = self.serialize(&anon_user)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            // Overwrite user record
            let mut table = write_txn.open_table(USERS)?;
            table.insert(user_id, user_data.as_slice())?;

            // Remove stale index entries and add anonymized ones
            let mut idx = write_txn.open_table(USERS_BY_USERNAME)?;
            let _ = idx.remove(old_username.as_str());
            idx.insert(anon_id.as_str(), user_id)?;

            let mut email_idx = write_txn.open_table(USERS_BY_EMAIL)?;
            let _ = email_idx.remove(old_email.as_str());
            email_idx.insert(anon_email.as_str(), user_id)?;
        }
        write_txn.commit()?;

        // Delete all vehicles (personal data — can be deleted per GDPR Art. 17)
        let vehicles = self
            .list_vehicles_by_user(user_id)
            .await
            .unwrap_or_default();
        for vehicle in vehicles {
            if let Err(e) = self.delete_vehicle(&vehicle.id.to_string()).await {
                tracing::warn!("GDPR: failed to delete vehicle {}: {e}", vehicle.id);
            }
        }

        // Scrub license plate from bookings (keep records for accounting, strip PII)
        let bookings = self
            .list_bookings_by_user(user_id)
            .await
            .unwrap_or_default();
        for mut booking in bookings {
            booking.vehicle.license_plate = "[DELETED]".to_string();
            if let Err(e) = self.save_booking(&booking).await {
                tracing::warn!("GDPR: failed to scrub booking {}: {e}", booking.id);
            }
        }

        info!(
            "GDPR anonymization completed for user: {} → {}",
            user_id, anon_id
        );
        Ok(true)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SETTINGS OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // ABSENCE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // WAITLIST OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a waitlist entry
    pub async fn save_waitlist_entry(&self, entry: &WaitlistEntry) -> Result<()> {
        let id = entry.id.to_string();
        let data = self.serialize(entry)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(WAITLIST)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved waitlist entry: {}", entry.id);
        Ok(())
    }

    /// Get a waitlist entry by ID
    pub async fn get_waitlist_entry(&self, id: &str) -> Result<Option<WaitlistEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WAITLIST)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List waitlist entries for a user
    pub async fn list_waitlist_by_user(&self, user_id: &str) -> Result<Vec<WaitlistEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WAITLIST)?;

        let mut entries = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let waitlist_entry: WaitlistEntry = self.deserialize(value.value())?;
            if waitlist_entry.user_id.to_string() == user_id {
                entries.push(waitlist_entry);
            }
        }
        Ok(entries)
    }

    /// List all waitlist entries for a specific parking lot, ordered by creation time.
    pub async fn list_waitlist_by_lot(&self, lot_id: &str) -> Result<Vec<WaitlistEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WAITLIST)?;

        let mut entries = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let waitlist_entry: WaitlistEntry = self.deserialize(value.value())?;
            if waitlist_entry.lot_id.to_string() == lot_id {
                entries.push(waitlist_entry);
            }
        }
        // Sort by created_at so earlier waitlist entries are notified first
        entries.sort_by_key(|e| e.created_at);
        Ok(entries)
    }

    /// Delete a waitlist entry
    pub async fn delete_waitlist_entry(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(WAITLIST)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted waitlist entry: {}", id);
        }
        Ok(existed)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GUEST BOOKING OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a guest booking
    pub async fn save_guest_booking(&self, booking: &GuestBooking) -> Result<()> {
        let id = booking.id.to_string();
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(GUEST_BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved guest booking: {}", booking.id);
        Ok(())
    }

    /// Get a guest booking by ID
    pub async fn get_guest_booking(&self, id: &str) -> Result<Option<GuestBooking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(GUEST_BOOKINGS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all guest bookings
    pub async fn list_guest_bookings(&self) -> Result<Vec<GuestBooking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(GUEST_BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            bookings.push(self.deserialize(value.value())?);
        }
        Ok(bookings)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SWAP REQUEST OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a swap request
    pub async fn save_swap_request(&self, req: &SwapRequest) -> Result<()> {
        let id = req.id.to_string();
        let data = self.serialize(req)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(SWAP_REQUESTS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved swap request: {}", req.id);
        Ok(())
    }

    /// Get a swap request by ID
    pub async fn get_swap_request(&self, id: &str) -> Result<Option<SwapRequest>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SWAP_REQUESTS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List swap requests involving a user (as requester or target)
    pub async fn list_swap_requests_by_user(&self, user_id: &str) -> Result<Vec<SwapRequest>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SWAP_REQUESTS)?;

        let mut requests = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let req: SwapRequest = self.deserialize(value.value())?;
            if req.requester_id.to_string() == user_id || req.target_id.to_string() == user_id {
                requests.push(req);
            }
        }
        Ok(requests)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RECURRING BOOKING OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a recurring booking
    pub async fn save_recurring_booking(&self, booking: &RecurringBooking) -> Result<()> {
        let id = booking.id.to_string();
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(RECURRING_BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved recurring booking: {}", booking.id);
        Ok(())
    }

    /// List recurring bookings for a user
    pub async fn list_recurring_bookings_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<RecurringBooking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(RECURRING_BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let booking: RecurringBooking = self.deserialize(value.value())?;
            if booking.user_id.to_string() == user_id {
                bookings.push(booking);
            }
        }
        Ok(bookings)
    }

    /// Delete a recurring booking
    pub async fn delete_recurring_booking(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(RECURRING_BOOKINGS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted recurring booking: {}", id);
        }
        Ok(existed)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ANNOUNCEMENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save an announcement
    pub async fn save_announcement(&self, ann: &Announcement) -> Result<()> {
        let id = ann.id.to_string();
        let data = self.serialize(ann)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(ANNOUNCEMENTS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved announcement: {}", ann.id);
        Ok(())
    }

    /// List all announcements
    pub async fn list_announcements(&self) -> Result<Vec<Announcement>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(ANNOUNCEMENTS)?;

        let mut announcements = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            announcements.push(self.deserialize(value.value())?);
        }
        Ok(announcements)
    }

    /// Delete an announcement
    pub async fn delete_announcement(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(ANNOUNCEMENTS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted announcement: {}", id);
        }
        Ok(existed)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NOTIFICATION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a notification
    pub async fn save_notification(&self, notification: &Notification) -> Result<()> {
        let id = notification.id.to_string();
        let data = self.serialize(notification)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(NOTIFICATIONS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved notification: {}", notification.id);
        Ok(())
    }

    /// List notifications for a user
    pub async fn list_notifications_by_user(&self, user_id: &str) -> Result<Vec<Notification>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(NOTIFICATIONS)?;

        let mut notifications = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let notification: Notification = self.deserialize(value.value())?;
            if notification.user_id.to_string() == user_id {
                notifications.push(notification);
            }
        }
        Ok(notifications)
    }

    /// Mark a notification as read
    pub async fn mark_notification_read(&self, id: &str) -> Result<bool> {
        let Some(mut notification) = self.get_notification(id).await? else {
            return Ok(false);
        };

        notification.read = true;
        self.save_notification(&notification).await?;
        Ok(true)
    }

    /// Get a notification by ID (helper for `mark_notification_read`)
    async fn get_notification(&self, id: &str) -> Result<Option<Notification>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(NOTIFICATIONS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    // ── Credit Transactions ──

    pub async fn save_credit_transaction(
        &self,
        tx: &parkhub_common::models::CreditTransaction,
    ) -> Result<()> {
        let data = self.serialize(tx)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(CREDIT_TRANSACTIONS)?;
            table.insert(tx.id.to_string().as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub async fn list_credit_transactions_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<parkhub_common::models::CreditTransaction>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(CREDIT_TRANSACTIONS)?;
        let mut transactions = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let tx: parkhub_common::models::CreditTransaction = self.deserialize(value.value())?;
            if tx.user_id == user_id {
                transactions.push(tx);
            }
        }
        transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(transactions)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WEBHOOK OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a webhook (insert or update)
    pub async fn save_webhook(&self, webhook: &Webhook) -> Result<()> {
        let id = webhook.id.to_string();
        let data = self.serialize(webhook)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(WEBHOOKS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved webhook: {}", webhook.id);
        Ok(())
    }

    /// Get a webhook by ID
    pub async fn get_webhook(&self, id: &str) -> Result<Option<Webhook>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WEBHOOKS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all webhooks
    pub async fn list_webhooks(&self) -> Result<Vec<Webhook>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WEBHOOKS)?;

        let mut webhooks = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            webhooks.push(self.deserialize(value.value())?);
        }
        Ok(webhooks)
    }

    /// Delete a webhook by ID
    pub async fn delete_webhook(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(WEBHOOKS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted webhook: {}", id);
        }
        Ok(existed)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PUSH SUBSCRIPTION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a push subscription (upsert by id)
    pub async fn save_push_subscription(&self, sub: &PushSubscription) -> Result<()> {
        let id = sub.id.to_string();
        let data = self.serialize(sub)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(PUSH_SUBSCRIPTIONS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!(
            "Saved push subscription {} for user {}",
            sub.id, sub.user_id
        );
        Ok(())
    }

    /// Get all push subscriptions for a given user
    pub async fn get_push_subscriptions_by_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<PushSubscription>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PUSH_SUBSCRIPTIONS)?;

        let mut subs = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let sub: PushSubscription = self.deserialize(value.value())?;
            if sub.user_id == *user_id {
                subs.push(sub);
            }
        }
        Ok(subs)
    }

    /// Delete all push subscriptions for a given user
    pub async fn delete_push_subscriptions_by_user(&self, user_id: &Uuid) -> Result<u64> {
        // First, collect IDs to delete
        let ids: Vec<String> = self
            .get_push_subscriptions_by_user(user_id)
            .await?
            .iter()
            .map(|s| s.id.to_string())
            .collect();

        let count = ids.len() as u64;
        if count == 0 {
            return Ok(0);
        }

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(PUSH_SUBSCRIPTIONS)?;
            for id in &ids {
                table.remove(id.as_str())?;
            }
        }
        write_txn.commit()?;
        debug!(
            "Deleted {} push subscription(s) for user {}",
            count, user_id
        );
        Ok(count)
    }

    /// List all push subscriptions (admin use / delivery fan-out)
    pub async fn list_all_push_subscriptions(&self) -> Result<Vec<PushSubscription>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PUSH_SUBSCRIPTIONS)?;

        let mut subs = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            subs.push(self.deserialize(value.value())?);
        }
        Ok(subs)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ZONE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // FAVORITE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // AUDIT LOG OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // TRANSLATION MANAGEMENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a translation proposal
    pub async fn save_translation_proposal(&self, proposal: &TranslationProposal) -> Result<()> {
        let id = proposal.id.to_string();
        let data = self.serialize(proposal)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(TRANSLATION_PROPOSALS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved translation proposal: {}", proposal.id);
        Ok(())
    }

    /// List translation proposals, optionally filtered by status
    pub async fn list_translation_proposals(
        &self,
        status_filter: Option<&ProposalStatus>,
    ) -> Result<Vec<TranslationProposal>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_PROPOSALS)?;

        let mut proposals = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let p: TranslationProposal = self.deserialize(value.value())?;
            if let Some(filter) = status_filter {
                if &p.status == filter {
                    proposals.push(p);
                }
            } else {
                proposals.push(p);
            }
        }
        proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(proposals)
    }

    /// Get a single translation proposal by ID
    pub async fn get_translation_proposal(&self, id: &str) -> Result<Option<TranslationProposal>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_PROPOSALS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Delete a translation proposal
    pub async fn delete_translation_proposal(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(TRANSLATION_PROPOSALS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        Ok(existed)
    }

    /// Save a translation vote
    pub async fn save_translation_vote(&self, vote: &TranslationVote) -> Result<()> {
        let id = vote.id.to_string();
        let data = self.serialize(vote)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(TRANSLATION_VOTES)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// List votes for a specific proposal
    pub async fn list_votes_for_proposal(&self, proposal_id: Uuid) -> Result<Vec<TranslationVote>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_VOTES)?;

        let mut votes = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let v: TranslationVote = self.deserialize(value.value())?;
            if v.proposal_id == proposal_id {
                votes.push(v);
            }
        }
        Ok(votes)
    }

    /// Get a user's vote on a specific proposal
    pub async fn get_user_vote(
        &self,
        proposal_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TranslationVote>> {
        let votes = self.list_votes_for_proposal(proposal_id).await?;
        Ok(votes.into_iter().find(|v| v.user_id == user_id))
    }

    /// Delete a vote by ID
    pub async fn delete_translation_vote(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(TRANSLATION_VOTES)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        Ok(existed)
    }

    /// Save a translation override (approved translation)
    pub async fn save_translation_override(&self, ovr: &TranslationOverride) -> Result<()> {
        // Key format: "language:key" for uniqueness
        let composite_key = format!("{}:{}", ovr.language, ovr.key);
        let data = self.serialize(ovr)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(TRANSLATION_OVERRIDES)?;
            table.insert(composite_key.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved translation override: {}:{}", ovr.language, ovr.key);
        Ok(())
    }

    /// List all translation overrides
    pub async fn list_translation_overrides(&self) -> Result<Vec<TranslationOverride>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_OVERRIDES)?;

        let mut overrides = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            overrides.push(self.deserialize(value.value())?);
        }
        Ok(overrides)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use parkhub_common::models::{SlotFeature, SlotPosition, SlotStatus, SlotType};
    use tempfile::tempdir;

    fn test_config(path: PathBuf, encrypted: bool) -> DatabaseConfig {
        DatabaseConfig {
            path,
            encryption_enabled: encrypted,
            passphrase: if encrypted {
                Some("test-passphrase".to_string())
            } else {
                None
            },
            create_if_missing: true,
        }
    }

    #[tokio::test]
    async fn test_database_create() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();
        assert!(!db.is_encrypted());
        assert!(db.is_fresh().await.unwrap());
    }

    #[tokio::test]
    async fn test_database_encrypted() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), true);
        let db = Database::open(&config).unwrap();
        assert!(db.is_encrypted());
    }

    #[tokio::test]
    async fn test_setup_completed() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        assert!(db.is_fresh().await.unwrap());
        db.mark_setup_completed().await.unwrap();
        assert!(!db.is_fresh().await.unwrap());
    }

    #[tokio::test]
    async fn test_settings() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        assert!(db.get_setting("test_key").await.unwrap().is_none());
        db.set_setting("test_key", "test_value").await.unwrap();
        assert_eq!(
            db.get_setting("test_key").await.unwrap(),
            Some("test_value".to_string())
        );
    }

    #[tokio::test]
    async fn test_stats() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.users, 0);
        assert_eq!(stats.bookings, 0);
        assert_eq!(stats.parking_lots, 0);
    }

    #[tokio::test]
    async fn test_push_subscriptions_crud() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        let user_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();

        // No subscriptions initially
        let subs = db.get_push_subscriptions_by_user(&user_id).await.unwrap();
        assert!(subs.is_empty());

        // Save two subscriptions for user_id
        let sub1 = PushSubscription {
            id: Uuid::new_v4(),
            user_id,
            endpoint: "https://push.example.com/sub1".into(),
            p256dh: "key1".into(),
            auth: "auth1".into(),
            created_at: Utc::now(),
        };
        let sub2 = PushSubscription {
            id: Uuid::new_v4(),
            user_id,
            endpoint: "https://push.example.com/sub2".into(),
            p256dh: "key2".into(),
            auth: "auth2".into(),
            created_at: Utc::now(),
        };
        // Save one subscription for other_user
        let sub3 = PushSubscription {
            id: Uuid::new_v4(),
            user_id: other_user,
            endpoint: "https://push.example.com/sub3".into(),
            p256dh: "key3".into(),
            auth: "auth3".into(),
            created_at: Utc::now(),
        };

        db.save_push_subscription(&sub1).await.unwrap();
        db.save_push_subscription(&sub2).await.unwrap();
        db.save_push_subscription(&sub3).await.unwrap();

        // List for user_id -> 2
        let subs = db.get_push_subscriptions_by_user(&user_id).await.unwrap();
        assert_eq!(subs.len(), 2);

        // List all -> 3
        let all = db.list_all_push_subscriptions().await.unwrap();
        assert_eq!(all.len(), 3);

        // Delete user_id subscriptions
        let deleted = db
            .delete_push_subscriptions_by_user(&user_id)
            .await
            .unwrap();
        assert_eq!(deleted, 2);

        // user_id has none, other_user still has 1
        assert!(db
            .get_push_subscriptions_by_user(&user_id)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            db.get_push_subscriptions_by_user(&other_user)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WEBHOOK CRUD
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_webhook_crud() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        // Empty initially
        let all = db.list_webhooks().await.unwrap();
        assert!(all.is_empty());

        // Create two webhooks
        let wh1 = Webhook {
            id: Uuid::new_v4(),
            url: "https://example.com/hooks/parking".to_string(),
            secret: "sec_abc123".to_string(),
            events: vec!["booking.created".into(), "booking.cancelled".into()],
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let wh2 = Webhook {
            id: Uuid::new_v4(),
            url: "https://other.io/webhooks".to_string(),
            secret: "sec_xyz789".to_string(),
            events: vec!["slot.status_changed".into()],
            active: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.save_webhook(&wh1).await.unwrap();
        db.save_webhook(&wh2).await.unwrap();

        // List returns both
        let all = db.list_webhooks().await.unwrap();
        assert_eq!(all.len(), 2);

        // Get by ID
        let fetched = db.get_webhook(&wh1.id.to_string()).await.unwrap().unwrap();
        assert_eq!(fetched.url, "https://example.com/hooks/parking");
        assert_eq!(fetched.events.len(), 2);
        assert!(fetched.active);

        // Get non-existent returns None
        let missing = db.get_webhook(&Uuid::new_v4().to_string()).await.unwrap();
        assert!(missing.is_none());

        // Delete first webhook
        let deleted = db.delete_webhook(&wh1.id.to_string()).await.unwrap();
        assert!(deleted);

        // Second delete of same ID returns false
        let deleted_again = db.delete_webhook(&wh1.id.to_string()).await.unwrap();
        assert!(!deleted_again);

        // Only wh2 remains
        let remaining = db.list_webhooks().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, wh2.id);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ZONE CRUD
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_zone_crud() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        let lot_a = Uuid::new_v4();
        let lot_b = Uuid::new_v4();

        // No zones initially
        let zones = db.list_zones_by_lot(&lot_a.to_string()).await.unwrap();
        assert!(zones.is_empty());

        // Create zones in lot_a
        let z1 = Zone {
            id: Uuid::new_v4(),
            lot_id: lot_a,
            name: "Level A".to_string(),
            description: Some("Ground floor, near entrance".to_string()),
            color: Some("#4CAF50".to_string()),
            created_at: Utc::now(),
        };
        let z2 = Zone {
            id: Uuid::new_v4(),
            lot_id: lot_a,
            name: "VIP Section".to_string(),
            description: None,
            color: Some("#FFD700".to_string()),
            created_at: Utc::now(),
        };
        // Zone in a different lot
        let z3 = Zone {
            id: Uuid::new_v4(),
            lot_id: lot_b,
            name: "Basement B1".to_string(),
            description: Some("Underground level".to_string()),
            color: None,
            created_at: Utc::now(),
        };

        db.save_zone(&z1).await.unwrap();
        db.save_zone(&z2).await.unwrap();
        db.save_zone(&z3).await.unwrap();

        // List by lot_a -> 2
        let zones_a = db.list_zones_by_lot(&lot_a.to_string()).await.unwrap();
        assert_eq!(zones_a.len(), 2);
        let names: Vec<&str> = zones_a.iter().map(|z| z.name.as_str()).collect();
        assert!(names.contains(&"Level A"));
        assert!(names.contains(&"VIP Section"));

        // List by lot_b -> 1
        let zones_b = db.list_zones_by_lot(&lot_b.to_string()).await.unwrap();
        assert_eq!(zones_b.len(), 1);
        assert_eq!(zones_b[0].name, "Basement B1");

        // Delete z1 from lot_a
        let deleted = db
            .delete_zone(&lot_a.to_string(), &z1.id.to_string())
            .await
            .unwrap();
        assert!(deleted);

        // Delete non-existent zone returns false
        let no_delete = db
            .delete_zone(&lot_a.to_string(), &Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(!no_delete);

        // Only z2 remains in lot_a
        let zones_a = db.list_zones_by_lot(&lot_a.to_string()).await.unwrap();
        assert_eq!(zones_a.len(), 1);
        assert_eq!(zones_a[0].name, "VIP Section");

        // lot_b untouched
        assert_eq!(
            db.list_zones_by_lot(&lot_b.to_string())
                .await
                .unwrap()
                .len(),
            1
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AUDIT LOG
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_audit_log() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        // Empty initially
        let entries = db.list_audit_log(10).await.unwrap();
        assert!(entries.is_empty());

        let user_id = Uuid::new_v4();

        // Insert entries with staggered timestamps so ordering is deterministic
        let base = Utc::now();
        let e1 = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: base - chrono::Duration::seconds(30),
            event_type: "user.login".to_string(),
            user_id: Some(user_id),
            username: Some("alice".to_string()),
            details: Some("Login from 192.168.1.10".to_string()),
        };
        let e2 = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: base - chrono::Duration::seconds(20),
            event_type: "booking.created".to_string(),
            user_id: Some(user_id),
            username: Some("alice".to_string()),
            details: Some("Booked slot A-12 for 2h".to_string()),
        };
        let e3 = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: base - chrono::Duration::seconds(10),
            event_type: "admin.reset".to_string(),
            user_id: None,
            username: None,
            details: Some("Demo data reset triggered".to_string()),
        };
        let e4 = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: base,
            event_type: "user.logout".to_string(),
            user_id: Some(user_id),
            username: Some("alice".to_string()),
            details: None,
        };

        db.save_audit_log(&e1).await.unwrap();
        db.save_audit_log(&e2).await.unwrap();
        db.save_audit_log(&e3).await.unwrap();
        db.save_audit_log(&e4).await.unwrap();

        // List all 4
        let all = db.list_audit_log(100).await.unwrap();
        assert_eq!(all.len(), 4);

        // Most recent first
        assert_eq!(all[0].event_type, "user.logout");
        assert_eq!(all[1].event_type, "admin.reset");
        assert_eq!(all[2].event_type, "booking.created");
        assert_eq!(all[3].event_type, "user.login");

        // Limit truncates
        let limited = db.list_audit_log(2).await.unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].event_type, "user.logout");
        assert_eq!(limited[1].event_type, "admin.reset");

        // Verify entry fields
        let login = &all[3];
        assert_eq!(login.user_id, Some(user_id));
        assert_eq!(login.username.as_deref(), Some("alice"));
        assert_eq!(login.details.as_deref(), Some("Login from 192.168.1.10"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLEAR ALL DATA — new tables included
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_clear_all_data_includes_new_tables() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        // Populate webhooks
        let wh = Webhook {
            id: Uuid::new_v4(),
            url: "https://hooks.test/a".to_string(),
            secret: "s".to_string(),
            events: vec!["booking.created".into()],
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.save_webhook(&wh).await.unwrap();

        // Populate zones
        let lot_id = Uuid::new_v4();
        let zone = Zone {
            id: Uuid::new_v4(),
            lot_id,
            name: "Zone-1".to_string(),
            description: None,
            color: None,
            created_at: Utc::now(),
        };
        db.save_zone(&zone).await.unwrap();

        // Populate favorites
        let fav = Favorite {
            user_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            lot_id,
            created_at: Utc::now(),
        };
        db.save_favorite(&fav).await.unwrap();

        // Populate audit log
        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: "test.event".to_string(),
            user_id: None,
            username: None,
            details: None,
        };
        db.save_audit_log(&entry).await.unwrap();

        // Verify data exists
        assert_eq!(db.list_webhooks().await.unwrap().len(), 1);
        assert_eq!(
            db.list_zones_by_lot(&lot_id.to_string())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.list_favorites_by_user(&fav.user_id.to_string())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(db.list_audit_log(10).await.unwrap().len(), 1);

        // Clear everything
        db.clear_all_data().await.unwrap();

        // All new tables must be empty
        assert!(db.list_webhooks().await.unwrap().is_empty());
        assert!(db
            .list_zones_by_lot(&lot_id.to_string())
            .await
            .unwrap()
            .is_empty());
        assert!(db
            .list_favorites_by_user(&fav.user_id.to_string())
            .await
            .unwrap()
            .is_empty());
        assert!(db.list_audit_log(10).await.unwrap().is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DELETE PARKING SLOT
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_delete_parking_slot() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(&config).unwrap();

        let lot_id = Uuid::new_v4();
        let floor_id = Uuid::new_v4();

        let slot1 = ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: 1,
            row: 0,
            column: 0,
            slot_type: SlotType::Standard,
            status: SlotStatus::Available,
            current_booking: None,
            features: vec![SlotFeature::NearExit, SlotFeature::WellLit],
            position: SlotPosition {
                x: 10.0,
                y: 20.0,
                width: 3.0,
                height: 5.0,
                rotation: 0.0,
            },
        };
        let slot2 = ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: 2,
            row: 0,
            column: 1,
            slot_type: SlotType::Electric,
            status: SlotStatus::Available,
            current_booking: None,
            features: vec![SlotFeature::ChargingStation],
            position: SlotPosition {
                x: 14.0,
                y: 20.0,
                width: 3.0,
                height: 5.0,
                rotation: 0.0,
            },
        };

        db.save_parking_slot(&slot1).await.unwrap();
        db.save_parking_slot(&slot2).await.unwrap();

        // Both slots exist
        assert!(db
            .get_parking_slot(&slot1.id.to_string())
            .await
            .unwrap()
            .is_some());
        assert!(db
            .get_parking_slot(&slot2.id.to_string())
            .await
            .unwrap()
            .is_some());
        let by_lot = db.list_slots_by_lot(&lot_id.to_string()).await.unwrap();
        assert_eq!(by_lot.len(), 2);

        // Delete slot1
        let removed = db.delete_parking_slot(&slot1.id.to_string()).await.unwrap();
        assert!(removed);

        // slot1 gone from primary table
        assert!(db
            .get_parking_slot(&slot1.id.to_string())
            .await
            .unwrap()
            .is_none());

        // slot1 gone from index (lot query returns only slot2)
        let by_lot = db.list_slots_by_lot(&lot_id.to_string()).await.unwrap();
        assert_eq!(by_lot.len(), 1);
        assert_eq!(by_lot[0].id, slot2.id);

        // slot2 unaffected
        let s2 = db
            .get_parking_slot(&slot2.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(s2.slot_number, 2);

        // Deleting non-existent slot returns false
        let no_op = db
            .delete_parking_slot(&Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(!no_op);

        // Double-delete returns false
        let again = db.delete_parking_slot(&slot1.id.to_string()).await.unwrap();
        assert!(!again);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // USER CRUD
    // ═══════════════════════════════════════════════════════════════════════════

    fn make_user(username: &str, email: &str) -> User {
        let now = Utc::now();
        User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: email.to_string(),
            password_hash: "$argon2id$v=19$m=65536,t=3,p=4$fake".to_string(),
            name: format!("{} User", username),
            picture: None,
            phone: None,
            role: parkhub_common::models::UserRole::User,
            created_at: now,
            updated_at: now,
            last_login: None,
            preferences: parkhub_common::models::UserPreferences::default(),
            is_active: true,
            credits_balance: 0,
            credits_monthly_quota: 40,
            credits_last_refilled: None,
        }
    }

    fn make_vehicle(user_id: Uuid, plate: &str) -> Vehicle {
        Vehicle {
            id: Uuid::new_v4(),
            user_id,
            license_plate: plate.to_string(),
            make: Some("Tesla".to_string()),
            model: Some("Model 3".to_string()),
            color: Some("White".to_string()),
            vehicle_type: parkhub_common::models::VehicleType::Electric,
            is_default: true,
            created_at: Utc::now(),
        }
    }

    fn make_booking(user_id: Uuid, lot_id: Uuid, vehicle: &Vehicle) -> Booking {
        let now = Utc::now();
        Booking {
            id: Uuid::new_v4(),
            user_id,
            lot_id,
            slot_id: Uuid::new_v4(),
            slot_number: 1,
            floor_name: "Ground".to_string(),
            vehicle: vehicle.clone(),
            start_time: now,
            end_time: now + chrono::Duration::hours(2),
            status: parkhub_common::models::BookingStatus::Confirmed,
            pricing: parkhub_common::models::BookingPricing {
                base_price: 5.0,
                discount: 0.0,
                tax: 0.95,
                total: 5.95,
                currency: "EUR".to_string(),
                payment_status: parkhub_common::models::PaymentStatus::Paid,
                payment_method: Some("card".to_string()),
            },
            created_at: now,
            updated_at: now,
            check_in_time: None,
            check_out_time: None,
            qr_code: None,
            notes: None,
        }
    }

    fn make_slot(lot_id: Uuid, floor_id: Uuid, number: i32) -> ParkingSlot {
        ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: number,
            row: 0,
            column: number,
            slot_type: SlotType::Standard,
            status: SlotStatus::Available,
            current_booking: None,
            features: vec![],
            position: SlotPosition {
                x: number as f32 * 4.0,
                y: 0.0,
                width: 3.0,
                height: 5.0,
                rotation: 0.0,
            },
        }
    }

    fn make_parking_lot() -> ParkingLot {
        let now = Utc::now();
        ParkingLot {
            id: Uuid::new_v4(),
            name: "Test Lot".to_string(),
            address: "123 Test St".to_string(),
            latitude: 48.1351,
            longitude: 11.582,
            total_slots: 50,
            available_slots: 50,
            floors: vec![],
            amenities: vec!["EV Charging".to_string()],
            pricing: parkhub_common::models::PricingInfo {
                currency: "EUR".to_string(),
                rates: vec![],
                daily_max: Some(20.0),
                monthly_pass: Some(150.0),
            },
            operating_hours: parkhub_common::models::OperatingHours {
                is_24h: true,
                monday: None,
                tuesday: None,
                wednesday: None,
                thursday: None,
                friday: None,
                saturday: None,
                sunday: None,
            },
            images: vec![],
            status: parkhub_common::models::LotStatus::Open,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_user_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let mut user = make_user("alice", "alice@example.com");

        // Create
        db.save_user(&user).await.unwrap();

        // Get by ID
        let fetched = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
        assert_eq!(fetched.username, "alice");
        assert_eq!(fetched.email, "alice@example.com");

        // Get by username
        let by_name = db.get_user_by_username("alice").await.unwrap().unwrap();
        assert_eq!(by_name.id, user.id);

        // Get by email
        let by_email = db
            .get_user_by_email("alice@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_email.id, user.id);

        // Update
        user.name = "Alice Updated".to_string();
        user.updated_at = Utc::now();
        db.save_user(&user).await.unwrap();
        let updated = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
        assert_eq!(updated.name, "Alice Updated");

        // Delete
        let deleted = db.delete_user(&user.id.to_string()).await.unwrap();
        assert!(deleted);
        assert!(db.get_user(&user.id.to_string()).await.unwrap().is_none());
        assert!(db.get_user_by_username("alice").await.unwrap().is_none());
        assert!(db
            .get_user_by_email("alice@example.com")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_user_list() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let u1 = make_user("alice", "alice@test.com");
        let u2 = make_user("bob", "bob@test.com");
        let u3 = make_user("charlie", "charlie@test.com");

        db.save_user(&u1).await.unwrap();
        db.save_user(&u2).await.unwrap();
        db.save_user(&u3).await.unwrap();

        let all = db.list_users().await.unwrap();
        assert_eq!(all.len(), 3);
        let names: Vec<&str> = all.iter().map(|u| u.username.as_str()).collect();
        assert!(names.contains(&"alice"));
        assert!(names.contains(&"bob"));
        assert!(names.contains(&"charlie"));
    }

    #[tokio::test]
    async fn test_user_not_found() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let fake_id = Uuid::new_v4().to_string();
        assert!(db.get_user(&fake_id).await.unwrap().is_none());
        assert!(db
            .get_user_by_username("nonexistent")
            .await
            .unwrap()
            .is_none());
        assert!(db
            .get_user_by_email("nobody@nowhere.com")
            .await
            .unwrap()
            .is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BOOKING OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_booking_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user = make_user("parker", "parker@test.com");
        let vehicle = make_vehicle(user.id, "M-PH 1234");
        let lot_id = Uuid::new_v4();
        let booking = make_booking(user.id, lot_id, &vehicle);

        // Create
        db.save_booking(&booking).await.unwrap();

        // Get
        let fetched = db
            .get_booking(&booking.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.user_id, user.id);
        assert_eq!(fetched.lot_id, lot_id);
        assert_eq!(fetched.vehicle.license_plate, "M-PH 1234");

        // List by user
        let by_user = db
            .list_bookings_by_user(&user.id.to_string())
            .await
            .unwrap();
        assert_eq!(by_user.len(), 1);
        assert_eq!(by_user[0].id, booking.id);

        // List all
        let all = db.list_bookings().await.unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        let deleted = db.delete_booking(&booking.id.to_string()).await.unwrap();
        assert!(deleted);
        assert!(db
            .get_booking(&booking.id.to_string())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_booking_by_lot() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user = make_user("parker", "parker@test.com");
        let vehicle = make_vehicle(user.id, "M-PH 5678");
        let lot_a = Uuid::new_v4();
        let lot_b = Uuid::new_v4();

        let b1 = make_booking(user.id, lot_a, &vehicle);
        let b2 = make_booking(user.id, lot_a, &vehicle);
        let b3 = make_booking(user.id, lot_b, &vehicle);

        db.save_booking(&b1).await.unwrap();
        db.save_booking(&b2).await.unwrap();
        db.save_booking(&b3).await.unwrap();

        let all = db.list_bookings().await.unwrap();
        assert_eq!(all.len(), 3);

        let lot_a_bookings: Vec<_> = all.iter().filter(|b| b.lot_id == lot_a).collect();
        let lot_b_bookings: Vec<_> = all.iter().filter(|b| b.lot_id == lot_b).collect();
        assert_eq!(lot_a_bookings.len(), 2);
        assert_eq!(lot_b_bookings.len(), 1);
        assert_eq!(lot_b_bookings[0].id, b3.id);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VEHICLE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_vehicle_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let vehicle = make_vehicle(user_id, "B-AB 9876");

        // Create
        db.save_vehicle(&vehicle).await.unwrap();

        // Get
        let fetched = db
            .get_vehicle(&vehicle.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.license_plate, "B-AB 9876");
        assert_eq!(fetched.user_id, user_id);

        // List by user
        let by_user = db
            .list_vehicles_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert_eq!(by_user.len(), 1);

        // Delete
        let deleted = db.delete_vehicle(&vehicle.id.to_string()).await.unwrap();
        assert!(deleted);
        assert!(db
            .get_vehicle(&vehicle.id.to_string())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_vehicle_delete_nonexistent() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let result = db
            .delete_vehicle(&Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(!result);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_session_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let session = Session::new(user_id, 24, "testuser", "user");
        let token = "access_tok_abc123";

        // Save
        db.save_session(token, &session).await.unwrap();

        // Get
        let fetched = db.get_session(token).await.unwrap().unwrap();
        assert_eq!(fetched.user_id, user_id);
        assert_eq!(fetched.username, "testuser");
        assert_eq!(fetched.role, "user");
        assert!(!fetched.is_expired());

        // Delete
        let deleted = db.delete_session(token).await.unwrap();
        assert!(deleted);
        assert!(db.get_session(token).await.unwrap().is_none());

        // Delete again returns false
        let again = db.delete_session(token).await.unwrap();
        assert!(!again);
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        // Create a session that expired 1 hour ago
        let mut session = Session::new(user_id, 1, "expired_user", "user");
        session.expires_at = Utc::now() - chrono::Duration::hours(1);
        let token = "expired_token_xyz";

        db.save_session(token, &session).await.unwrap();

        // is_expired should be true on the raw struct
        assert!(session.is_expired());

        // get_session filters out expired sessions -> returns None
        assert!(db.get_session(token).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_sessions_by_user() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();

        let s1 = Session::new(user_a, 24, "alice", "user");
        let s2 = Session::new(user_a, 24, "alice", "user");
        let s3 = Session::new(user_a, 24, "alice", "user");
        let s4 = Session::new(user_b, 24, "bob", "admin");

        db.save_session("tok_a1", &s1).await.unwrap();
        db.save_session("tok_a2", &s2).await.unwrap();
        db.save_session("tok_a3", &s3).await.unwrap();
        db.save_session("tok_b1", &s4).await.unwrap();

        // Delete all sessions for user_a
        let deleted = db.delete_sessions_by_user(user_a).await.unwrap();
        assert_eq!(deleted, 3);

        // user_a sessions gone
        assert!(db.get_session("tok_a1").await.unwrap().is_none());
        assert!(db.get_session("tok_a2").await.unwrap().is_none());
        assert!(db.get_session("tok_a3").await.unwrap().is_none());

        // user_b session untouched
        let bob = db.get_session("tok_b1").await.unwrap().unwrap();
        assert_eq!(bob.user_id, user_b);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SETTINGS OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_settings_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        // Non-existent key returns None
        assert!(db.get_setting("theme").await.unwrap().is_none());

        // Set
        db.set_setting("theme", "dark").await.unwrap();
        assert_eq!(
            db.get_setting("theme").await.unwrap(),
            Some("dark".to_string())
        );

        // Overwrite
        db.set_setting("theme", "light").await.unwrap();
        assert_eq!(
            db.get_setting("theme").await.unwrap(),
            Some("light".to_string())
        );

        // Another key is independent
        assert!(db.get_setting("locale").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_setup_workflow() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        // Fresh DB
        assert!(db.is_fresh().await.unwrap());

        // Mark setup completed
        db.mark_setup_completed().await.unwrap();
        assert!(!db.is_fresh().await.unwrap());

        // Idempotent — marking again doesn't fail
        db.mark_setup_completed().await.unwrap();
        assert!(!db.is_fresh().await.unwrap());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NOTIFICATION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_notification_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let notif = Notification {
            id: Uuid::new_v4(),
            user_id,
            notification_type: parkhub_common::models::NotificationType::BookingConfirmed,
            title: "Booking Confirmed".to_string(),
            message: "Your slot A-12 is booked for 14:00-16:00".to_string(),
            data: None,
            read: false,
            created_at: Utc::now(),
        };

        // Save
        db.save_notification(&notif).await.unwrap();

        // List by user
        let list = db
            .list_notifications_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Booking Confirmed");
        assert!(!list[0].read);

        // Mark read
        let marked = db
            .mark_notification_read(&notif.id.to_string())
            .await
            .unwrap();
        assert!(marked);

        // Verify read=true
        let list = db
            .list_notifications_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert!(list[0].read);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ANNOUNCEMENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_announcement_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let ann = Announcement {
            id: Uuid::new_v4(),
            title: "Maintenance Notice".to_string(),
            message: "Level B2 closed for repairs on Saturday".to_string(),
            severity: parkhub_common::models::AnnouncementSeverity::Warning,
            active: true,
            created_by: Some(Uuid::new_v4()),
            expires_at: Some(Utc::now() + chrono::Duration::days(7)),
            created_at: Utc::now(),
        };

        // Save
        db.save_announcement(&ann).await.unwrap();

        // List
        let all = db.list_announcements().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "Maintenance Notice");
        assert_eq!(
            all[0].severity,
            parkhub_common::models::AnnouncementSeverity::Warning
        );

        // Delete
        let deleted = db.delete_announcement(&ann.id.to_string()).await.unwrap();
        assert!(deleted);
        assert!(db.list_announcements().await.unwrap().is_empty());

        // Delete non-existent
        let nope = db
            .delete_announcement(&Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(!nope);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ABSENCE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_absence_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let absence = Absence {
            id: Uuid::new_v4(),
            user_id,
            absence_type: parkhub_common::models::AbsenceType::Homeoffice,
            start_date: "2026-03-20".to_string(),
            end_date: "2026-03-20".to_string(),
            note: Some("Working from home".to_string()),
            source: "manual".to_string(),
            created_at: Utc::now(),
        };

        // Create
        db.save_absence(&absence).await.unwrap();

        // List by user
        let list = db
            .list_absences_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].start_date, "2026-03-20");

        // Delete
        let deleted = db.delete_absence(&absence.id.to_string()).await.unwrap();
        assert!(deleted);
        assert!(db
            .list_absences_by_user(&user_id.to_string())
            .await
            .unwrap()
            .is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CREDIT TRANSACTION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_credit_transaction_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let tx1 = parkhub_common::models::CreditTransaction {
            id: Uuid::new_v4(),
            user_id,
            booking_id: Some(Uuid::new_v4()),
            amount: -2,
            transaction_type: parkhub_common::models::CreditTransactionType::Deduction,
            description: Some("Booking slot A-5".to_string()),
            granted_by: None,
            created_at: Utc::now() - chrono::Duration::minutes(10),
        };
        let tx2 = parkhub_common::models::CreditTransaction {
            id: Uuid::new_v4(),
            user_id,
            booking_id: None,
            amount: 40,
            transaction_type: parkhub_common::models::CreditTransactionType::MonthlyRefill,
            description: Some("Monthly refill".to_string()),
            granted_by: None,
            created_at: Utc::now(),
        };

        db.save_credit_transaction(&tx1).await.unwrap();
        db.save_credit_transaction(&tx2).await.unwrap();

        let list = db.list_credit_transactions_for_user(user_id).await.unwrap();
        assert_eq!(list.len(), 2);
        // Sorted newest first
        assert_eq!(
            list[0].transaction_type,
            parkhub_common::models::CreditTransactionType::MonthlyRefill
        );
        assert_eq!(
            list[1].transaction_type,
            parkhub_common::models::CreditTransactionType::Deduction
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SLOT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_slot_batch_save() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let lot_id = Uuid::new_v4();
        let floor_id = Uuid::new_v4();

        let slots: Vec<ParkingSlot> = (1..=5).map(|n| make_slot(lot_id, floor_id, n)).collect();

        db.save_parking_slots_batch(&slots).await.unwrap();

        let by_lot = db.list_slots_by_lot(&lot_id.to_string()).await.unwrap();
        assert_eq!(by_lot.len(), 5);

        // Each slot accessible by ID
        for slot in &slots {
            let fetched = db
                .get_parking_slot(&slot.id.to_string())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(fetched.lot_id, lot_id);
        }
    }

    #[tokio::test]
    async fn test_slot_status_update() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let lot_id = Uuid::new_v4();
        let floor_id = Uuid::new_v4();
        let slot = make_slot(lot_id, floor_id, 1);

        db.save_parking_slot(&slot).await.unwrap();
        let fetched = db
            .get_parking_slot(&slot.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.status, SlotStatus::Available);

        // Update status
        let updated = db
            .update_slot_status(&slot.id.to_string(), SlotStatus::Occupied)
            .await
            .unwrap();
        assert!(updated);

        let after = db
            .get_parking_slot(&slot.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.status, SlotStatus::Occupied);

        // Update non-existent slot returns false
        let nope = db
            .update_slot_status(&Uuid::new_v4().to_string(), SlotStatus::Maintenance)
            .await
            .unwrap();
        assert!(!nope);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STATS
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_stats_after_data() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        // Users
        let u1 = make_user("alice", "alice@stats.com");
        let u2 = make_user("bob", "bob@stats.com");
        db.save_user(&u1).await.unwrap();
        db.save_user(&u2).await.unwrap();

        // Parking lot
        let lot = make_parking_lot();
        db.save_parking_lot(&lot).await.unwrap();

        // Slots
        let floor_id = Uuid::new_v4();
        let s1 = make_slot(lot.id, floor_id, 1);
        let s2 = make_slot(lot.id, floor_id, 2);
        let s3 = make_slot(lot.id, floor_id, 3);
        db.save_parking_slots_batch(&[s1, s2, s3]).await.unwrap();

        // Vehicle + Bookings
        let v = make_vehicle(u1.id, "M-ST 1111");
        db.save_vehicle(&v).await.unwrap();
        let b1 = make_booking(u1.id, lot.id, &v);
        let b2 = make_booking(u1.id, lot.id, &v);
        db.save_booking(&b1).await.unwrap();
        db.save_booking(&b2).await.unwrap();

        // Session
        let session = Session::new(u1.id, 24, "alice", "user");
        db.save_session("stats_tok", &session).await.unwrap();

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.users, 2);
        assert_eq!(stats.parking_lots, 1);
        assert_eq!(stats.slots, 3);
        assert_eq!(stats.bookings, 2);
        assert_eq!(stats.vehicles, 1);
        assert_eq!(stats.sessions, 1);
    }

    #[tokio::test]
    async fn test_database_stats_empty() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.users, 0);
        assert_eq!(stats.bookings, 0);
        assert_eq!(stats.parking_lots, 0);
        assert_eq!(stats.slots, 0);
        assert_eq!(stats.sessions, 0);
        assert_eq!(stats.vehicles, 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // FAVORITES CRUD
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_favorites_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let slot_a = Uuid::new_v4();
        let slot_b = Uuid::new_v4();
        let lot_id = Uuid::new_v4();

        // No favorites initially
        let favs = db
            .list_favorites_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert!(favs.is_empty());

        // Add two favorites
        let fav1 = Favorite {
            user_id,
            slot_id: slot_a,
            lot_id,
            created_at: Utc::now(),
        };
        let fav2 = Favorite {
            user_id,
            slot_id: slot_b,
            lot_id,
            created_at: Utc::now(),
        };

        db.save_favorite(&fav1).await.unwrap();
        db.save_favorite(&fav2).await.unwrap();

        // List returns both
        let favs = db
            .list_favorites_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert_eq!(favs.len(), 2);

        // Delete one
        let deleted = db
            .delete_favorite(&user_id.to_string(), &slot_a.to_string())
            .await
            .unwrap();
        assert!(deleted);

        // Only one remains
        let favs = db
            .list_favorites_by_user(&user_id.to_string())
            .await
            .unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].slot_id, slot_b);

        // Delete non-existent returns false
        let nope = db
            .delete_favorite(&user_id.to_string(), &Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(!nope);
    }

    #[tokio::test]
    async fn test_favorites_isolation_between_users() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let lot_id = Uuid::new_v4();

        // Both users favorite the same slot
        let fav_a = Favorite {
            user_id: user_a,
            slot_id,
            lot_id,
            created_at: Utc::now(),
        };
        let fav_b = Favorite {
            user_id: user_b,
            slot_id,
            lot_id,
            created_at: Utc::now(),
        };

        db.save_favorite(&fav_a).await.unwrap();
        db.save_favorite(&fav_b).await.unwrap();

        // Each user sees only their own
        let a_favs = db
            .list_favorites_by_user(&user_a.to_string())
            .await
            .unwrap();
        assert_eq!(a_favs.len(), 1);
        assert_eq!(a_favs[0].user_id, user_a);

        let b_favs = db
            .list_favorites_by_user(&user_b.to_string())
            .await
            .unwrap();
        assert_eq!(b_favs.len(), 1);
        assert_eq!(b_favs[0].user_id, user_b);

        // Delete user_a's favorite doesn't affect user_b
        db.delete_favorite(&user_a.to_string(), &slot_id.to_string())
            .await
            .unwrap();
        assert!(db
            .list_favorites_by_user(&user_a.to_string())
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            db.list_favorites_by_user(&user_b.to_string())
                .await
                .unwrap()
                .len(),
            1
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GDPR ANONYMIZATION
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_gdpr_anonymize_user() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user = make_user("alice", "alice@example.com");
        db.save_user(&user).await.unwrap();

        // Add a vehicle
        let vehicle = make_vehicle(user.id, "M-AB 1234");
        db.save_vehicle(&vehicle).await.unwrap();

        // Add a booking
        let lot_id = Uuid::new_v4();
        let booking = make_booking(user.id, lot_id, &vehicle);
        db.save_booking(&booking).await.unwrap();

        // Anonymize
        let result = db.anonymize_user(&user.id.to_string()).await.unwrap();
        assert!(result);

        // User still exists but is anonymized
        let anon = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
        assert_eq!(anon.name, "[Deleted User]");
        assert!(anon.username.starts_with("deleted-"));
        assert!(anon.email.ends_with("@deleted.invalid"));

        // Old username/email lookups return None
        assert!(db.get_user_by_username("alice").await.unwrap().is_none());
        assert!(db
            .get_user_by_email("alice@example.com")
            .await
            .unwrap()
            .is_none());

        // New anonymized username lookup works
        let by_name = db.get_user_by_username(&anon.username).await.unwrap();
        assert!(by_name.is_some());

        // Vehicle is deleted
        assert!(db
            .get_vehicle(&vehicle.id.to_string())
            .await
            .unwrap()
            .is_none());

        // Booking still exists but license plate is scrubbed
        let scrubbed = db
            .get_booking(&booking.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(scrubbed.vehicle.license_plate, "[DELETED]");
    }

    #[tokio::test]
    async fn test_gdpr_anonymize_nonexistent_user() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let result = db
            .anonymize_user(&Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(!result);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION REFRESH TOKEN LOOKUP
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_get_session_by_refresh_token() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let session = Session::new(user_id, 24, "alice", "user");
        let refresh_token = session.refresh_token.clone();
        let access_token = "access_abc";

        db.save_session(access_token, &session).await.unwrap();

        // Lookup by refresh token
        let (found_access, found_session) = db
            .get_session_by_refresh_token(&refresh_token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_access, access_token);
        assert_eq!(found_session.user_id, user_id);
        assert_eq!(found_session.username, "alice");

        // Non-existent refresh token
        let missing = db
            .get_session_by_refresh_token("nonexistent")
            .await
            .unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_get_session_by_refresh_token_expired() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let user_id = Uuid::new_v4();
        let mut session = Session::new(user_id, 1, "expired", "user");
        let refresh_token = session.refresh_token.clone();
        session.expires_at = Utc::now() - chrono::Duration::hours(1);

        db.save_session("expired_tok", &session).await.unwrap();

        // Expired session should return None
        let result = db
            .get_session_by_refresh_token(&refresh_token)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLEAR ALL DATA — verifies settings preserved
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_clear_all_data_preserves_settings() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        // Add some data
        let user = make_user("alice", "alice@test.com");
        db.save_user(&user).await.unwrap();

        // Set a custom setting
        db.set_setting("custom_key", "custom_value").await.unwrap();
        db.mark_setup_completed().await.unwrap();

        // Clear
        db.clear_all_data().await.unwrap();

        // Users gone
        assert!(db.list_users().await.unwrap().is_empty());

        // Settings preserved
        assert_eq!(
            db.get_setting("custom_key").await.unwrap(),
            Some("custom_value".to_string())
        );
        assert!(!db.is_fresh().await.unwrap());
    }

    #[tokio::test]
    async fn test_clear_all_data_sessions_and_bookings() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        // Sessions
        let session = Session::new(Uuid::new_v4(), 24, "test", "user");
        db.save_session("tok1", &session).await.unwrap();

        // Bookings
        let user = make_user("alice", "alice@test.com");
        let vehicle = make_vehicle(user.id, "X-1");
        let booking = make_booking(user.id, Uuid::new_v4(), &vehicle);
        db.save_user(&user).await.unwrap();
        db.save_booking(&booking).await.unwrap();

        db.clear_all_data().await.unwrap();

        assert!(db.get_session("tok1").await.unwrap().is_none());
        assert!(db.list_bookings().await.unwrap().is_empty());
        assert!(db.list_users().await.unwrap().is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ANNOUNCEMENT — additional coverage
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_announcement_multiple_and_order() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let a1 = Announcement {
            id: Uuid::new_v4(),
            title: "First".to_string(),
            message: "First announcement".to_string(),
            severity: parkhub_common::models::AnnouncementSeverity::Info,
            active: true,
            created_by: None,
            expires_at: None,
            created_at: Utc::now(),
        };
        let a2 = Announcement {
            id: Uuid::new_v4(),
            title: "Second".to_string(),
            message: "Second announcement".to_string(),
            severity: parkhub_common::models::AnnouncementSeverity::Error,
            active: false,
            created_by: Some(Uuid::new_v4()),
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            created_at: Utc::now(),
        };

        db.save_announcement(&a1).await.unwrap();
        db.save_announcement(&a2).await.unwrap();

        let all = db.list_announcements().await.unwrap();
        assert_eq!(all.len(), 2);

        let titles: Vec<&str> = all.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"First"));
        assert!(titles.contains(&"Second"));

        // Verify severity is preserved
        let critical = all.iter().find(|a| a.title == "Second").unwrap();
        assert_eq!(
            critical.severity,
            parkhub_common::models::AnnouncementSeverity::Error
        );
        assert!(!critical.active);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PARKING LOT CRUD
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_parking_lot_crud() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

        let lot = make_parking_lot();

        // Save
        db.save_parking_lot(&lot).await.unwrap();

        // Get
        let fetched = db
            .get_parking_lot(&lot.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.name, "Test Lot");
        assert_eq!(fetched.address, "123 Test St");

        // List
        let all = db.list_parking_lots().await.unwrap();
        assert_eq!(all.len(), 1);

        // Non-existent
        let missing = db
            .get_parking_lot(&Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(missing.is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION — constructor invariants
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_session_new_fields() {
        let user_id = Uuid::new_v4();
        let session = Session::new(user_id, 48, "testuser", "admin");

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.username, "testuser");
        assert_eq!(session.role, "admin");
        assert!(session.refresh_token.starts_with("rt_"));
        // Refresh token has 64 hex chars after prefix
        assert_eq!(session.refresh_token.len(), 3 + 64);
        assert!(!session.is_expired());
        assert!(session.expires_at > session.created_at);
    }

    #[test]
    fn test_session_unique_refresh_tokens() {
        let uid = Uuid::new_v4();
        let s1 = Session::new(uid, 1, "u", "r");
        let s2 = Session::new(uid, 1, "u", "r");
        assert_ne!(
            s1.refresh_token, s2.refresh_token,
            "Each session must have a unique refresh token"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENCRYPTED DATABASE — data roundtrip
    // ═══════════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_encrypted_data_roundtrip() {
        let dir = tempdir().unwrap();
        let db = Database::open(&test_config(dir.path().to_path_buf(), true)).unwrap();
        assert!(db.is_encrypted());

        // Save and retrieve a user through encryption
        let user = make_user("encrypted_alice", "encrypted@test.com");
        db.save_user(&user).await.unwrap();

        let fetched = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
        assert_eq!(fetched.username, "encrypted_alice");
        assert_eq!(fetched.email, "encrypted@test.com");
    }
}
