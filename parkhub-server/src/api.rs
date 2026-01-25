//! HTTP API Routes
//!
//! RESTful API for the parking system.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use parkhub_common::{ApiResponse, HandshakeRequest, HandshakeResponse, ServerStatus, PROTOCOL_VERSION};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// Create the API router
pub fn create_router(state: SharedState) -> Router {
    Router::new()
        // Health & Discovery
        .route("/health", get(health_check))
        .route("/handshake", post(handshake))
        .route("/status", get(server_status))
        // Authentication
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/refresh", post(refresh_token))
        // Users
        .route("/api/v1/users/me", get(get_current_user))
        .route("/api/v1/users/:id", get(get_user))
        // Parking lots
        .route("/api/v1/lots", get(list_lots))
        .route("/api/v1/lots/:id", get(get_lot))
        .route("/api/v1/lots/:id/slots", get(get_lot_slots))
        // Bookings
        .route("/api/v1/bookings", get(list_bookings).post(create_booking))
        .route("/api/v1/bookings/:id", get(get_booking).delete(cancel_booking))
        // State and middleware
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEALTH & DISCOVERY
// ═══════════════════════════════════════════════════════════════════════════════

async fn health_check() -> &'static str {
    "OK"
}

async fn handshake(
    State(state): State<SharedState>,
    Json(request): Json<HandshakeRequest>,
) -> Json<ApiResponse<HandshakeResponse>> {
    let state = state.read().await;

    // Check protocol version compatibility
    if request.protocol_version != PROTOCOL_VERSION {
        return Json(ApiResponse::error(
            "PROTOCOL_MISMATCH",
            format!(
                "Protocol version mismatch: server={}, client={}",
                PROTOCOL_VERSION, request.protocol_version
            ),
        ));
    }

    Json(ApiResponse::success(HandshakeResponse {
        server_name: state.config.server_name.clone(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        requires_auth: true,
        certificate_fingerprint: String::new(), // TODO: Add actual fingerprint
    }))
}

async fn server_status(State(state): State<SharedState>) -> Json<ApiResponse<ServerStatus>> {
    let state = state.read().await;
    let db_stats = state.db.stats().await.unwrap_or_else(|_| crate::db::DbStats {
        users: 0,
        bookings: 0,
        parking_lots: 0,
    });

    Json(ApiResponse::success(ServerStatus {
        uptime_seconds: 0, // TODO: Track uptime
        connected_clients: 0,
        total_users: db_stats.users as u32,
        total_bookings: db_stats.bookings as u32,
        database_size_bytes: 0,
    }))
}

// ═══════════════════════════════════════════════════════════════════════════════
// AUTHENTICATION
// ═══════════════════════════════════════════════════════════════════════════════

async fn login(
    State(_state): State<SharedState>,
    Json(_request): Json<parkhub_common::LoginRequest>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::LoginResponse>>) {
    // TODO: Implement actual authentication
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("NOT_IMPLEMENTED", "Authentication not yet implemented")),
    )
}

async fn register(
    State(_state): State<SharedState>,
    Json(_request): Json<parkhub_common::RegisterRequest>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::LoginResponse>>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("NOT_IMPLEMENTED", "Registration not yet implemented")),
    )
}

async fn refresh_token(
    State(_state): State<SharedState>,
    Json(_request): Json<parkhub_common::RefreshTokenRequest>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::AuthTokens>>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("NOT_IMPLEMENTED", "Token refresh not yet implemented")),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// USERS
// ═══════════════════════════════════════════════════════════════════════════════

async fn get_current_user(
    State(_state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::User>>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiResponse::error("UNAUTHORIZED", "Not authenticated")),
    )
}

async fn get_user(
    State(_state): State<SharedState>,
    Path(_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::User>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("NOT_FOUND", "User not found")),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// PARKING LOTS
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_lots(
    State(_state): State<SharedState>,
) -> Json<ApiResponse<Vec<parkhub_common::ParkingLot>>> {
    // TODO: Return actual lots from database
    Json(ApiResponse::success(vec![]))
}

async fn get_lot(
    State(_state): State<SharedState>,
    Path(_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::ParkingLot>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
    )
}

async fn get_lot_slots(
    State(_state): State<SharedState>,
    Path(_id): Path<String>,
) -> Json<ApiResponse<Vec<parkhub_common::ParkingSlot>>> {
    Json(ApiResponse::success(vec![]))
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_bookings(
    State(_state): State<SharedState>,
) -> Json<ApiResponse<Vec<parkhub_common::Booking>>> {
    Json(ApiResponse::success(vec![]))
}

async fn create_booking(
    State(_state): State<SharedState>,
    Json(_request): Json<parkhub_common::CreateBookingRequest>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::Booking>>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("NOT_IMPLEMENTED", "Booking creation not yet implemented")),
    )
}

async fn get_booking(
    State(_state): State<SharedState>,
    Path(_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<parkhub_common::Booking>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
    )
}

async fn cancel_booking(
    State(_state): State<SharedState>,
    Path(_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
    )
}
