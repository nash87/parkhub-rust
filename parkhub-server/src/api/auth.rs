//! Authentication handlers: login, register, token refresh, password management.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::{
    ApiResponse, AuthTokens, LoginRequest, LoginResponse, RefreshTokenRequest, RegisterRequest,
    User, UserPreferences, UserRole,
};

use crate::audit::{AuditEntry, AuditEventType};
use crate::db::Session;
#[cfg(feature = "mod-email")]
use crate::email;
use crate::metrics;

use super::{
    generate_access_token, hash_password, hash_password_simple, verify_password, SharedState,
};

// ─────────────────────────────────────────────────────────────────────────────
// Cookie helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Cookie name for the auth token.
pub const AUTH_COOKIE_NAME: &str = "parkhub_token";

/// Build a `Set-Cookie` header value for the auth token.
///
/// The cookie is `HttpOnly`, `SameSite=Lax`, `Path=/`, and `Secure` unless
/// running on localhost (detected via `APP_URL` env var).
pub(super) fn build_auth_cookie(token: &str, max_age_secs: i64) -> String {
    let secure_flag = std::env::var("APP_URL")
        .map(|u| !u.starts_with("http://localhost") && !u.starts_with("http://127.0.0.1"))
        .unwrap_or(false);

    let mut cookie = format!(
        "{AUTH_COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_secs}"
    );
    if secure_flag {
        cookie.push_str("; Secure");
    }
    cookie
}

/// Build a `Set-Cookie` header value that clears (expires) the auth cookie.
fn build_clear_auth_cookie() -> String {
    format!("{AUTH_COOKIE_NAME}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")
}

/// Attach a `Set-Cookie` header to an existing `(StatusCode, Json<...>)` response.
fn with_auth_cookie<T: serde::Serialize>(
    status: StatusCode,
    body: Json<T>,
    cookie_value: &str,
) -> Response {
    let mut resp = (status, body).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(cookie_value) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    resp
}

// ─────────────────────────────────────────────────────────────────────────────
// Request types
// ─────────────────────────────────────────────────────────────────────────────

/// Request body for the forgot-password endpoint
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ForgotPasswordRequest {
    email: String,
}

/// Request body for the reset-password endpoint
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ResetPasswordRequest {
    token: String,
    password: String,
}

