//! Local Repository
//!
//! Handles all database operations for the local SQLite database.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

use super::schema::{CREATE_SCHEMA, SCHEMA_VERSION};

/// Error type for database operations
#[derive(Debug)]
pub enum DbError {
    ConnectionError(String),
    QueryError(String),
    SerializationError(String),
    NotFound,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::ConnectionError(msg) => write!(f, "Database connection error: {}", msg),
            DbError::QueryError(msg) => write!(f, "Database query error: {}", msg),
            DbError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            DbError::NotFound => write!(f, "Record not found"),
        }
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError::QueryError(err.to_string())
    }
}

impl From<serde_json::Error> for DbError {
    fn from(err: serde_json::Error) -> Self {
        DbError::SerializationError(err.to_string())
    }
}

pub type DbResult<T> = Result<T, DbError>;

/// Local repository for SQLite database operations
pub struct LocalRepository {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl LocalRepository {
    /// Create a new repository with the given database path
    pub fn new(db_path: PathBuf) -> DbResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DbError::ConnectionError(e.to_string()))?;
        }

        let conn =
            Connection::open(&db_path).map_err(|e| DbError::ConnectionError(e.to_string()))?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        };

        // Initialize schema
        repo.initialize_schema()?;

        Ok(repo)
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> DbResult<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| DbError::ConnectionError(e.to_string()))?;

        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        };

        repo.initialize_schema()?;
        Ok(repo)
    }

    /// Initialize the database schema
    fn initialize_schema(&self) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();

        // Check current schema version
        let version: Option<i32> = conn
            .query_row(
                "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;

        if version.is_none() || version.unwrap() < SCHEMA_VERSION {
            info!("Initializing database schema v{}", SCHEMA_VERSION);
            conn.execute_batch(CREATE_SCHEMA)?;
            conn.execute(
                "INSERT OR REPLACE INTO schema_version (version) VALUES (?)",
                params![SCHEMA_VERSION],
            )?;
        }

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save user session
    pub fn save_session(
        &self,
        user_id: &str,
        email: &str,
        name: &str,
        picture: Option<&str>,
        role: &str,
        access_token: &str,
        refresh_token: &str,
        token_expires_at: &str,
    ) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();

        // Clear existing sessions
        conn.execute("DELETE FROM user_session", [])?;

        conn.execute(
            "INSERT INTO user_session (user_id, email, name, picture, role, access_token, refresh_token, token_expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![user_id, email, name, picture, role, access_token, refresh_token, token_expires_at],
        )?;

        debug!("Session saved for user: {}", email);
        Ok(())
    }

    /// Get current session
    pub fn get_session(&self) -> DbResult<Option<SessionData>> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                "SELECT user_id, email, name, picture, role, access_token, refresh_token, token_expires_at
                 FROM user_session LIMIT 1",
                [],
                |row| {
                    Ok(SessionData {
                        user_id: row.get(0)?,
                        email: row.get(1)?,
                        name: row.get(2)?,
                        picture: row.get(3)?,
                        role: row.get(4)?,
                        access_token: row.get(5)?,
                        refresh_token: row.get(6)?,
                        token_expires_at: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Clear all session data
    pub fn clear_session(&self) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM user_session", [])?;
        info!("Session cleared");
        Ok(())
    }

    /// Update access token
    pub fn update_access_token(&self, access_token: &str, expires_at: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE user_session SET access_token = ?, token_expires_at = ?, updated_at = datetime('now')",
            params![access_token, expires_at],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VEHICLES
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a vehicle
    pub fn save_vehicle(&self, vehicle: &VehicleData) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO vehicles (id, user_id, license_plate, make, model, color, vehicle_type, is_default)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                vehicle.id,
                vehicle.user_id,
                vehicle.license_plate,
                vehicle.make,
                vehicle.model,
                vehicle.color,
                vehicle.vehicle_type,
                vehicle.is_default as i32,
            ],
        )?;
        Ok(())
    }

    /// Get all vehicles for a user
    pub fn get_vehicles(&self, user_id: &str) -> DbResult<Vec<VehicleData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, license_plate, make, model, color, vehicle_type, is_default
             FROM vehicles WHERE user_id = ? ORDER BY is_default DESC, created_at DESC",
        )?;

        let vehicles = stmt
            .query_map(params![user_id], |row| {
                Ok(VehicleData {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    license_plate: row.get(2)?,
                    make: row.get(3)?,
                    model: row.get(4)?,
                    color: row.get(5)?,
                    vehicle_type: row.get(6)?,
                    is_default: row.get::<_, i32>(7)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vehicles)
    }

    /// Delete a vehicle
    pub fn delete_vehicle(&self, vehicle_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM vehicles WHERE id = ?", params![vehicle_id])?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BOOKINGS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a booking
    pub fn save_booking(&self, booking: &BookingData) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO bookings
             (id, user_id, lot_id, slot_id, slot_number, floor_name, vehicle_json, start_time, end_time, status, pricing_json, qr_code, notes, synced)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                booking.id,
                booking.user_id,
                booking.lot_id,
                booking.slot_id,
                booking.slot_number,
                booking.floor_name,
                booking.vehicle_json,
                booking.start_time,
                booking.end_time,
                booking.status,
                booking.pricing_json,
                booking.qr_code,
                booking.notes,
                booking.synced as i32,
            ],
        )?;
        Ok(())
    }

    /// Get all bookings for a user
    pub fn get_bookings(&self, user_id: &str) -> DbResult<Vec<BookingData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, lot_id, slot_id, slot_number, floor_name, vehicle_json,
                    start_time, end_time, status, pricing_json, qr_code, notes, synced
             FROM bookings WHERE user_id = ? ORDER BY start_time DESC",
        )?;

        let bookings = stmt
            .query_map(params![user_id], |row| {
                Ok(BookingData {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    lot_id: row.get(2)?,
                    slot_id: row.get(3)?,
                    slot_number: row.get(4)?,
                    floor_name: row.get(5)?,
                    vehicle_json: row.get(6)?,
                    start_time: row.get(7)?,
                    end_time: row.get(8)?,
                    status: row.get(9)?,
                    pricing_json: row.get(10)?,
                    qr_code: row.get(11)?,
                    notes: row.get(12)?,
                    synced: row.get::<_, i32>(13)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(bookings)
    }

    /// Get active bookings
    pub fn get_active_bookings(&self, user_id: &str) -> DbResult<Vec<BookingData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, lot_id, slot_id, slot_number, floor_name, vehicle_json,
                    start_time, end_time, status, pricing_json, qr_code, notes, synced
             FROM bookings
             WHERE user_id = ? AND status IN ('pending', 'confirmed', 'active')
             ORDER BY start_time ASC",
        )?;

        let bookings = stmt
            .query_map(params![user_id], |row| {
                Ok(BookingData {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    lot_id: row.get(2)?,
                    slot_id: row.get(3)?,
                    slot_number: row.get(4)?,
                    floor_name: row.get(5)?,
                    vehicle_json: row.get(6)?,
                    start_time: row.get(7)?,
                    end_time: row.get(8)?,
                    status: row.get(9)?,
                    pricing_json: row.get(10)?,
                    qr_code: row.get(11)?,
                    notes: row.get(12)?,
                    synced: row.get::<_, i32>(13)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(bookings)
    }

    /// Update booking status
    pub fn update_booking_status(&self, booking_id: &str, status: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE bookings SET status = ?, updated_at = datetime('now') WHERE id = ?",
            params![status, booking_id],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // OFFLINE QUEUE
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add action to offline queue
    pub fn queue_offline_action(
        &self,
        action_type: &str,
        endpoint: &str,
        method: &str,
        payload: Option<&str>,
    ) -> DbResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO offline_queue (action_type, endpoint, method, payload_json)
             VALUES (?, ?, ?, ?)",
            params![action_type, endpoint, method, payload],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get pending offline actions
    pub fn get_offline_queue(&self) -> DbResult<Vec<OfflineAction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, action_type, endpoint, method, payload_json, retry_count
             FROM offline_queue ORDER BY created_at ASC",
        )?;

        let actions = stmt
            .query_map([], |row| {
                Ok(OfflineAction {
                    id: row.get(0)?,
                    action_type: row.get(1)?,
                    endpoint: row.get(2)?,
                    method: row.get(3)?,
                    payload_json: row.get(4)?,
                    retry_count: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(actions)
    }

    /// Remove action from offline queue
    pub fn remove_from_queue(&self, id: i64) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM offline_queue WHERE id = ?", params![id])?;
        Ok(())
    }

    /// Increment retry count for an action
    pub fn increment_retry(&self, id: i64, error: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE offline_queue SET retry_count = retry_count + 1, last_error = ? WHERE id = ?",
            params![error, id],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // FAVORITES
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add a favorite slot
    pub fn add_favorite(
        &self,
        user_id: &str,
        lot_id: &str,
        slot_id: &str,
        slot_number: i32,
        lot_name: &str,
    ) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO favorite_slots (user_id, lot_id, slot_id, slot_number, lot_name)
             VALUES (?, ?, ?, ?, ?)",
            params![user_id, lot_id, slot_id, slot_number, lot_name],
        )?;
        Ok(())
    }

    /// Remove a favorite slot
    pub fn remove_favorite(&self, user_id: &str, slot_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM favorite_slots WHERE user_id = ? AND slot_id = ?",
            params![user_id, slot_id],
        )?;
        Ok(())
    }

    /// Get all favorite slots for a user
    pub fn get_favorites(&self, user_id: &str) -> DbResult<Vec<FavoriteSlot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT lot_id, slot_id, slot_number, lot_name FROM favorite_slots WHERE user_id = ?",
        )?;

        let favorites = stmt
            .query_map(params![user_id], |row| {
                Ok(FavoriteSlot {
                    lot_id: row.get(0)?,
                    slot_id: row.get(1)?,
                    slot_number: row.get(2)?,
                    lot_name: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(favorites)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // APP SETTINGS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get a setting value
    pub fn get_setting(&self, key: &str) -> DbResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result)
    }

    /// Set a setting value
    pub fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value, updated_at)
             VALUES (?, ?, datetime('now'))",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a setting
    pub fn delete_setting(&self, key: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM app_settings WHERE key = ?", params![key])?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NOTIFICATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save a notification
    pub fn save_notification(&self, notification: &NotificationData) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO notifications (id, user_id, notification_type, title, message, data_json, read, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                notification.id,
                notification.user_id,
                notification.notification_type,
                notification.title,
                notification.message,
                notification.data_json,
                notification.read as i32,
                notification.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get notifications for a user
    pub fn get_notifications(&self, user_id: &str, limit: i32) -> DbResult<Vec<NotificationData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, notification_type, title, message, data_json, read, created_at
             FROM notifications WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
        )?;

        let notifications = stmt
            .query_map(params![user_id, limit], |row| {
                Ok(NotificationData {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    notification_type: row.get(2)?,
                    title: row.get(3)?,
                    message: row.get(4)?,
                    data_json: row.get(5)?,
                    read: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(notifications)
    }

    /// Get unread notification count
    pub fn get_unread_count(&self, user_id: &str) -> DbResult<i32> {
        let conn = self.conn.lock().unwrap();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read = 0",
            params![user_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Mark notification as read
    pub fn mark_notification_read(&self, notification_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE notifications SET read = 1 WHERE id = ?",
            params![notification_id],
        )?;
        Ok(())
    }

    /// Mark all notifications as read
    pub fn mark_all_notifications_read(&self, user_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE notifications SET read = 1 WHERE user_id = ?",
            params![user_id],
        )?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════════

/// Session data from database
#[derive(Debug, Clone)]
pub struct SessionData {
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub role: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expires_at: String,
}

/// Vehicle data for database
#[derive(Debug, Clone)]
pub struct VehicleData {
    pub id: String,
    pub user_id: String,
    pub license_plate: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub vehicle_type: String,
    pub is_default: bool,
}

/// Booking data for database
#[derive(Debug, Clone)]
pub struct BookingData {
    pub id: String,
    pub user_id: String,
    pub lot_id: String,
    pub slot_id: String,
    pub slot_number: i32,
    pub floor_name: Option<String>,
    pub vehicle_json: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub pricing_json: Option<String>,
    pub qr_code: Option<String>,
    pub notes: Option<String>,
    pub synced: bool,
}

/// Offline action from queue
#[derive(Debug, Clone)]
pub struct OfflineAction {
    pub id: i64,
    pub action_type: String,
    pub endpoint: String,
    pub method: String,
    pub payload_json: Option<String>,
    pub retry_count: i32,
}

/// Favorite slot
#[derive(Debug, Clone)]
pub struct FavoriteSlot {
    pub lot_id: String,
    pub slot_id: String,
    pub slot_number: i32,
    pub lot_name: Option<String>,
}

/// Notification data
#[derive(Debug, Clone)]
pub struct NotificationData {
    pub id: String,
    pub user_id: String,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub data_json: Option<String>,
    pub read: bool,
    pub created_at: String,
}
