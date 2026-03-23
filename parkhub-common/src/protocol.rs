//! Protocol Definitions
//!
//! API request/response types and real-time message definitions
//! for client-server communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{AuthTokens, Booking, Notification, SlotBookingInfo, SlotStatus, User};

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
    pub const fn success(data: T) -> Self {
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Register new user request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
    BookingUpdate(Box<Booking>),

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── ApiResponse builder tests ───────────────────────────────────────────

    #[test]
    fn api_response_success_sets_fields_correctly() {
        let resp = ApiResponse::success("hello");
        assert!(resp.success);
        assert_eq!(resp.data, Some("hello"));
        assert!(resp.error.is_none());
        assert!(resp.meta.is_none());
    }

    #[test]
    fn api_response_error_sets_fields_correctly() {
        let resp = ApiResponse::<()>::error("NOT_FOUND", "Item missing");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.message, "Item missing");
        assert!(err.details.is_none());
    }

    #[test]
    fn api_response_success_serde_round_trip() {
        let resp = ApiResponse::success(42);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ApiResponse<i32> = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.data, Some(42));
    }

    #[test]
    fn api_response_error_serde_round_trip() {
        let resp = ApiResponse::<String>::error("ERR", "something broke");
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ApiResponse<String> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.success);
        assert_eq!(parsed.error.as_ref().unwrap().code, "ERR");
    }

    #[test]
    fn api_response_with_complex_data() {
        let resp = ApiResponse::success(vec![1, 2, 3]);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ApiResponse<Vec<i32>> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data.unwrap(), vec![1, 2, 3]);
    }

    // ── PaginatedResponse tests ─────────────────────────────────────────────

    #[test]
    fn paginated_response_serde_round_trip() {
        let resp = PaginatedResponse {
            items: vec!["a".to_string(), "b".to_string()],
            page: 1,
            per_page: 10,
            total: 2,
            total_pages: 1,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: PaginatedResponse<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.page, 1);
        assert_eq!(parsed.total, 2);
        assert_eq!(parsed.total_pages, 1);
    }

    #[test]
    fn paginated_response_empty_items() {
        let resp = PaginatedResponse::<i32> {
            items: vec![],
            page: 1,
            per_page: 10,
            total: 0,
            total_pages: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: PaginatedResponse<i32> = serde_json::from_str(&json).unwrap();
        assert!(parsed.items.is_empty());
        assert_eq!(parsed.total, 0);
    }

    // ── Authentication DTOs ─────────────────────────────────────────────────

    #[test]
    fn login_request_serde() {
        let json = r#"{"username":"alice","password":"secret"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice");
        assert_eq!(req.password, "secret");
    }

    #[test]
    fn refresh_token_request_serde() {
        let json = r#"{"refresh_token":"tok-123"}"#;
        let req: RefreshTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "tok-123");
    }

    #[test]
    fn register_request_serde() {
        let json = r#"{
            "email":"a@b.com",
            "password":"pass123",
            "password_confirmation":"pass123",
            "name":"Alice"
        }"#;
        let req: RegisterRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "a@b.com");
        assert_eq!(req.name, "Alice");
        assert_eq!(req.password, req.password_confirmation);
    }

    // ── Server discovery DTOs ───────────────────────────────────────────────

    #[test]
    fn server_info_serde_round_trip() {
        let info = ServerInfo {
            name: "ParkHub".into(),
            version: "4.3.0".into(),
            protocol_version: "1.0.0".into(),
            host: "192.168.1.1".into(),
            port: 7878,
            tls: true,
            fingerprint: Some("AA:BB:CC".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "ParkHub");
        assert_eq!(parsed.port, 7878);
        assert!(parsed.tls);
        assert_eq!(parsed.fingerprint.unwrap(), "AA:BB:CC");
    }

    #[test]
    fn server_info_no_fingerprint() {
        let info = ServerInfo {
            name: "test".into(),
            version: "1.0".into(),
            protocol_version: "1.0.0".into(),
            host: "localhost".into(),
            port: 8080,
            tls: false,
            fingerprint: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"fingerprint\":null"));
    }

    #[test]
    fn handshake_request_serde() {
        let json = r#"{"client_version":"1.0","protocol_version":"1.0.0"}"#;
        let req: HandshakeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.client_version, "1.0");
        assert_eq!(req.protocol_version, "1.0.0");
    }

    #[test]
    fn handshake_response_serde() {
        let resp = HandshakeResponse {
            server_name: "ParkHub".into(),
            server_version: "4.3.0".into(),
            protocol_version: "1.0.0".into(),
            requires_auth: true,
            certificate_fingerprint: "AA:BB".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: HandshakeResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.requires_auth);
        assert_eq!(parsed.certificate_fingerprint, "AA:BB");
    }

    // ── WebSocket messages ──────────────────────────────────────────────────

    #[test]
    fn ws_message_ping_serde() {
        let msg = WsMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WsMessage::Ping));
    }

    #[test]
    fn ws_message_pong_serde() {
        let msg = WsMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WsMessage::Pong));
    }

    #[test]
    fn ws_message_error_serde() {
        let msg = WsMessage::Error(ApiError {
            code: "TIMEOUT".into(),
            message: "timed out".into(),
            details: None,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            WsMessage::Error(e) => assert_eq!(e.code, "TIMEOUT"),
            other => panic!("Expected Error, got {other:?}"),
        }
    }

    #[test]
    fn ws_message_tagged_format() {
        let msg = WsMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ping\""));
    }

    // ── ServerConfig and ServerStatus ────────────────────────────────────────

    #[test]
    fn server_config_serde_round_trip() {
        let config = ServerConfig {
            server_name: "Test".into(),
            port: 9090,
            enable_tls: false,
            enable_mdns: true,
            admin_username: "admin".into(),
            data_directory: "/data".into(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.server_name, "Test");
        assert_eq!(parsed.port, 9090);
        assert!(!parsed.enable_tls);
        assert!(parsed.enable_mdns);
    }

    #[test]
    fn server_status_serde_round_trip() {
        let status = ServerStatus {
            uptime_seconds: 3600,
            connected_clients: 5,
            total_users: 100,
            total_bookings: 500,
            database_size_bytes: 1_048_576,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: ServerStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uptime_seconds, 3600);
        assert_eq!(parsed.database_size_bytes, 1_048_576);
    }

    // ── ResponseMeta ────────────────────────────────────────────────────────

    #[test]
    fn response_meta_all_none() {
        let meta = ResponseMeta {
            page: None,
            per_page: None,
            total: None,
            total_pages: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: ResponseMeta = serde_json::from_str(&json).unwrap();
        assert!(parsed.page.is_none());
    }

    #[test]
    fn response_meta_with_values() {
        let meta = ResponseMeta {
            page: Some(2),
            per_page: Some(25),
            total: Some(100),
            total_pages: Some(4),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: ResponseMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.page, Some(2));
        assert_eq!(parsed.total_pages, Some(4));
    }
}