/// Stored data for a password-reset token (serialized to JSON in SETTINGS)
#[derive(Debug, serde::Serialize, Deserialize)]
struct PasswordResetToken {
    user_id: String,
    expires_at: chrono::DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    summary = "Log in",
    description = "Authenticate with username/email and password. Returns access and refresh tokens.",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful"),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Account disabled"),
    )
)]
#[tracing::instrument(skip(state, request), fields(username = %request.username))]
pub async fn login(
    State(state): State<SharedState>,
    Json(request): Json<LoginRequest>,
) -> Response {
    // ── Input length validation (issue #115) ────────────────────────────────
    if request.username.len() > 254 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "INVALID_INPUT",
                "Username/email must be at most 254 characters",
            )),
        )
            .into_response();
    }

    let state_guard = state.read().await;

    // Find user by username
    let user = match state_guard.db.get_user_by_username(&request.username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Also try by email
            if let Ok(Some(u)) = state_guard.db.get_user_by_email(&request.username).await {
                u
            } else {
                AuditEntry::new(AuditEventType::LoginFailed)
                    .error("User not found")
                    .log();
                metrics::record_auth_event("login", false);
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::<LoginResponse>::error(
                        "INVALID_CREDENTIALS",
                        "Invalid username or password",
                    )),
                )
                    .into_response();
            }
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<LoginResponse>::error(
                    "SERVER_ERROR",
                    "Internal server error",
                )),
            )
                .into_response();
        }
    };

    // Reject excessively long passwords before hashing (Argon2 CPU DoS prevention)
    if request.password.len() > 256 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "INVALID_INPUT",
                "Password must not exceed 256 characters",
            )),
        )
            .into_response();
    }

    // Verify password
    if !verify_password(&request.password, &user.password_hash).await {
        AuditEntry::new(AuditEventType::LoginFailed)
            .user(user.id, &user.username)
            .error("Invalid password")
            .log();
        metrics::record_auth_event("login", false);
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<LoginResponse>::error(
                "INVALID_CREDENTIALS",
                "Invalid username or password",
            )),
        )
            .into_response();
    }

    // Check if user is active
    if !user.is_active {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<LoginResponse>::error(
                "ACCOUNT_DISABLED",
                "This account has been disabled",
            )),
        )
            .into_response();
    }

    // Create session using configured timeout (converted from minutes to hours, minimum 1h)
    let session_hours = i64::from(state_guard.config.session_timeout_minutes).max(60) / 60;
    let role_str = format!("{:?}", user.role).to_lowercase();
    let session = Session::new(user.id, session_hours, &user.username, &role_str);
    let access_token = generate_access_token();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<LoginResponse>::error(
                "SERVER_ERROR",
                "Failed to create session",
            )),
        )
            .into_response();
    }

    let audit = AuditEntry::new(AuditEventType::LoginSuccess)
        .user(user.id, &user.username)
        .log();
    audit.persist(&state_guard.db).await;
    drop(state_guard);
    metrics::record_auth_event("login", true);

    // Create response — never send password_hash to clients
    let mut response_user = user;
    response_user.password_hash = String::new();

    // Cookie max-age: session_hours converted to seconds
    let cookie_max_age = session_hours * 3600;
    let cookie = build_auth_cookie(&access_token, cookie_max_age);

    with_auth_cookie(
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
        &cookie,
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Authentication",
    summary = "Register a new account",
    description = "Create a new user account. May be disabled via admin settings.",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Registration successful"),
        (status = 400, description = "Invalid input"),
        (status = 403, description = "Registration disabled"),
        (status = 409, description = "Email already exists"),
    )
)]
#[tracing::instrument(skip(state, request), fields(email = %request.email))]
#[allow(clippy::too_many_lines)]
pub async fn register(
    State(state): State<SharedState>,
    Json(request): Json<RegisterRequest>,
) -> Response {
    // ── Input length validation (issue #115) ────────────────────────────────
    if request.email.len() > 254 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "INVALID_INPUT",
                "Email must be at most 254 characters",
            )),
        )
            .into_response();
    }
    if request.name.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "INVALID_INPUT",
                "Name must be at most 100 characters",
            )),
        )
            .into_response();
    }

    let state_guard = state.read().await;

    // Enforce allow_self_registration setting
    if !state_guard.config.allow_self_registration {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<LoginResponse>::error(
                "REGISTRATION_DISABLED",
                "Self-registration is disabled. Contact an administrator.",
            )),
        )
            .into_response();
    }

    // Password confirmation must match
    if request.password != request.password_confirmation {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "PASSWORD_MISMATCH",
                "Password and confirmation do not match",
            )),
        )
            .into_response();
    }

    // Password complexity: min 8 chars, at least one lowercase, uppercase, digit
    let pw = &request.password;
    if pw.len() < 8
        || !pw.chars().any(|c| c.is_ascii_lowercase())
        || !pw.chars().any(|c| c.is_ascii_uppercase())
        || !pw.chars().any(|c| c.is_ascii_digit())
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "WEAK_PASSWORD",
                "Password must be at least 8 characters with uppercase, lowercase, and a digit",
            )),
        )
            .into_response();
    }

    // Check if email already exists
    if let Ok(Some(_)) = state_guard.db.get_user_by_email(&request.email).await {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<LoginResponse>::error(
                "EMAIL_EXISTS",
                "An account with this email already exists",
            )),
        )
            .into_response();
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
        final_username = format!("{username}{counter}");
        counter += 1;
    }

    // Reject excessively long passwords before hashing (Argon2 CPU DoS prevention)
    if request.password.len() > 256 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<LoginResponse>::error(
                "INVALID_INPUT",
                "Password must not exceed 256 characters",
            )),
        )
            .into_response();
    }

    // Hash password
    let password_hash = match hash_password(&request.password).await {
        Ok(h) => h,
        Err(e) => return e.into_response(),
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
        credits_balance: 40,
        credits_monthly_quota: 40,
        credits_last_refilled: Some(now),
        tenant_id: None,
        accessibility_needs: None,
        cost_center: None,
        department: None,
    };

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<LoginResponse>::error(
                "SERVER_ERROR",
                "Failed to create account",
            )),
        )
            .into_response();
    }

    let audit = AuditEntry::new(AuditEventType::UserCreated)
        .user(user.id, &user.username)
        .log();
    audit.persist(&state_guard.db).await;
    metrics::record_auth_event("register", true);

    // Dispatch webhook for user creation
    #[cfg(feature = "mod-webhooks")]
    {
        let state_clone = state.clone();
        let payload = serde_json::json!({
            "user_id": user.id,
            "username": user.username,
        });
        tokio::spawn(async move {
            crate::api::webhooks::dispatch_webhook_event(&state_clone, "user.created", payload)
                .await;
        });
    }

    // Send welcome email (async, best-effort — failures are logged, not propagated)
    #[cfg(feature = "mod-email")]
    {
        let user_email = user.email.clone();
        let user_name = user.name.clone();
        let org_name = state_guard.config.organization_name.clone();
        tokio::spawn(async move {
            let email_html = crate::email::build_welcome_email(&user_name, &org_name);
            if let Err(e) = crate::email::send_email(
                &user_email,
                &format!("Welcome to {org_name}"),
                &email_html,
            )
            .await
            {
                tracing::warn!("Failed to send welcome email: {}", e);
            }
        });
    }

    // Create session using configured timeout (converted from minutes to hours, minimum 1h)
    let session_hours = i64::from(state_guard.config.session_timeout_minutes).max(60) / 60;
    let role_str = format!("{:?}", user.role).to_lowercase();
    let session = Session::new(user.id, session_hours, &user.username, &role_str);
    let access_token = generate_access_token();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<LoginResponse>::error(
                "SERVER_ERROR",
                "Failed to create session",
            )),
        )
            .into_response();
    }
    drop(state_guard);

    // Create response — never send password_hash to clients
    let mut response_user = user;
    response_user.password_hash = String::new();

    // Cookie max-age: session_hours converted to seconds
    let cookie_max_age = session_hours * 3600;
    let cookie = build_auth_cookie(&access_token, cookie_max_age);

    with_auth_cookie(
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
        &cookie,
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    summary = "Refresh access token",
    description = "Exchange a valid refresh token for a new access/refresh token pair.",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully"),
        (status = 401, description = "Invalid or expired refresh token"),
    )
)]
#[tracing::instrument(skip(state, request))]
pub async fn refresh_token(
    State(state): State<SharedState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Response {
    let state_guard = state.read().await;

    // Look up the session that holds this refresh token
    let (old_access_token, session) = match state_guard
        .db
        .get_session_by_refresh_token(&request.refresh_token)
        .await
    {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<AuthTokens>::error(
                    "INVALID_REFRESH_TOKEN",
                    "Refresh token is invalid or expired",
                )),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error during token refresh: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<AuthTokens>::error(
                    "SERVER_ERROR",
                    "Internal server error",
                )),
            )
                .into_response();
        }
    };

    // Re-query the database to get the current role and verify the user is still active.
    // This prevents stale role claims (issue #55): a role change takes effect on the next
    // refresh rather than being carried forward from the old session indefinitely.
    let current_user = match state_guard.db.get_user(&session.user_id.to_string()).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<AuthTokens>::error(
                    "INVALID_REFRESH_TOKEN",
                    "User account no longer exists",
                )),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error during role re-validation: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<AuthTokens>::error(
                    "SERVER_ERROR",
                    "Internal server error",
                )),
            )
                .into_response();
        }
    };

    if !current_user.is_active {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<AuthTokens>::error(
                "ACCOUNT_DISABLED",
                "This account has been disabled",
            )),
        )
            .into_response();
    }

    let current_role = format!("{:?}", current_user.role).to_lowercase();

    // Create a fresh session using the configured session timeout (minimum 1 h)
    let session_hours = i64::from(state_guard.config.session_timeout_minutes).max(60) / 60;
    let new_session = Session::new(
        session.user_id,
        session_hours,
        &session.username,
        &current_role,
    );
    let new_access_token = generate_access_token();

    // Save new session
    if let Err(e) = state_guard
        .db
        .save_session(&new_access_token, &new_session)
        .await
    {
        tracing::error!("Failed to save refreshed session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<AuthTokens>::error(
                "SERVER_ERROR",
                "Failed to refresh token",
            )),
        )
            .into_response();
    }

    // Invalidate old session
    drop(state_guard);
    {
        let state_guard = state.read().await;
        if let Err(e) = state_guard.db.delete_session(&old_access_token).await {
            tracing::warn!("Failed to delete old session during refresh: {}", e);
        }
    }

    tracing::info!(
        user_id = %session.user_id,
        username = %session.username,
        "Token refreshed successfully"
    );

    // Cookie max-age: session_hours converted to seconds
    let cookie_max_age = session_hours * 3600;
    let cookie = build_auth_cookie(&new_access_token, cookie_max_age);

    with_auth_cookie(
        StatusCode::OK,
        Json(ApiResponse::success(AuthTokens {
            access_token: new_access_token,
            refresh_token: new_session.refresh_token,
            expires_at: new_session.expires_at,
            token_type: "Bearer".to_string(),
        })),
        &cookie,
    )
}

