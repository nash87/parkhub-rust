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

    // ── HEAD: ApiResponse builder tests ─────────────────────────────────────

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

    // ── HEAD: PaginatedResponse tests ───────────────────────────────────────

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

    // ── HEAD: Authentication DTOs ───────────────────────────────────────────

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

    // ── HEAD: Server discovery DTOs ─────────────────────────────────────────

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

    // ── HEAD: WebSocket messages ────────────────────────────────────────────

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

    // ── HEAD: ServerConfig and ServerStatus ─────────────────────────────────

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

    // ── HEAD: ResponseMeta ──────────────────────────────────────────────────

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

    // ── Copilot: ApiResponse::success ───────────────────────────────────────

    #[test]
    fn test_api_response_success_sets_flag() {
        let r: ApiResponse<i32> = ApiResponse::success(42);
        assert!(r.success);
        assert_eq!(r.data, Some(42));
        assert!(r.error.is_none());
        assert!(r.meta.is_none());
    }

    #[test]
    fn test_api_response_success_with_string() {
        let r = ApiResponse::success("hello".to_string());
        assert!(r.success);
        assert_eq!(r.data.as_deref(), Some("hello"));
    }

    #[test]
    fn test_api_response_success_with_vec() {
        let r: ApiResponse<Vec<i32>> = ApiResponse::success(vec![1, 2, 3]);
        assert!(r.success);
        assert_eq!(r.data, Some(vec![1, 2, 3]));
    }

    // ── Copilot: ApiResponse::error ─────────────────────────────────────────

    #[test]
    fn test_api_response_error_sets_flag() {
        let r: ApiResponse<()> = ApiResponse::error("NOT_FOUND", "Resource not found");
        assert!(!r.success);
        assert!(r.data.is_none());
        assert!(r.error.is_some());
        let err = r.error.unwrap();
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.message, "Resource not found");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_api_response_error_with_owned_strings() {
        let r: ApiResponse<i32> =
            ApiResponse::error("CONFLICT".to_string(), "Already booked".to_string());
        assert!(!r.success);
        let err = r.error.unwrap();
        assert_eq!(err.code, "CONFLICT");
        assert_eq!(err.message, "Already booked");
    }

    // ── Copilot: ApiResponse serialization round-trip ───────────────────────

    #[test]
    fn test_api_response_success_roundtrip() {
        let original = ApiResponse::success(99_i32);
        let json = serde_json::to_string(&original).unwrap();
        let back: ApiResponse<i32> = serde_json::from_str(&json).unwrap();
        assert!(back.success);
        assert_eq!(back.data, Some(99));
    }

    #[test]
    fn test_api_response_error_roundtrip() {
        let original: ApiResponse<String> = ApiResponse::error("BAD_REQUEST", "bad input");
        let json = serde_json::to_string(&original).unwrap();
        let back: ApiResponse<String> = serde_json::from_str(&json).unwrap();
        assert!(!back.success);
        let err = back.error.unwrap();
        assert_eq!(err.code, "BAD_REQUEST");
        assert_eq!(err.message, "bad input");
    }

    // ── Copilot: PaginatedResponse ──────────────────────────────────────────

    #[test]
    fn test_paginated_response_roundtrip() {
        let pr = PaginatedResponse {
            items: vec![1_i32, 2, 3],
            page: 1,
            per_page: 10,
            total: 3,
            total_pages: 1,
        };
        let json = serde_json::to_string(&pr).unwrap();
        let back: PaginatedResponse<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.items, vec![1, 2, 3]);
        assert_eq!(back.page, 1);
        assert_eq!(back.per_page, 10);
        assert_eq!(back.total, 3);
        assert_eq!(back.total_pages, 1);
    }

    #[test]
    fn test_paginated_response_empty() {
        let pr: PaginatedResponse<String> = PaginatedResponse {
            items: vec![],
            page: 1,
            per_page: 20,
            total: 0,
            total_pages: 0,
        };
        assert!(pr.items.is_empty());
        assert_eq!(pr.total, 0);
    }

    // ── Copilot: ResponseMeta ───────────────────────────────────────────────

    #[test]
    fn test_response_meta_all_none() {
        let meta = ResponseMeta {
            page: None,
            per_page: None,
            total: None,
            total_pages: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: ResponseMeta = serde_json::from_str(&json).unwrap();
        assert!(back.page.is_none());
        assert!(back.total.is_none());
    }

    #[test]
    fn test_response_meta_with_values() {
        let meta = ResponseMeta {
            page: Some(2),
            per_page: Some(25),
            total: Some(100),
            total_pages: Some(4),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: ResponseMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.page, Some(2));
        assert_eq!(back.per_page, Some(25));
        assert_eq!(back.total, Some(100));
        assert_eq!(back.total_pages, Some(4));
    }

    // ── Copilot: LoginRequest / RegisterRequest ─────────────────────────────

    #[test]
    fn test_login_request_roundtrip() {
        let req = LoginRequest {
            username: "alice".to_string(),
            password: "s3cr3t".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.username, "alice");
        assert_eq!(back.password, "s3cr3t");
    }

    #[test]
    fn test_register_request_roundtrip() {
        let req = RegisterRequest {
            email: "bob@example.com".to_string(),
            password: "Password1".to_string(),
            password_confirmation: "Password1".to_string(),
            name: "Bob".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: RegisterRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.email, "bob@example.com");
        assert_eq!(back.name, "Bob");
    }

    #[test]
    fn test_refresh_token_request_roundtrip() {
        let req = RefreshTokenRequest {
            refresh_token: "tok_abc".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: RefreshTokenRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.refresh_token, "tok_abc");
    }

    // ── Copilot: WsMessage tag-based serialization ──────────────────────────

    #[test]
    fn test_ws_message_ping_tag() {
        let msg = WsMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ping\""));
    }

    #[test]
    fn test_ws_message_pong_tag() {
        let msg = WsMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"pong\""));
    }

    #[test]
    fn test_ws_message_error_tag() {
        let api_err = ApiError {
            code: "SERVER_ERROR".to_string(),
            message: "Something went wrong".to_string(),
            details: None,
        };
        let msg = WsMessage::Error(api_err);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("SERVER_ERROR"));
    }

    // ── Copilot: HandshakeRequest / HandshakeResponse ───────────────────────

    #[test]
    fn test_handshake_request_roundtrip() {
        let req = HandshakeRequest {
            client_version: "1.2.3".to_string(),
            protocol_version: "1.0.0".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: HandshakeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.client_version, "1.2.3");
        assert_eq!(back.protocol_version, "1.0.0");
    }

    #[test]
    fn test_handshake_response_roundtrip() {
        let resp = HandshakeResponse {
            server_name: "ParkHub".to_string(),
            server_version: "0.9.0".to_string(),
            protocol_version: "1.0.0".to_string(),
            requires_auth: true,
            certificate_fingerprint: "aa:bb:cc".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: HandshakeResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_name, "ParkHub");
        assert!(back.requires_auth);
        assert_eq!(back.certificate_fingerprint, "aa:bb:cc");
    }

    // ── Copilot: ServerInfo ─────────────────────────────────────────────────

    #[test]
    fn test_server_info_roundtrip() {
        let info = ServerInfo {
            name: "MyHub".to_string(),
            version: "1.0.0".to_string(),
            protocol_version: "1.0.0".to_string(),
            host: "192.168.1.1".to_string(),
            port: 8080,
            tls: true,
            fingerprint: Some("fp123".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "MyHub");
        assert_eq!(back.port, 8080);
        assert!(back.tls);
        assert_eq!(back.fingerprint, Some("fp123".to_string()));
    }

    // ── Copilot: ServerStatus ───────────────────────────────────────────────

    #[test]
    fn test_server_status_roundtrip() {
        let status = ServerStatus {
            uptime_seconds: 3600,
            connected_clients: 5,
            total_users: 42,
            total_bookings: 1024,
            database_size_bytes: 204_800,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: ServerStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.uptime_seconds, 3600);
        assert_eq!(back.connected_clients, 5);
        assert_eq!(back.total_users, 42);
        assert_eq!(back.total_bookings, 1024);
        assert_eq!(back.database_size_bytes, 204_800);
    }

    // ── Copilot: ServerConfig ───────────────────────────────────────────────

    #[test]
    fn test_server_config_roundtrip() {
        let cfg = ServerConfig {
            server_name: "ParkHub HQ".to_string(),
            port: 8080,
            enable_tls: true,
            enable_mdns: false,
            admin_username: "admin".to_string(),
            data_directory: "/data".to_string(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: ServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_name, "ParkHub HQ");
        assert_eq!(back.port, 8080);
        assert!(back.enable_tls);
        assert!(!back.enable_mdns);
    }
}
