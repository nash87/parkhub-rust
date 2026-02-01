//! HTTP API Routes
//!
//! RESTful API for the parking system.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use parkhub_common::{
    ApiResponse, AuthTokens, Booking, BookingPricing, BookingStatus, CreateBookingRequest,
    HandshakeRequest, HandshakeResponse, LoginRequest, LoginResponse, ParkingLot, ParkingSlot,
    PaymentStatus, RefreshTokenRequest, RegisterRequest, ServerStatus, SlotStatus, User,
    UserPreferences, UserRole, Vehicle, VehicleType, PROTOCOL_VERSION,
};

use crate::db::Session;
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// User ID extracted from auth token
#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

/// Create the API router
pub fn create_router(state: SharedState) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/handshake", post(handshake))
        .route("/status", get(server_status))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/refresh", post(refresh_token));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/api/v1/users/me", get(get_current_user))
        .route("/api/v1/users/:id", get(get_user))
        .route("/api/v1/lots", get(list_lots).post(create_lot))
        .route("/api/v1/lots/:id", get(get_lot))
        .route("/api/v1/lots/:id/slots", get(get_lot_slots))
        .route("/api/v1/bookings", get(list_bookings).post(create_booking))
        .route(
            "/api/v1/bookings/:id",
            get(get_booking).delete(cancel_booking),
        )
        .route("/api/v1/vehicles", get(list_vehicles).post(create_vehicle))
        .route("/api/v1/vehicles/:id", delete(delete_vehicle))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
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
// AUTH MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

async fn auth_middleware(
    State(state): State<SharedState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract bearer token
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(
                    "UNAUTHORIZED",
                    "Missing or invalid authorization header",
                )),
            ));
        }
    };

    // Validate session
    let state_guard = state.read().await;
    let session = match state_guard.db.get_session(token).await {
        Ok(Some(s)) if !s.is_expired() => s,
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("UNAUTHORIZED", "Invalid or expired token")),
            ));
        }
    };
    drop(state_guard);

    // Insert user info into request extensions
    request.extensions_mut().insert(AuthUser {
        user_id: session.user_id,
    });

    Ok(next.run(request).await)
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
        certificate_fingerprint: String::new(),
    }))
}

async fn server_status(State(state): State<SharedState>) -> Json<ApiResponse<ServerStatus>> {
    let state = state.read().await;
    let db_stats = state
        .db
        .stats()
        .await
        .unwrap_or_else(|_| crate::db::DatabaseStats {
            users: 0,
            bookings: 0,
            parking_lots: 0,
            slots: 0,
            sessions: 0,
        });

    Json(ApiResponse::success(ServerStatus {
        uptime_seconds: 0,
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
    State(state): State<SharedState>,
    Json(request): Json<LoginRequest>,
) -> (StatusCode, Json<ApiResponse<LoginResponse>>) {
    let state_guard = state.read().await;

    // Find user by username
    let user = match state_guard.db.get_user_by_username(&request.username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Also try by email
            match state_guard.db.get_user_by_email(&request.username).await {
                Ok(Some(u)) => u,
                _ => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(ApiResponse::error(
                            "INVALID_CREDENTIALS",
                            "Invalid username or password",
                        )),
                    );
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Verify password
    if !verify_password(&request.password, &user.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error(
                "INVALID_CREDENTIALS",
                "Invalid username or password",
            )),
        );
    }

    // Check if user is active
    if !user.is_active {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "ACCOUNT_DISABLED",
                "This account has been disabled",
            )),
        );
    }

    // Create session
    let session = Session::new(user.id, 24); // 24 hour session
    let access_token = Uuid::new_v4().to_string();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create session")),
        );
    }

    // Create response (remove password_hash from response)
    let mut response_user = user.clone();
    response_user.password_hash = String::new();

    (
        StatusCode::OK,
        Json(ApiResponse::success(LoginResponse {
            user: response_user,
            tokens: AuthTokens {
                access_token,
                refresh_token: session.refresh_token,
                expires_at: session.expires_at,
                token_type: "Bearer".to_string(),
            },
        })),
    )
}

async fn register(
    State(state): State<SharedState>,
    Json(request): Json<RegisterRequest>,
) -> (StatusCode, Json<ApiResponse<LoginResponse>>) {
    let state_guard = state.read().await;

    // Check if email already exists
    if let Ok(Some(_)) = state_guard.db.get_user_by_email(&request.email).await {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "EMAIL_EXISTS",
                "An account with this email already exists",
            )),
        );
    }

    // Generate username from email
    let username = request
        .email
        .split('@')
        .next()
        .unwrap_or("user")
        .to_string();

    // Check if username already exists, append number if needed
    let mut final_username = username.clone();
    let mut counter = 1;
    while let Ok(Some(_)) = state_guard.db.get_user_by_username(&final_username).await {
        final_username = format!("{}{}", username, counter);
        counter += 1;
    }

    // Hash password
    let password_hash = hash_password(&request.password);

    // Create user
    let now = Utc::now();
    let user = User {
        id: Uuid::new_v4(),
        username: final_username,
        email: request.email,
        password_hash,
        name: request.name,
        picture: None,
        phone: None,
        role: UserRole::User,
        created_at: now,
        updated_at: now,
        last_login: Some(now),
        preferences: UserPreferences::default(),
        is_active: true,
    };

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create account")),
        );
    }

    // Create session
    let session = Session::new(user.id, 24);
    let access_token = Uuid::new_v4().to_string();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create session")),
        );
    }

    // Create response
    let mut response_user = user.clone();
    response_user.password_hash = String::new();

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(LoginResponse {
            user: response_user,
            tokens: AuthTokens {
                access_token,
                refresh_token: session.refresh_token,
                expires_at: session.expires_at,
                token_type: "Bearer".to_string(),
            },
        })),
    )
}