/// `POST /api/v1/auth/forgot-password`
///
/// Accepts `{"email": "..."}`, generates a one-time reset token (UUID),
/// stores it in the database with a 1-hour expiry, and sends a reset link
/// to the user's email address.  Always returns 200 to prevent user
/// enumeration attacks.
#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    tag = "Authentication",
    summary = "Request password reset",
    description = "Send a password reset email. Always returns 200 to prevent user enumeration.",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Reset email sent (always succeeds to prevent enumeration)"),
    )
)]
#[tracing::instrument(skip(state, request), fields(email = %request.email))]
pub async fn forgot_password(
    State(state): State<SharedState>,
    Json(request): Json<ForgotPasswordRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Look up the user — silently succeed even if not found (anti-enumeration)
    let Ok(Some(user)) = state_guard.db.get_user_by_email(&request.email).await else {
        tracing::info!(
            email = %request.email,
            "Forgot-password request for unknown email — silently accepted"
        );
        return (StatusCode::OK, Json(ApiResponse::success(())));
    };

    // Generate a cryptographically random token (32 bytes, hex-encoded)
    let mut token_bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut token_bytes);
    let reset_token = hex::encode(token_bytes);
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
    let settings_key = format!("pwreset:{reset_token}");
    if let Err(e) = state_guard.db.set_setting(&settings_key, &token_json).await {
        tracing::error!("Failed to store reset token: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
        );
    }

    // Build and send the reset email (gracefully degraded if SMTP not configured)
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:8443".to_string());
    let reset_url = format!("{app_url}/reset-password?token={reset_token}");
    let org_name = state_guard.config.organization_name.clone();

    drop(state_guard);

    #[cfg(feature = "mod-email")]
    {
        let html = email::build_password_reset_email(&reset_url, &org_name);

        // Fire-and-forget: email errors are logged but do not fail the request
        if let Err(e) = email::send_email(&user.email, "Reset your password", &html).await {
            tracing::warn!(
                user_id = %user.id,
                error = %e,
                "Failed to send password-reset email"
            );
        }
    }

    #[cfg(not(feature = "mod-email"))]
    {
        let _ = (&reset_url, &org_name);
        tracing::info!(
            user_id = %user.id,
            "Email module disabled — password reset email not sent"
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
#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    tag = "Authentication",
    summary = "Reset password with token",
    description = "Set a new password using a one-time reset token from the forgot-password email.",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful"),
        (status = 400, description = "Invalid or expired token"),
    )
)]
#[tracing::instrument(skip(state, request))]
pub async fn reset_password(
    State(state): State<SharedState>,
    Json(request): Json<ResetPasswordRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Retrieve token data from settings
    let settings_key = format!("pwreset:{}", request.token);
    let Ok(Some(token_json)) = state_guard.db.get_setting(&settings_key).await else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_TOKEN",
                "Reset token is invalid or has already been used",
            )),
        );
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
        if let Err(e) = state_guard.db.set_setting(&settings_key, "").await {
            tracing::warn!("Failed to clean up expired reset token: {e}");
        }
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "TOKEN_EXPIRED",
                "Reset token has expired",
            )),
        );
    }

    // Reject excessively long passwords before hashing (Argon2 CPU DoS prevention)
    if request.password.len() > 256 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Password must not exceed 256 characters",
            )),
        );
    }

    // Validate new password using strong password rules
    if let Err(e) = crate::validation::validate_password_strength(&request.password) {
        let msg = e.message.map_or_else(
            || "Password does not meet strength requirements".to_string(),
            |m| m.to_string(),
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_PASSWORD", msg)),
        );
    }

    // Fetch and update the user
    let Ok(Some(mut user)) = state_guard.db.get_user(&token_data.user_id).await else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_TOKEN", "User not found")),
        );
    };

    // Hash the new password
    let new_hash = match hash_password_simple(&request.password).await {
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
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update password",
            )),
        );
    }

    // Invalidate the token by deleting it (write empty string as tombstone)
    if let Err(e) = state_guard.db.set_setting(&settings_key, "").await {
        tracing::warn!("Failed to invalidate reset token: {e}");
    }

    // Invalidate all existing sessions for this user — a password change must
    // force re-authentication on every device.
    if let Err(e) = state_guard.db.delete_sessions_by_user(user.id).await {
        tracing::warn!(user_id = %user.id, error = %e, "Failed to invalidate sessions after password reset");
    }
    drop(state_guard);

    AuditEntry::new(AuditEventType::PasswordChanged)
        .user(user.id, &user.username)
        .log();

    tracing::info!(
        user_id = %user.id,
        "Password reset successfully"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ─────────────────────────────────────────────────────────────────────────────
// Logout
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/auth/logout`
///
/// Clears the httpOnly auth cookie. If a Bearer token is present in the
/// Authorization header, the corresponding session is also invalidated
/// server-side.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    summary = "Log out",
    description = "Clear the auth cookie and optionally invalidate the server session.",
    responses(
        (status = 200, description = "Logged out successfully"),
    )
)]
pub async fn logout(
    State(state): State<SharedState>,
    request: axum::http::Request<axum::body::Body>,
) -> Response {
    // Try to extract the token from the Authorization header or cookie,
    // then delete the session server-side (best-effort).
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(String::from)
        .or_else(|| extract_cookie_token(request.headers()));

    if let Some(tok) = token {
        let state_guard = state.read().await;
        if let Err(e) = state_guard.db.delete_session(&tok).await {
            tracing::warn!("Failed to delete session during logout: {}", e);
        }
    }

    let cookie = build_clear_auth_cookie();
    with_auth_cookie(
        StatusCode::OK,
        Json(ApiResponse::<()>::success(())),
        &cookie,
    )
}

/// Extract the auth token from the `Cookie` header.
fn extract_cookie_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{AUTH_COOKIE_NAME}="))
                    .map(std::string::ToString::to_string)
            })
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forgot_password_request_deserialize() {
        let json = r#"{"email": "alice@example.com"}"#;
        let req: ForgotPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "alice@example.com");
    }

    #[test]
    fn test_forgot_password_request_missing_email() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<ForgotPasswordRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_password_request_deserialize() {
        let json = r#"{"token": "abc123", "password": "NewP@ss1234"}"#;
        let req: ResetPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "abc123");
        assert_eq!(req.password, "NewP@ss1234");
    }

    #[test]
    fn test_reset_password_request_missing_fields() {
        // Missing password
        let json = r#"{"token": "abc123"}"#;
        assert!(serde_json::from_str::<ResetPasswordRequest>(json).is_err());

        // Missing token
        let json = r#"{"password": "secret"}"#;
        assert!(serde_json::from_str::<ResetPasswordRequest>(json).is_err());

        // Empty object
        let json = r#"{}"#;
        assert!(serde_json::from_str::<ResetPasswordRequest>(json).is_err());
    }

    #[test]
    fn test_password_reset_token_roundtrip() {
        let token = PasswordResetToken {
            user_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let json = serde_json::to_string(&token).unwrap();
        let deserialized: PasswordResetToken = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, token.user_id);
        assert_eq!(deserialized.expires_at, token.expires_at);
    }

    #[test]
    fn test_password_reset_token_expired_check() {
        let expired = PasswordResetToken {
            user_id: "test-user".to_string(),
            expires_at: Utc::now() - chrono::Duration::hours(1),
        };
        assert!(expired.expires_at < Utc::now());

        let valid = PasswordResetToken {
            user_id: "test-user".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };
        assert!(valid.expires_at > Utc::now());
    }

    #[test]
    fn test_password_reset_token_deserialize_from_db_format() {
        // Simulate what gets stored in the settings table
        let json = r#"{"user_id":"abc-123","expires_at":"2026-12-31T23:59:59Z"}"#;
        let token: PasswordResetToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.user_id, "abc-123");
    }

    #[test]
    fn test_forgot_password_request_extra_fields_ignored() {
        let json = r#"{"email": "test@test.com", "unknown_field": true}"#;
        let req: ForgotPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "test@test.com");
    }

    #[test]
    fn test_reset_password_request_extra_fields_ignored() {
        let json = r#"{"token": "t", "password": "p", "extra": 42}"#;
        let req: ResetPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "t");
        assert_eq!(req.password, "p");
    }

    // ── Cookie helpers ──

    #[test]
    fn test_build_auth_cookie_contains_httponly() {
        let cookie = build_auth_cookie("test-token-123", 3600);
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=3600"));
        assert!(cookie.contains("parkhub_token=test-token-123"));
    }

    #[test]
    fn test_build_auth_cookie_no_secure_on_localhost() {
        // APP_URL not set defaults to localhost
        std::env::remove_var("APP_URL");
        let cookie = build_auth_cookie("tok", 7200);
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn test_build_clear_auth_cookie_expires_immediately() {
        let cookie = build_clear_auth_cookie();
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("parkhub_token="));
    }

    #[test]
    fn test_extract_cookie_token_found() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_static("other=x; parkhub_token=abc123; session=y"),
        );
        let result = extract_cookie_token(&headers);
        assert_eq!(result, Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_cookie_token_not_found() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_static("other=x; session=y"),
        );
        let result = extract_cookie_token(&headers);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_cookie_token_no_cookie_header() {
        let headers = axum::http::HeaderMap::new();
        let result = extract_cookie_token(&headers);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_cookie_token_single_cookie() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_static("parkhub_token=single-value"),
        );
        let result = extract_cookie_token(&headers);
        assert_eq!(result, Some("single-value".to_string()));
    }

    #[test]
    fn test_auth_cookie_name_constant() {
        assert_eq!(AUTH_COOKIE_NAME, "parkhub_token");
    }
}
