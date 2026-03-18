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
    RecurringBooking, SwapRequest, User, Vehicle, WaitlistEntry,
};

// ═══════════════════════════════════════════════════════════════════════════════
// TABLE DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════════

const USERS: TableDefinition<&str, &[u8]> = TableDefinition::new("users");
const USERS_BY_USERNAME: TableDefinition<&str, &str> = TableDefinition::new("users_by_username");
const USERS_BY_EMAIL: TableDefinition<&str, &str> = TableDefinition::new("users_by_email");
const SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");
const BOOKINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("bookings");
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
    /// Passphrase for encryption (required if encryption_enabled)
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
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
        Ok(Self { cipher })
    }

    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

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
            .map_err(|e| anyhow!("Decryption failed: {}", e))
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
    pub fn open(config: DatabaseConfig) -> Result<Self> {
        let db_path = config.path.join("parkhub.redb");

        // Check if database exists
        let db_exists = db_path.exists();
        if !db_exists && !config.create_if_missing {
            return Err(anyhow!(
                "Database not found at {:?} and create_if_missing is false",
                db_path
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
                match table.get(SETTING_ENCRYPTION_SALT)? {
                    Some(value) => {
                        hex::decode(value.value()).context("Invalid salt in database")?
                    }
                    None => {
                        // Generate new salt
                        let mut salt = [0u8; 32];
                        rand::rng().fill_bytes(&mut salt);

                        // Store salt
                        let write_txn = db.begin_write()?;
                        {
                            let mut table = write_txn.open_table(SETTINGS)?;
                            table.insert(SETTING_ENCRYPTION_SALT, hex::encode(&salt).as_str())?;
                        }
                        write_txn.commit()?;

                        salt.to_vec()
                    }
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
    pub fn is_encrypted(&self) -> bool {
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
        drain_table!(write_txn, USERS);
        drain_table!(write_txn, USERS_BY_USERNAME);
        drain_table!(write_txn, USERS_BY_EMAIL);
        drain_table!(write_txn, SESSIONS);
        drain_table!(write_txn, BOOKINGS);
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
        // Preserve SETTINGS table (encryption salt, setup status, etc.)
        write_txn.commit()?;
        info!("All data tables cleared for demo reset");
        Ok(())
    }

    /// Check if the database is fresh (no setup completed)
    pub async fn is_fresh(&self) -> Result<bool> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(SETTINGS)?;

        match table.get(SETTING_SETUP_COMPLETED)? {
            Some(value) => Ok(value.value() != "true"),
            None => Ok(true),
        }
    }

    /// Mark the initial setup as completed
    pub async fn mark_setup_completed(&self) -> Result<()> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
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
    /// Returns a tuple of (access_token, session) if found and not expired.
    pub async fn get_session_by_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<Option<(String, Session)>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
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
        let user = match self.get_user(id).await? {
            Some(u) => u,
            None => return Ok(false),
        };

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
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
        {
            // Save main slot data
            let mut table = write_txn.open_table(PARKING_SLOTS)?;
            table.insert(id.as_str(), data.as_slice())?;

            // Update lot->slots index
            let mut idx = write_txn.open_table(SLOTS_BY_LOT)?;
            let key = format!("{}:{}", lot_id, id);
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
        let table = read_txn.open_table(PARKING_SLOTS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get all parking slots for a lot (list_slots_by_lot)
    pub async fn list_slots_by_lot(&self, lot_id: &str) -> Result<Vec<ParkingSlot>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(SLOTS_BY_LOT)?;

        let prefix = format!("{}:", lot_id);
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
    /// Removes entries from both PARKING_SLOTS and SLOTS_BY_LOT index.
    pub async fn delete_slots_by_lot(&self, lot_id: &str) -> Result<()> {
        let prefix = format!("{}:", lot_id);

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
        {
            let mut table = write_txn.open_table(PARKING_SLOTS)?;
            let mut idx = write_txn.open_table(SLOTS_BY_LOT)?;
            for (id, lot_id, data) in &serialized {
                table.insert(id.as_str(), data.as_slice())?;
                let key = format!("{}:{}", lot_id, id);
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
        let mut slot = match self.get_parking_slot(slot_id).await? {
            Some(s) => s,
            None => return Ok(false),
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
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved booking: {}", booking.id);
        Ok(())
    }

    /// Get a booking by ID (string)
    pub async fn get_booking(&self, id: &str) -> Result<Option<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
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
        let table = read_txn.open_table(BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            bookings.push(self.deserialize(value.value())?);
        }
        Ok(bookings)
    }

    /// Get bookings for a user (list_bookings_by_user)
    pub async fn list_bookings_by_user(&self, user_id: &str) -> Result<Vec<Booking>> {
        let all_bookings = self.list_bookings().await?;
        Ok(all_bookings
            .into_iter()
            .filter(|b| b.user_id.to_string() == user_id)
            .collect())
    }

    /// Delete a booking
    pub async fn delete_booking(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        let existed = {
            let mut table = write_txn.open_table(BOOKINGS)?;
            let result = table.remove(id)?;
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
        let table = read_txn.open_table(VEHICLES)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get vehicles for a user (list_vehicles_by_user)
    pub async fn list_vehicles_by_user(&self, user_id: &str) -> Result<Vec<Vehicle>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
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
        let user = match self.get_user(user_id).await? {
            Some(u) => u,
            None => return Ok(false),
        };

        let old_username = user.username.clone();
        let old_email = user.email.clone();
        let anon_id = format!("deleted-{}", Uuid::new_v4());
        let anon_email = format!("{}@deleted.invalid", anon_id);
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
        drop(db);

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

    // ═══════════════════════════════════════════════════════════════════════════
    // ABSENCE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save an absence record
    pub async fn save_absence(&self, absence: &Absence) -> Result<()> {
        let id = absence.id.to_string();
        let data = self.serialize(absence)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
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

    /// Delete a waitlist entry
    pub async fn delete_waitlist_entry(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
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
        let mut notification = match self.get_notification(id).await? {
            Some(n) => n,
            None => return Ok(false),
        };

        notification.read = true;
        self.save_notification(&notification).await?;
        Ok(true)
    }

    /// Get a notification by ID (helper for mark_notification_read)
    async fn get_notification(&self, id: &str) -> Result<Option<Notification>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
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
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
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
        let db = Database::open(config).unwrap();
        assert!(!db.is_encrypted());
        assert!(db.is_fresh().await.unwrap());
    }

    #[tokio::test]
    async fn test_database_encrypted() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), true);
        let db = Database::open(config).unwrap();
        assert!(db.is_encrypted());
    }

    #[tokio::test]
    async fn test_setup_completed() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(config).unwrap();

        assert!(db.is_fresh().await.unwrap());
        db.mark_setup_completed().await.unwrap();
        assert!(!db.is_fresh().await.unwrap());
    }

    #[tokio::test]
    async fn test_settings() {
        let dir = tempdir().unwrap();
        let config = test_config(dir.path().to_path_buf(), false);
        let db = Database::open(config).unwrap();

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
        let db = Database::open(config).unwrap();

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.users, 0);
        assert_eq!(stats.bookings, 0);
        assert_eq!(stats.parking_lots, 0);
    }
}
