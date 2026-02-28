//! HTTP API Routes
//!
//! RESTful API for the parking system.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::email;
use crate::metrics;
use crate::openapi::ApiDoc;
use crate::rate_limit::{EndpointRateLimiters, ip_rate_limit_middleware};
use crate::static_files;

/// Maximum allowed request body size: 1 MiB.
/// Prevents DoS via excessively large JSON payloads.
const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024; // 1 MiB

use parkhub_common::{
    ApiResponse, AuthTokens, Booking, BookingPricing, BookingStatus, CreateBookingRequest,
    HandshakeRequest, HandshakeResponse, LoginRequest, LoginResponse, ParkingLot, ParkingSlot,
    PaymentStatus, RefreshTokenRequest, RegisterRequest, ServerStatus, SlotStatus, User,
    UserPreferences, UserRole, Vehicle, VehicleType, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};

use crate::db::Session;
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// User ID extracted from auth token
#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

/// Create the API router with OpenAPI docs and metrics
pub fn create_router(state: SharedState) -> Router {
    // Initialize Prometheus metrics
    let metrics_handle = metrics::init_metrics();

    // Instantiate per-endpoint rate limiters
    let rate_limiters = EndpointRateLimiters::new();

    // Rate-limited auth routes — each sub-router gets its own per-IP limiter applied
    // via route_layer so only that specific route is affected.

    // POST /api/v1/auth/login — 5 requests per minute per IP
    let login_limiter = rate_limiters.login.clone();
    let login_route = Router::new()
        .route("/api/v1/auth/login", post(login))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(login_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/register — 3 requests per minute per IP
    let register_limiter = rate_limiters.register.clone();
    let register_route = Router::new()
        .route("/api/v1/auth/register", post(register))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(register_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/forgot-password — 3 requests per 15 minutes per IP
    let forgot_limiter = rate_limiters.forgot_password.clone();
    let forgot_route = Router::new()
        .route("/api/v1/auth/forgot-password", post(forgot_password))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(forgot_limiter.clone(), req, next)
        }));

    // Remaining public routes (no rate limiting needed)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_check))
        .route("/health/ready", get(readiness_check))
        .route("/handshake", post(handshake))
        .route("/status", get(server_status))
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/auth/reset-password", post(reset_password))
        // Legal — public (DDG § 5 requires Impressum to be freely accessible)
        .route("/api/v1/legal/impressum", get(get_impressum));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/api/v1/users/me", get(get_current_user))
        .route("/api/v1/users/me/export", get(gdpr_export_data))
        .route("/api/v1/users/me/delete", delete(gdpr_delete_account))
        // Admin-only: retrieve any user by ID
        .route("/api/v1/users/:id", get(get_user))
        .route("/api/v1/lots", get(list_lots).post(create_lot))
        .route("/api/v1/lots/:id", get(get_lot))
        .route("/api/v1/lots/:id/slots", get(get_lot_slots))
        .route("/api/v1/bookings", get(list_bookings).post(create_booking))
        .route(
            "/api/v1/bookings/:id",
            get(get_booking).delete(cancel_booking),
        )
        .route("/api/v1/bookings/:id/invoice", get(get_booking_invoice))
        .route("/api/v1/vehicles", get(list_vehicles).post(create_vehicle))
        .route("/api/v1/vehicles/:id", delete(delete_vehicle))
        // Admin-only: update Impressum settings
        .route("/api/v1/admin/impressum", get(get_impressum_admin).put(update_impressum))
        // Admin-only: user management
        .route("/api/v1/admin/users", get(admin_list_users))
        .route("/api/v1/admin/users/:id/role", axum::routing::patch(admin_update_user_role))
        .route("/api/v1/admin/users/:id/status", axum::routing::patch(admin_update_user_status))
        .route("/api/v1/admin/users/:id", delete(admin_delete_user))
        // Admin-only: all bookings
        .route("/api/v1/admin/bookings", get(admin_list_bookings))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Clone handle for the closure
    let metrics_handle_clone = metrics_handle.clone();

    Router::new()
        .merge(public_routes)
        .merge(login_route)
        .merge(register_route)
        .merge(forgot_route)
        .merge(protected_routes)
        // Prometheus metrics endpoint
        .route("/metrics", get(move || async move {
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                metrics_handle_clone.render(),
            )
        }))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Static files (web frontend) - fallback for all other routes
        .fallback(static_files::static_handler)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        // Security headers applied to every response
        .layer(axum::middleware::from_fn(security_headers_middleware))
        // Restrict request body size to prevent DoS via large payloads
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        // CORS: same-origin by default; no wildcard
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(
                    |origin: &HeaderValue, _req_parts: &axum::http::request::Parts| {
                        // Allow requests with no Origin header (same-origin, curl, mobile)
                        // and explicitly allow localhost origins during development.
                        // In production, the SPA is served from the same origin, so
                        // cross-origin requests are not expected.
                        let s = origin.to_str().unwrap_or("");
                        s.starts_with("http://localhost:")
                            || s.starts_with("https://localhost:")
                            || s.starts_with("http://127.0.0.1:")
                    },
                ))
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
                .allow_credentials(false),
        )
}

