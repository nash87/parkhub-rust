//! Database Schema
//!
//! SQL schema definitions for the local SQLite database.

/// Schema version for migrations
pub const SCHEMA_VERSION: i32 = 1;

/// SQL to create all tables
pub const CREATE_SCHEMA: &str = r#"
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- User session storage
CREATE TABLE IF NOT EXISTS user_session (
    id INTEGER PRIMARY KEY,
    user_id TEXT NOT NULL,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    picture TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    token_expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- User preferences (cached)
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id TEXT PRIMARY KEY,
    default_duration_minutes INTEGER DEFAULT 60,
    favorite_slots TEXT DEFAULT '[]',
    notifications_enabled INTEGER DEFAULT 1,
    email_reminders INTEGER DEFAULT 1,
    language TEXT DEFAULT 'de',
    theme TEXT DEFAULT 'dark',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Saved vehicles
CREATE TABLE IF NOT EXISTS vehicles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    license_plate TEXT NOT NULL,
    make TEXT,
    model TEXT,
    color TEXT,
    vehicle_type TEXT DEFAULT 'car',
    is_default INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_vehicles_user ON vehicles(user_id);

-- Cached parking lots
CREATE TABLE IF NOT EXISTS parking_lots (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    latitude REAL,
    longitude REAL,
    total_slots INTEGER NOT NULL,
    available_slots INTEGER NOT NULL,
    pricing_json TEXT,
    operating_hours_json TEXT,
    amenities_json TEXT,
    status TEXT DEFAULT 'open',
    cached_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Cached parking floors
CREATE TABLE IF NOT EXISTS parking_floors (
    id TEXT PRIMARY KEY,
    lot_id TEXT NOT NULL,
    name TEXT NOT NULL,
    floor_number INTEGER NOT NULL,
    total_slots INTEGER NOT NULL,
    available_slots INTEGER NOT NULL,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (lot_id) REFERENCES parking_lots(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_floors_lot ON parking_floors(lot_id);

-- Cached parking slots
CREATE TABLE IF NOT EXISTS parking_slots (
    id TEXT PRIMARY KEY,
    lot_id TEXT NOT NULL,
    floor_id TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    row INTEGER NOT NULL,
    col INTEGER NOT NULL,
    slot_type TEXT DEFAULT 'standard',
    status TEXT DEFAULT 'available',
    features_json TEXT DEFAULT '[]',
    position_json TEXT,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (lot_id) REFERENCES parking_lots(id) ON DELETE CASCADE,
    FOREIGN KEY (floor_id) REFERENCES parking_floors(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_slots_lot ON parking_slots(lot_id);
CREATE INDEX IF NOT EXISTS idx_slots_floor ON parking_slots(floor_id);

-- Bookings (local cache and offline queue)
CREATE TABLE IF NOT EXISTS bookings (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    lot_id TEXT NOT NULL,
    slot_id TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    floor_name TEXT,
    vehicle_json TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    pricing_json TEXT,
    qr_code TEXT,
    notes TEXT,
    check_in_time TEXT,
    check_out_time TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced INTEGER DEFAULT 1,
    sync_action TEXT
);

CREATE INDEX IF NOT EXISTS idx_bookings_user ON bookings(user_id);
CREATE INDEX IF NOT EXISTS idx_bookings_status ON bookings(status);
CREATE INDEX IF NOT EXISTS idx_bookings_synced ON bookings(synced);

-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    data_json TEXT,
    read INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    received_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(read);

-- Offline action queue
CREATE TABLE IF NOT EXISTS offline_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_type TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    payload_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    retry_count INTEGER DEFAULT 0,
    last_error TEXT
);

-- App settings
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Statistics cache
CREATE TABLE IF NOT EXISTS statistics_cache (
    user_id TEXT PRIMARY KEY,
    stats_json TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Favorite slots
CREATE TABLE IF NOT EXISTS favorite_slots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    lot_id TEXT NOT NULL,
    slot_id TEXT NOT NULL,
    slot_number INTEGER NOT NULL,
    lot_name TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, slot_id)
);

CREATE INDEX IF NOT EXISTS idx_favorites_user ON favorite_slots(user_id);

-- Recent searches
CREATE TABLE IF NOT EXISTS recent_searches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    search_type TEXT NOT NULL,
    query TEXT NOT NULL,
    result_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_recent_user ON recent_searches(user_id);
"#;

/// SQL to drop all tables (for reset)
pub const DROP_SCHEMA: &str = r#"
DROP TABLE IF EXISTS recent_searches;
DROP TABLE IF EXISTS favorite_slots;
DROP TABLE IF EXISTS statistics_cache;
DROP TABLE IF EXISTS app_settings;
DROP TABLE IF EXISTS offline_queue;
DROP TABLE IF EXISTS notifications;
DROP TABLE IF EXISTS bookings;
DROP TABLE IF EXISTS parking_slots;
DROP TABLE IF EXISTS parking_floors;
DROP TABLE IF EXISTS parking_lots;
DROP TABLE IF EXISTS vehicles;
DROP TABLE IF EXISTS user_preferences;
DROP TABLE IF EXISTS user_session;
DROP TABLE IF EXISTS schema_version;
"#;
