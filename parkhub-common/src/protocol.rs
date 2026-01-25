//! Protocol Definitions
//!
//! API request/response types and real-time message definitions
//! for client-server communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::*;

// ═══════════════════════════════════════════════════════════════════════════════
// API REQUEST/RESPONSE
// ═══════════════════════════════════════════════════════════════════════════════

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub meta: Option<ResponseMeta>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: None,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
            meta: None,
        }
    }
}

/// API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Response metadata for pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub total: Option<i32>,
    pub total_pages: Option<i32>,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: i32,
    pub per_page: i32,
    pub total: i32,
    pub total_pages: i32,
}

// ═══════════════════════════════════════════════════════════════════════════════
// AUTHENTICATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Login request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub tokens: AuthTokens,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Register new user request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SERVER DISCOVERY
// ═══════════════════════════════════════════════════════════════════════════════

/// Server information broadcast via mDNS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub fingerprint: Option<String>,
}

/// Server handshake request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub client_version: String,
    pub protocol_version: String,
}

/// Server handshake response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub server_name: String,
    pub server_version: String,
    pub protocol_version: String,
    pub requires_auth: bool,
    pub certificate_fingerprint: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// REAL-TIME EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Real-time slot update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotUpdateEvent {
    pub lot_id: String,
    pub slot_id: String,
    pub slot_number: i32,
    pub floor_id: String,
    pub previous_status: SlotStatus,
    pub new_status: SlotStatus,
    pub booking_info: Option<SlotBookingInfo>,
    pub timestamp: DateTime<Utc>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    #[serde(rename = "slot_update")]
    SlotUpdate(SlotUpdateEvent),

    #[serde(rename = "booking_update")]
    BookingUpdate(Booking),

    #[serde(rename = "notification")]
    Notification(Notification),

    #[serde(rename = "server_info")]
    ServerInfo(ServerInfo),

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "error")]
    Error(ApiError),
}

// ═══════════════════════════════════════════════════════════════════════════════
// SERVER CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Server configuration (for onboarding)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_name: String,
    pub port: u16,
    pub enable_tls: bool,
    pub enable_mdns: bool,
    pub admin_username: String,
    pub data_directory: String,
}

/// Server status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub uptime_seconds: u64,
    pub connected_clients: u32,
    pub total_users: u32,
    pub total_bookings: u32,
    pub database_size_bytes: u64,
}