/// Middleware that adds security-related response headers to every request.
///
/// - `X-Content-Type-Options`: prevents MIME sniffing
/// - `X-Frame-Options`: prevents clickjacking
/// - `Content-Security-Policy`: restricts resource origins
/// - `Referrer-Policy`: limits referrer leakage
/// - `Permissions-Policy`: disables unneeded browser features
/// - `Strict-Transport-Security`: enforces HTTPS for 1 year including subdomains
async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Safety: all header names and values below are valid ASCII strings.
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            // 'unsafe-inline' removed from script-src — use nonces or hashes for
            // any inline scripts.  style-src retains 'unsafe-inline' because React
            // and similar frameworks inject critical CSS at runtime.
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self'; \
             frame-ancestors 'none'",
        ),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), camera=(), microphone=()"),
    );
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );

    response
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

/// Kubernetes liveness probe - just checks if the service is running
async fn liveness_check() -> StatusCode {
    StatusCode::OK
}

/// Kubernetes readiness probe - checks if the service can handle traffic.
///
/// Returns only a boolean `ready` field. Internal error details are logged
/// server-side but never exposed in the response body.
async fn readiness_check(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    match state.db.stats().await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"ready": true}))),
        Err(e) => {
            tracing::error!("Readiness check failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"ready": false})),
            )
        }
    }
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
            vehicles: 0,
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
    let role_str = format!("{:?}", user.role).to_lowercase();
    let session = Session::new(user.id, 24, &user.username, &role_str); // 24 hour session
    let access_token = Uuid::new_v4().to_string();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create session")),
        );
    }

    // Create response — never send password_hash to clients
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

    // Enforce allow_self_registration setting
    if !state_guard.config.allow_self_registration {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "REGISTRATION_DISABLED",
                "Self-registration is disabled. Contact an administrator.",
            )),
        );
    }

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
    let password_hash = match hash_password(&request.password) {
        Ok(h) => h,
        Err(e) => return e,
    };

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
    let role_str = format!("{:?}", user.role).to_lowercase();
    let session = Session::new(user.id, 24, &user.username, &role_str);
    let access_token = Uuid::new_v4().to_string();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create session")),
        );
    }

    // Create response — never send password_hash to clients
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
    State(state): State<SharedState>,
    Json(request): Json<RefreshTokenRequest>,
) -> (StatusCode, Json<ApiResponse<AuthTokens>>) {
    let state_guard = state.read().await;

    // Look up the session that holds this refresh token
    let (old_access_token, session) =
        match state_guard.db.get_session_by_refresh_token(&request.refresh_token).await {
            Ok(Some(pair)) => pair,
            Ok(None) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error(
                        "INVALID_REFRESH_TOKEN",
                        "Refresh token is invalid or expired",
                    )),
                );
            }
            Err(e) => {
                tracing::error!("Database error during token refresh: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
                );
            }
        };

    // Create a fresh session (7-day expiry)
    let new_session = Session::new(session.user_id, 168, &session.username, &session.role); // 168h = 7 days
    let new_access_token = uuid::Uuid::new_v4().to_string();

    // Save new session
    if let Err(e) = state_guard.db.save_session(&new_access_token, &new_session).await {
        tracing::error!("Failed to save refreshed session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to refresh token")),
        );
    }

    // Invalidate old session
    drop(state_guard);
    let state_guard = state.read().await;
    if let Err(e) = state_guard.db.delete_session(&old_access_token).await {
        tracing::warn!("Failed to delete old session during refresh: {}", e);
    }

    tracing::info!(
        user_id = %session.user_id,
        username = %session.username,
        "Token refreshed successfully"
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(AuthTokens {
            access_token: new_access_token,
            refresh_token: new_session.refresh_token,
            expires_at: new_session.expires_at,
            token_type: "Bearer".to_string(),
        })),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// PASSWORD RESET
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for the forgot-password endpoint
#[derive(Debug, Deserialize)]
struct ForgotPasswordRequest {
    email: String,
}