async fn refresh_token(
    State(_state): State<SharedState>,
    Json(_request): Json<RefreshTokenRequest>,
) -> (StatusCode, Json<ApiResponse<AuthTokens>>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error(
            "NOT_IMPLEMENTED",
            "Token refresh not yet fully implemented",
        )),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// USERS
// ═══════════════════════════════════════════════════════════════════════════════

async fn get_current_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(mut user)) => {
            user.password_hash = String::new();
            (StatusCode::OK, Json(ApiResponse::success(user)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

async fn get_user(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    match state.db.get_user(&id).await {
        Ok(Some(mut user)) => {
            user.password_hash = String::new();
            (StatusCode::OK, Json(ApiResponse::success(user)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PARKING LOTS
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_lots(State(state): State<SharedState>) -> Json<ApiResponse<Vec<ParkingLot>>> {
    let state = state.read().await;

    match state.db.list_parking_lots().await {
        Ok(lots) => Json(ApiResponse::success(lots)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list parking lots",
            ))
        }
    }
}

async fn create_lot(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(lot): Json<ParkingLot>,
) -> (StatusCode, Json<ApiResponse<ParkingLot>>) {
    let state_guard = state.read().await;

    // Check if user is admin
    let user = match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    if let Err(e) = state_guard.db.save_parking_lot(&lot).await {
        tracing::error!("Failed to save parking lot: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create parking lot",
            )),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(lot)))
}

async fn get_lot(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ParkingLot>>) {
    let state = state.read().await;

    match state.db.get_parking_lot(&id).await {
        Ok(Some(lot)) => (StatusCode::OK, Json(ApiResponse::success(lot))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

async fn get_lot_slots(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<ParkingSlot>>> {
    let state = state.read().await;

    match state.db.list_slots_by_lot(&id).await {
        Ok(slots) => Json(ApiResponse::success(slots)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error("SERVER_ERROR", "Failed to list slots"))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Booking>>> {
    let state = state.read().await;

    match state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(bookings) => Json(ApiResponse::success(bookings)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error("SERVER_ERROR", "Failed to list bookings"))
        }
    }
}

async fn create_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateBookingRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.read().await;

    // Check if slot exists and is available
    let slot = match state_guard
        .db
        .get_parking_slot(&req.slot_id.to_string())
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Slot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if slot.status != SlotStatus::Available {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "SLOT_UNAVAILABLE",
                "This slot is not available",
            )),
        );
    }

    // Get or create vehicle info
    let vehicle = match state_guard
        .db
        .get_vehicle(&req.vehicle_id.to_string())
        .await
    {
        Ok(Some(v)) => v,
        _ => Vehicle {
            id: req.vehicle_id,
            user_id: auth_user.user_id,
            license_plate: req.license_plate.clone(),
            make: None,
            model: None,
            color: None,
            vehicle_type: VehicleType::Car,
            is_default: false,
            created_at: Utc::now(),
        },
    };

    // Calculate end time and pricing
    let end_time = req.start_time + Duration::minutes(req.duration_minutes as i64);
    let base_price = (req.duration_minutes as f64 / 60.0) * 2.0; // 2 EUR per hour
    let tax = base_price * 0.1;
    let total = base_price + tax;

    let now = Utc::now();
    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        slot_number: slot.slot_number,
        floor_name: format!("Floor {}", slot.floor_id),
        vehicle,
        start_time: req.start_time,
        end_time,
        status: BookingStatus::Confirmed,
        pricing: BookingPricing {
            base_price,
            discount: 0.0,
            tax,
            total,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: Some(Uuid::new_v4().to_string()),
        notes: req.notes,
    };

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create booking")),
        );
    }

    // Update slot status
    let mut updated_slot = slot;
    updated_slot.status = SlotStatus::Reserved;
    let _ = state_guard.db.save_parking_slot(&updated_slot).await;

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

async fn get_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state = state.read().await;

    match state.db.get_booking(&id).await {
        Ok(Some(booking)) => {
            if booking.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
            (StatusCode::OK, Json(ApiResponse::success(booking)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

async fn cancel_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if booking.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    let mut updated_booking = booking.clone();
    updated_booking.status = BookingStatus::Cancelled;
    updated_booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&updated_booking).await {
        tracing::error!("Failed to update booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel booking",
            )),
        );
    }

    // Free up the slot
    if let Ok(Some(mut slot)) = state_guard
        .db
        .get_parking_slot(&booking.slot_id.to_string())
        .await
    {
        slot.status = SlotStatus::Available;
        let _ = state_guard.db.save_parking_slot(&slot).await;
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// VEHICLES
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_vehicles(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Vehicle>>> {
    let state = state.read().await;

    match state
        .db
        .list_vehicles_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(vehicles) => Json(ApiResponse::success(vehicles)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error("SERVER_ERROR", "Failed to list vehicles"))
        }
    }
}

async fn create_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(mut vehicle): Json<Vehicle>,
) -> (StatusCode, Json<ApiResponse<Vehicle>>) {
    vehicle.user_id = auth_user.user_id;
    vehicle.id = Uuid::new_v4();
    vehicle.created_at = Utc::now();

    let state_guard = state.read().await;
    if let Err(e) = state_guard.db.save_vehicle(&vehicle).await {
        tracing::error!("Failed to save vehicle: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create vehicle")),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(vehicle)))
}

async fn delete_vehicle(
    State(_state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// PASSWORD UTILITIES
// ═══════════════════════════════════════════════════════════════════════════════

fn hash_password(password: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