/// Request body for the reset-password endpoint
#[derive(Debug, Deserialize)]
struct ResetPasswordRequest {
    token: String,
    password: String,
}

/// Stored data for a password-reset token (serialized to JSON in SETTINGS)
#[derive(Debug, Serialize, Deserialize)]
struct PasswordResetToken {
    user_id: String,
    expires_at: chrono::DateTime<Utc>,
}

/// `POST /api/v1/auth/forgot-password`
///
/// Accepts `{"email": "..."}`, generates a one-time reset token (UUID),
/// stores it in the database with a 1-hour expiry, and sends a reset link
/// to the user's email address.  Always returns 200 to prevent user
/// enumeration attacks.
async fn forgot_password(
    State(state): State<SharedState>,
    Json(request): Json<ForgotPasswordRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Look up the user — silently succeed even if not found (anti-enumeration)
    let user = match state_guard.db.get_user_by_email(&request.email).await {
        Ok(Some(u)) => u,
        _ => {
            tracing::info!(
                email = %request.email,
                "Forgot-password request for unknown email — silently accepted"
            );
            return (StatusCode::OK, Json(ApiResponse::success(())));
        }
    };

    // Generate a cryptographically random token
    let reset_token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::hours(1);

    let token_data = PasswordResetToken {
        user_id: user.id.to_string(),
        expires_at,
    };

    let token_json = match serde_json::to_string(&token_data) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to serialize reset token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Store reset token in settings with key "pwreset:<token>"
    let settings_key = format!("pwreset:{}", reset_token);
    if let Err(e) = state_guard.db.set_setting(&settings_key, &token_json).await {
        tracing::error!("Failed to store reset token: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
        );
    }

    // Build and send the reset email (gracefully degraded if SMTP not configured)
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "http://localhost:8443".to_string());
    let reset_url = format!("{}/reset-password?token={}", app_url, reset_token);
    let org_name = state_guard.config.organization_name.clone();

    let html = email::build_password_reset_email(&reset_url, &org_name);

    // Fire-and-forget: email errors are logged but do not fail the request
    if let Err(e) = email::send_email(&user.email, "Reset your password", &html).await {
        tracing::warn!(
            user_id = %user.id,
            error = %e,
            "Failed to send password-reset email"
        );
    }

    tracing::info!(
        user_id = %user.id,
        "Password reset token generated"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

/// `POST /api/v1/auth/reset-password`
///
/// Accepts `{"token": "...", "password": "..."}`, validates the token,
/// updates the user's password, and invalidates the token.
async fn reset_password(
    State(state): State<SharedState>,
    Json(request): Json<ResetPasswordRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Retrieve token data from settings
    let settings_key = format!("pwreset:{}", request.token);
    let token_json = match state_guard.db.get_setting(&settings_key).await {
        Ok(Some(v)) => v,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_TOKEN",
                    "Reset token is invalid or has already been used",
                )),
            );
        }
    };

    let token_data: PasswordResetToken = match serde_json::from_str(&token_json) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to deserialize reset token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Check token expiry
    if token_data.expires_at < Utc::now() {
        // Clean up expired token
        let _ = state_guard.db.set_setting(&settings_key, "").await;
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("TOKEN_EXPIRED", "Reset token has expired")),
        );
    }

    // Validate new password (minimum 8 characters)
    if request.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_PASSWORD",
                "Password must be at least 8 characters long",
            )),
        );
    }

    // Fetch and update the user
    let mut user = match state_guard.db.get_user(&token_data.user_id).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_TOKEN", "User not found")),
            );
        }
    };

    // Hash the new password
    let new_hash = match hash_password_simple(&request.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Password hashing failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    user.password_hash = new_hash;
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save updated user during password reset: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update password")),
        );
    }

    // Invalidate the token by deleting it (write empty string as tombstone)
    // We write "" rather than delete because redb's table API requires an existing
    // key for in-place removal; callers treat an empty value as "not present".
    let _ = state_guard.db.set_setting(&settings_key, "").await;

    tracing::info!(
        user_id = %user.id,
        "Password reset successfully"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
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

/// Retrieve a user by ID.
///
/// Restricted to Admin and SuperAdmin roles. Regular users must use
/// `GET /api/v1/users/me` to access their own profile.
async fn get_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    // Verify caller is an admin before exposing arbitrary user records.
    let caller = match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

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
    // Use a WRITE lock for the entire booking creation to prevent race
    // conditions where two concurrent requests book the same slot simultaneously.
    // Both would read SlotStatus::Available, and both would succeed — leaving the
    // slot double-booked. Holding the write lock ensures only one request can
    // complete the check-and-update atomically.
    let state_guard = state.write().await;

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
        Ok(Some(v)) => {
            // Verify the vehicle belongs to the authenticated user.
            if v.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Vehicle does not belong to you")),
                );
            }
            v
        }
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

    // Validate duration is positive before arithmetic
    if req.duration_minutes <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "Duration must be positive")),
        );
    }

    // Validate start_time is in the future (at least 1 minute from now)
    if req.start_time <= Utc::now() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_BOOKING_TIME",
                "Booking start time must be in the future",
            )),
        );
    }

    // Calculate end time and pricing
    let end_time = req.start_time + Duration::minutes(req.duration_minutes as i64);
    let base_price = (req.duration_minutes as f64 / 60.0) * 2.0; // 2 EUR per hour
    let tax = base_price * 0.1;
    let total = base_price + tax;

    // Look up human-readable floor name from the lot's floors list
    let floor_name = if let Ok(Some(lot)) = state_guard
        .db
        .get_parking_lot(&req.lot_id.to_string())
        .await
    {
        lot.floors
            .iter()
            .find(|f| f.id == slot.floor_id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "Level 1".to_string())
    } else {
        "Level 1".to_string()
    };

    let now = Utc::now();
    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        slot_number: slot.slot_number,
        floor_name,
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

    // Update slot status atomically within the same write-lock scope.
    // The slot status is a critical cache of availability — if we cannot mark it
    // Reserved the slot will appear available and can be double-booked.
    let mut updated_slot = slot;
    updated_slot.status = SlotStatus::Reserved;
    if let Err(e) = state_guard.db.save_parking_slot(&updated_slot).await {
        tracing::error!("Failed to update slot status after booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SLOT_UPDATE_FAILED",
                "Booking created but slot status could not be updated. Please contact support.",
            )),
        );
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %booking.id,
        slot_id = %booking.slot_id,
        "Booking created"
    );

    // Send booking confirmation email (non-blocking, fire-and-forget).
    // TODO: Implement crate::email::send_booking_confirmation(config, email, name, booking)
    // when a dedicated booking confirmation template is available.  For now we use the
    // generic send_email helper with a minimal body so the wiring is in place.
    {
        let user_email_opt = state_guard
            .db
            .get_user(&auth_user.user_id.to_string())
            .await
            .ok()
            .flatten()
            .map(|u| (u.email, u.name));

        if let Some((user_email, user_name)) = user_email_opt {
            let booking_id_str = booking.id.to_string();
            tokio::spawn(async move {
                let subject = format!("Booking confirmation — {}", booking_id_str);
                let html = format!(
                    "<p>Dear {},</p><p>Your booking <strong>{}</strong> has been confirmed.</p>",
                    user_name, booking_id_str
                );
                if let Err(e) = crate::email::send_email(&user_email, &subject, &html).await {
                    tracing::warn!("Failed to send booking confirmation email: {}", e);
                }
            });
        }
    }

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
    // Use write lock so the booking status update and slot status update are
    // made while no other booking creation can interleave.
    let state_guard = state.write().await;

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

    // Only Confirmed or Pending bookings can be cancelled.
    if booking.status == BookingStatus::Cancelled {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error("ALREADY_CANCELLED", "Booking is already cancelled")),
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

    // Free up the slot — only restore to Available if it was Reserved.
    // Slots in Maintenance or Disabled state must remain as-is.
    if let Ok(Some(mut slot)) = state_guard
        .db
        .get_parking_slot(&booking.slot_id.to_string())
        .await
    {
        if slot.status == SlotStatus::Reserved {
            slot.status = SlotStatus::Available;
            if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
                tracing::error!("Failed to restore slot status after cancellation: {}", e);
            }
        }
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %id,
        "Booking cancelled"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// INVOICE
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/bookings/:id/invoice`
///
/// Returns an HTML invoice for the given booking.  The authenticated user must
/// own the booking (admin users may retrieve any invoice).
///
/// The invoice includes:
/// - Company/organisation name from server config
/// - Booking reference (booking UUID)
/// - User name and email
/// - Parking lot name and slot number
/// - Start / end time and duration
/// - Itemised pricing: base price, VAT at 19% (German standard), total
async fn get_booking_invoice(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_guard = state.read().await;

    // Fetch the booking
    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Booking not found".to_string(),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching booking for invoice: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error".to_string(),
            );
        }
    };

    // Ownership check — only the booking owner (or admin) may fetch the invoice
    let caller = match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Access denied".to_string(),
            );
        }
    };

    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Access denied".to_string(),
        );
    }

    // Fetch user details for the invoice
    let booking_user = match state_guard.db.get_user(&booking.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => caller.clone(),
    };

    // Fetch parking lot name
    let lot_name = match state_guard.db.get_parking_lot(&booking.lot_id.to_string()).await {
        Ok(Some(lot)) => lot.name,
        _ => "Unknown Parking Lot".to_string(),
    };

    let org_name = state_guard.config.organization_name.clone();
    let company = if org_name.is_empty() { "ParkHub".to_string() } else { org_name };

    // Calculate duration in minutes
    let duration_minutes = (booking.end_time - booking.start_time).num_minutes();
    let duration_hours = duration_minutes / 60;
    let duration_mins_part = duration_minutes % 60;

    // VAT breakdown (19% German standard — Umsatzsteuergesetz § 12 Abs. 1)
    // The stored `tax` field uses 10% (from create_booking); for the invoice we
    // display the correct 19% MwSt. breakdown on the net price.
    let net_price = booking.pricing.base_price;
    let vat_rate = 0.19_f64;
    let vat_amount = net_price * vat_rate;
    let gross_total = net_price + vat_amount;

    let invoice_date = booking.created_at.format("%d.%m.%Y").to_string();
    let start_str = booking.start_time.format("%d.%m.%Y %H:%M").to_string();
    let end_str = booking.end_time.format("%d.%m.%Y %H:%M").to_string();

    let invoice_number = format!("INV-{}", booking.id.to_string().to_uppercase().replace('-', "").chars().take(12).collect::<String>());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Rechnung {invoice_number}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{ font-family: 'Helvetica Neue', Arial, sans-serif; color: #1a1a2e; background: #f8f9fa; }}
    .page {{ max-width: 800px; margin: 40px auto; background: #ffffff; padding: 60px;
             box-shadow: 0 4px 20px rgba(0,0,0,0.08); border-radius: 4px; }}
    .header {{ display: flex; justify-content: space-between; align-items: flex-start;
               border-bottom: 3px solid #1a73e8; padding-bottom: 24px; margin-bottom: 40px; }}
    .company-name {{ font-size: 28px; font-weight: 700; color: #1a73e8; }}
    .company-sub {{ font-size: 12px; color: #666; margin-top: 4px; }}
    .invoice-meta {{ text-align: right; }}
    .invoice-meta h2 {{ font-size: 22px; color: #333; }}
    .invoice-meta p {{ font-size: 13px; color: #666; margin-top: 4px; }}
    .section {{ margin-bottom: 32px; }}
    .section-title {{ font-size: 11px; font-weight: 700; color: #999; text-transform: uppercase;
                      letter-spacing: 0.1em; margin-bottom: 8px; }}
    .bill-to {{ background: #f8f9fa; padding: 16px 20px; border-radius: 4px; border-left: 3px solid #1a73e8; }}
    .bill-to p {{ font-size: 14px; line-height: 1.6; color: #333; }}
    table {{ width: 100%; border-collapse: collapse; margin-bottom: 0; }}
    thead tr {{ background: #1a73e8; color: white; }}
    thead th {{ padding: 12px 16px; text-align: left; font-size: 13px; font-weight: 600; }}
    tbody tr {{ border-bottom: 1px solid #e8ecf0; }}
    tbody tr:hover {{ background: #f8f9fa; }}
    tbody td {{ padding: 14px 16px; font-size: 14px; color: #333; }}
    .text-right {{ text-align: right; }}
    .totals {{ margin-top: 0; border-top: 2px solid #e8ecf0; }}
    .totals tr td {{ padding: 10px 16px; font-size: 14px; }}
    .totals .total-row td {{ font-size: 16px; font-weight: 700; color: #1a73e8;
                              border-top: 2px solid #1a73e8; padding-top: 14px; }}
    .badge {{ display: inline-block; padding: 4px 10px; border-radius: 20px; font-size: 12px;
              font-weight: 600; }}
    .badge-confirmed {{ background: #e8f5e9; color: #2e7d32; }}
    .footer {{ margin-top: 48px; padding-top: 24px; border-top: 1px solid #e8ecf0;
               font-size: 11px; color: #999; text-align: center; line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="page">

    <!-- Header -->
    <div class="header">
      <div>
        <div class="company-name">{company}</div>
        <div class="company-sub">Parkverwaltungssystem</div>
      </div>
      <div class="invoice-meta">
        <h2>RECHNUNG</h2>
        <p><strong>{invoice_number}</strong></p>
        <p>Datum: {invoice_date}</p>
      </div>
    </div>

    <!-- Bill To -->
    <div class="section">
      <div class="section-title">Rechnungsempfänger</div>
      <div class="bill-to">
        <p><strong>{user_name}</strong></p>
        <p>{user_email}</p>
      </div>
    </div>

    <!-- Booking Details -->
    <div class="section">
      <div class="section-title">Buchungsdetails</div>
      <table>
        <thead>
          <tr>
            <th>Beschreibung</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Buchungsnummer</td>
            <td>{booking_id}</td>
          </tr>
          <tr>
            <td>Parkhaus</td>
            <td>{lot_name}</td>
          </tr>
          <tr>
            <td>Stellplatz</td>
            <td>Nr. {slot_number} &nbsp;·&nbsp; {floor_name}</td>
          </tr>
          <tr>
            <td>Fahrzeug (Kennzeichen)</td>
            <td>{license_plate}</td>
          </tr>
          <tr>
            <td>Beginn</td>
            <td>{start_str}</td>
          </tr>
          <tr>
            <td>Ende</td>
            <td>{end_str}</td>
          </tr>
          <tr>
            <td>Dauer</td>
            <td>{duration_hours} Std. {duration_mins_part} Min.</td>
          </tr>
          <tr>
            <td>Status</td>
            <td><span class="badge badge-confirmed">{status}</span></td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pricing -->
    <div class="section">
      <div class="section-title">Rechnungsbetrag</div>
      <table>
        <thead>
          <tr>
            <th>Position</th>
            <th class="text-right">Betrag ({currency})</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Parkgebühr (Netto)</td>
            <td class="text-right">{net_price:.2}</td>
          </tr>
        </tbody>
        <tbody class="totals">
          <tr>
            <td>Zwischensumme (Netto)</td>
            <td class="text-right">{net_price:.2}</td>
          </tr>
          <tr>
            <td>MwSt. 19% (§ 12 UStG)</td>
            <td class="text-right">{vat_amount:.2}</td>
          </tr>
          <tr class="total-row">
            <td>Gesamtbetrag (Brutto)</td>
            <td class="text-right">{gross_total:.2}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Footer -->
    <div class="footer">
      <p>{company} · Parkverwaltungssystem · Automatisch generierte Rechnung</p>
      <p>Diese Rechnung wurde automatisch erstellt und ist ohne Unterschrift gültig.</p>
    </div>

  </div>
</body>
</html>"#,
        invoice_number = invoice_number,
        invoice_date = invoice_date,
        company = company,
        user_name = booking_user.name,
        user_email = booking_user.email,
        booking_id = booking.id,
        lot_name = lot_name,
        slot_number = booking.slot_number,
        floor_name = booking.floor_name,
        license_plate = booking.vehicle.license_plate,
        start_str = start_str,
        end_str = end_str,
        duration_hours = duration_hours,
        duration_mins_part = duration_mins_part,
        status = format!("{:?}", booking.status),
        currency = booking.pricing.currency,
        net_price = net_price,
        vat_amount = vat_amount,
        gross_total = gross_total,
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
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

/// Delete a vehicle owned by the authenticated user.
///
/// Only the vehicle's owner may delete it. Returns 404 if the vehicle does not
/// exist or 403 if it belongs to another user.
async fn delete_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Fetch the vehicle first to verify ownership.
    let vehicle = match state_guard.db.get_vehicle(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching vehicle: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Ownership check — prevent users from deleting other users' vehicles.
    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    match state_guard.db.delete_vehicle(&id).await {
        Ok(true) => {
            tracing::info!(
                user_id = %auth_user.user_id,
                vehicle_id = %id,
                "Vehicle deleted"
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete vehicle {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to delete vehicle")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PASSWORD UTILITIES
// ═══════════════════════════════════════════════════════════════════════════════

/// Hash a password using Argon2id.
///
/// Returns `Err` on the (extremely unlikely) event that hashing fails so the
/// caller can propagate a proper HTTP 500 instead of panicking.
fn hash_password(password: &str) -> Result<String, (StatusCode, Json<ApiResponse<LoginResponse>>)> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| {
            tracing::error!("Password hashing failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        })
}

/// Hash a password using Argon2id, returning an `anyhow::Result`.
///
/// Used by code paths (e.g. password reset) that cannot return the typed
/// HTTP error tuple used by `hash_password`.
fn hash_password_simple(password: &str) -> anyhow::Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("Argon2 hashing failed: {}", e))
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

// ═══════════════════════════════════════════════════════════════════════════════
// LEGAL / IMPRESSUM (DDG § 5)
// ═══════════════════════════════════════════════════════════════════════════════

/// DDG § 5 Impressum fields stored as settings keys with "impressum_" prefix
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ImpressumData {
    pub provider_name: String,
    pub provider_legal_form: String,
    pub street: String,
    pub zip_city: String,
    pub country: String,
    pub email: String,
    pub phone: String,
    pub register_court: String,
    pub register_number: String,
    pub vat_id: String,
    pub responsible_person: String,
    pub custom_text: String,
}

const IMPRESSUM_FIELDS: &[&str] = &[
    "provider_name", "provider_legal_form", "street", "zip_city", "country",
    "email", "phone", "register_court", "register_number", "vat_id",
    "responsible_person", "custom_text",
];

/// Public Impressum endpoint — no auth required (DDG § 5)
async fn get_impressum(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    let mut data = serde_json::json!({});

    for field in IMPRESSUM_FIELDS {
        let key = format!("impressum_{}", field);
        let value = state.db.get_setting(&key).await.unwrap_or(None).unwrap_or_default();
        data[field] = serde_json::Value::String(value);
    }

    Json(data)
}

/// Admin: read Impressum settings (admin-only, protected).
///
/// Although the public endpoint exposes the same data, this route is kept
/// separate so admins can fetch the current values before editing them via PUT.
/// It is deliberately restricted to Admin/SuperAdmin.
async fn get_impressum_admin(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let state_guard = state.read().await;

    // Verify admin role.
    let caller = match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
            );
        }
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
        );
    }

    let mut data = serde_json::json!({});
    for field in IMPRESSUM_FIELDS {
        let key = format!("impressum_{}", field);
        let value = state_guard.db.get_setting(&key).await.unwrap_or(None).unwrap_or_default();
        data[field] = serde_json::Value::String(value);
    }

    (StatusCode::OK, Json(data))
}

/// Admin: update Impressum settings
async fn update_impressum(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Verify admin role
    let user_id_str = auth_user.user_id.to_string();
    let state_guard = state.read().await;
    let user = match state_guard.db.get_user(&user_id_str).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::FORBIDDEN, Json(ApiResponse::error("FORBIDDEN", "Admin required"))),
    };
    drop(state_guard);

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (StatusCode::FORBIDDEN, Json(ApiResponse::error("FORBIDDEN", "Admin required")));
    }

    let state_guard = state.read().await;
    for field in IMPRESSUM_FIELDS {
        if let Some(serde_json::Value::String(value)) = payload.get(*field) {
            let key = format!("impressum_{}", field);
            let _ = state_guard.db.set_setting(&key, value).await;
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// GDPR — Art. 15 (Data Export) + Art. 17 (Right to Erasure)
// ═══════════════════════════════════════════════════════════════════════════════

/// GDPR Art. 15 — Export all personal data for the authenticated user
async fn gdpr_export_data(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state = state.read().await;
    let user_id = auth_user.user_id.to_string();

    let user = match state.db.get_user(&user_id).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                serde_json::to_string(&ApiResponse::<()>::error("NOT_FOUND", "User not found"))
                    .unwrap_or_default(),
            );
        }
    };

    let bookings = state.db.list_bookings_by_user(&user_id).await.unwrap_or_default();
    let vehicles = state.db.list_vehicles_by_user(&user_id).await.unwrap_or_default();

    // Note: password_hash is intentionally excluded from GDPR exports.
    // Exporting a password hash would allow offline brute-force attacks
    // against the user's own credential — contrary to the spirit of Art. 15.
    let export = serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "gdpr_basis": "GDPR Art. 15 — Right of Access",
        "profile": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "name": user.name,
            "phone": user.phone,
            "role": user.role,
            "created_at": user.created_at,
            "last_login": user.last_login,
            "preferences": user.preferences,
        },
        "bookings": bookings,
        "vehicles": vehicles,
    });

    let json_str = serde_json::to_string_pretty(&export).unwrap_or_default();

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
        ],
        json_str,
    )
}

/// GDPR Art. 17 — Right to Erasure: anonymize user data, keep booking records for accounting.
/// Removes PII (name, email, username, password, vehicles) while preserving anonymized booking
/// records as required by German tax law (§ 147 AO — 10-year retention for accounting records).
async fn gdpr_delete_account(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let user_id = auth_user.user_id.to_string();
    let state_guard = state.read().await;

    match state_guard.db.anonymize_user(&user_id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("GDPR anonymization failed for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to anonymize account")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN — USER MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for updating a user's role
#[derive(Debug, Deserialize)]
struct UpdateUserRoleRequest {
    role: String,
}

/// Request body for updating a user's status
#[derive(Debug, Deserialize)]
struct UpdateUserStatusRequest {
    status: String,
}

/// Response type for admin user listing (includes status field)
#[derive(Debug, Serialize)]
struct AdminUserResponse {
    id: String,
    username: String,
    email: String,
    name: String,
    role: String,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

impl From<&User> for AdminUserResponse {
    fn from(u: &User) -> Self {
        Self {
            id: u.id.to_string(),
            username: u.username.clone(),
            email: u.email.clone(),
            name: u.name.clone(),
            role: format!("{:?}", u.role).to_lowercase(),
            status: if u.is_active { "active".to_string() } else { "disabled".to_string() },
            created_at: u.created_at,
        }
    }
}

/// Helper: verify the caller is an admin or superadmin.
/// Returns `Ok(())` on success, `Err(forbidden_response)` otherwise.
async fn check_admin(
    state: &crate::AppState,
    auth_user: &AuthUser,
) -> Result<(), (StatusCode, &'static str)> {
    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => Ok(()),
        _ => Err((StatusCode::FORBIDDEN, "Admin access required")),
    }
}

/// `GET /api/v1/admin/users` — list all users (admin only)
async fn admin_list_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminUserResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_users().await {
        Ok(users) => {
            let response: Vec<AdminUserResponse> = users.iter().map(AdminUserResponse::from).collect();
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list users")),
            )
        }
    }
}

/// `PATCH /api/v1/admin/users/:id/role` — update a user's role (admin only)
async fn admin_update_user_role(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut user = match state_guard.db.get_user(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
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

    // Parse role string
    user.role = match req.role.as_str() {
        "admin" => UserRole::Admin,
        "superadmin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    };
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user role: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    tracing::info!(
        admin_id = %auth_user.user_id,
        target_user_id = %id,
        new_role = %req.role,
        "Admin updated user role"
    );

    (StatusCode::OK, Json(ApiResponse::success(AdminUserResponse::from(&user))))
}

/// `PATCH /api/v1/admin/users/:id/status` — enable or disable a user account (admin only)
async fn admin_update_user_status(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut user = match state_guard.db.get_user(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
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

    user.is_active = req.status == "active";
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user status: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    tracing::info!(
        admin_id = %auth_user.user_id,
        target_user_id = %id,
        new_status = %req.status,
        "Admin updated user status"
    );

    (StatusCode::OK, Json(ApiResponse::success(AdminUserResponse::from(&user))))
}

/// `DELETE /api/v1/admin/users/:id` — delete a user account (admin only, GDPR anonymize)
async fn admin_delete_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Prevent admin from deleting their own account via admin panel
    if id == auth_user.user_id.to_string() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("CANNOT_DELETE_SELF", "You cannot delete your own account")),
        );
    }

    match state_guard.db.anonymize_user(&id).await {
        Ok(true) => {
            tracing::info!(
                admin_id = %auth_user.user_id,
                target_user_id = %id,
                "Admin anonymized user"
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to anonymize user {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to delete user")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN — BOOKING MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Response type for admin booking listing (includes user details)
#[derive(Debug, Serialize)]
struct AdminBookingResponse {
    id: String,
    user_id: String,
    user_name: String,
    user_email: String,
    lot_id: String,
    lot_name: String,
    slot_id: String,
    slot_number: String,
    vehicle_plate: String,
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

/// `GET /api/v1/admin/bookings` — list all bookings (admin only)
async fn admin_list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminBookingResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list bookings")),
            );
        }
    };

    // Build a response enriched with user info (best-effort: fall back to IDs if user not found)
    let mut response = Vec::with_capacity(bookings.len());
    for booking in bookings {
        let (user_name, user_email) = match state_guard.db.get_user(&booking.user_id.to_string()).await {
            Ok(Some(u)) => (u.name, u.email),
            _ => (booking.user_id.to_string(), String::new()),
        };

        let lot_name = match state_guard.db.get_parking_lot(&booking.lot_id.to_string()).await {
            Ok(Some(l)) => l.name,
            _ => booking.lot_id.to_string(),
        };

        response.push(AdminBookingResponse {
            id: booking.id.to_string(),
            user_id: booking.user_id.to_string(),
            user_name,
            user_email,
            lot_id: booking.lot_id.to_string(),
            lot_name,
            slot_id: booking.slot_id.to_string(),
            slot_number: booking.slot_number.to_string(),
            vehicle_plate: booking.vehicle.license_plate.clone(),
            start_time: booking.start_time,
            end_time: booking.end_time,
            status: format!("{:?}", booking.status).to_lowercase(),
            created_at: booking.created_at,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(response)))
}
