//! Security features: 2FA/TOTP, password policy, login history, session management, API keys.

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::audit::{AuditEntry, AuditEventType};

use super::{check_admin, generate_access_token, verify_password, AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// 2FA / TOTP
// ═══════════════════════════════════════════════════════════════════════════════

/// Response from 2FA setup — includes secret and provisioning URI for QR code.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TwoFactorSetupResponse {
    /// Base32-encoded TOTP secret
    pub secret: String,
    /// `otpauth://` URI for QR code generation
    pub otpauth_uri: String,
    /// QR code as base64-encoded PNG
    pub qr_code_base64: String,
}

/// Request body for verifying a TOTP code.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TwoFactorVerifyRequest {
    /// The 6-digit TOTP code from the authenticator app
    pub code: String,
}

/// Request body for disabling 2FA.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TwoFactorDisableRequest {
    /// Current password required to disable 2FA
    pub current_password: String,
}

/// Response indicating 2FA status.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
}

/// Login response extension when 2FA is required.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TwoFactorLoginRequest {
    /// Temporary token from initial login
    pub temp_token: String,
    /// The 6-digit TOTP code
    pub code: String,
}

/// `POST /api/v1/auth/2fa/setup` — Generate TOTP secret and QR code for enrollment.
#[utoipa::path(
    post,
    path = "/api/v1/auth/2fa/setup",
    tag = "Authentication",
    summary = "Set up 2FA",
    description = "Generates a TOTP secret and QR code URI for authenticator app enrollment. Does not enable 2FA until verified.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "2FA setup info"),
        (status = 409, description = "2FA already enabled"),
    )
)]
pub async fn two_factor_setup(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<TwoFactorSetupResponse>>) {
    let state_guard = state.read().await;

    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
    };

    // Check if 2FA is already enabled
    let totp_key = format!("totp:{}", user.id);
    if let Ok(Some(val)) = state_guard.db.get_setting(&totp_key).await {
        if !val.is_empty() {
            // Check if it's enabled (not just pending)
            let enabled_key = format!("totp_enabled:{}", user.id);
            if let Ok(Some(e)) = state_guard.db.get_setting(&enabled_key).await {
                if e == "true" {
                    return (
                        StatusCode::CONFLICT,
                        Json(ApiResponse::error(
                            "2FA_ALREADY_ENABLED",
                            "Two-factor authentication is already enabled",
                        )),
                    );
                }
            }
        }
    }

    // Generate TOTP secret
    let totp = match totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        totp_rs::Secret::generate_secret().to_bytes().unwrap(),
        Some("ParkHub".to_string()),
        user.email.clone(),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create TOTP: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to generate 2FA secret")),
            );
        }
    };

    let secret = totp_rs::Secret::Raw(totp.secret.clone()).to_encoded().to_string();
    let otpauth_uri = totp.get_url();

    // Generate QR code as base64 PNG
    let qr_code_base64 = match generate_qr_base64(&otpauth_uri) {
        Ok(b64) => b64,
        Err(e) => {
            tracing::error!("Failed to generate QR code: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to generate QR code")),
            );
        }
    };

    // Store the secret (pending verification)
    if let Err(e) = state_guard.db.set_setting(&totp_key, &secret).await {
        tracing::error!("Failed to store TOTP secret: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(TwoFactorSetupResponse {
            secret,
            otpauth_uri,
            qr_code_base64,
        })),
    )
}

/// `POST /api/v1/auth/2fa/verify` — Verify a TOTP code to enable 2FA.
#[utoipa::path(
    post,
    path = "/api/v1/auth/2fa/verify",
    tag = "Authentication",
    summary = "Verify and enable 2FA",
    description = "Verifies a TOTP code from the authenticator app and enables 2FA for the user.",
    security(("bearer_auth" = [])),
    request_body = TwoFactorVerifyRequest,
    responses(
        (status = 200, description = "2FA enabled"),
        (status = 400, description = "Invalid code"),
        (status = 404, description = "No pending 2FA setup"),
    )
)]
pub async fn two_factor_verify(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<TwoFactorVerifyRequest>,
) -> (StatusCode, Json<ApiResponse<TwoFactorStatusResponse>>) {
    let state_guard = state.read().await;

    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
    };

    // Get pending TOTP secret
    let totp_key = format!("totp:{}", user.id);
    let secret_b32 = match state_guard.db.get_setting(&totp_key).await {
        Ok(Some(s)) if !s.is_empty() => s,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "NO_PENDING_SETUP",
                    "No pending 2FA setup found. Call /2fa/setup first.",
                )),
            );
        }
    };

    // Parse and verify the code
    let secret_bytes = match totp_rs::Secret::Encoded(secret_b32).to_bytes() {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to decode TOTP secret: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let totp = match totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("ParkHub".to_string()),
        user.email.clone(),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create TOTP for verification: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if !totp.check_current(&req.code).unwrap_or(false) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_CODE",
                "The verification code is invalid or expired",
            )),
        );
    }

    // Enable 2FA
    let enabled_key = format!("totp_enabled:{}", user.id);
    if let Err(e) = state_guard.db.set_setting(&enabled_key, "true").await {
        tracing::error!("Failed to enable 2FA: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to enable 2FA")),
        );
    }

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(user.id, &user.username)
        .detail("2FA enabled")
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(TwoFactorStatusResponse { enabled: true })),
    )
}

/// `POST /api/v1/auth/2fa/disable` — Disable 2FA (requires current password).
#[utoipa::path(
    post,
    path = "/api/v1/auth/2fa/disable",
    tag = "Authentication",
    summary = "Disable 2FA",
    description = "Disables two-factor authentication. Requires current password.",
    security(("bearer_auth" = [])),
    request_body = TwoFactorDisableRequest,
    responses(
        (status = 200, description = "2FA disabled"),
        (status = 401, description = "Invalid password"),
    )
)]
pub async fn two_factor_disable(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<TwoFactorDisableRequest>,
) -> (StatusCode, Json<ApiResponse<TwoFactorStatusResponse>>) {
    let state_guard = state.read().await;

    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
    };

    // Verify password
    if !verify_password(&req.current_password, &user.password_hash).await {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error(
                "INVALID_PASSWORD",
                "Current password is incorrect",
            )),
        );
    }

    // Clear TOTP secret and enabled flag
    let totp_key = format!("totp:{}", user.id);
    let enabled_key = format!("totp_enabled:{}", user.id);
    let _ = state_guard.db.set_setting(&totp_key, "").await;
    let _ = state_guard.db.set_setting(&enabled_key, "false").await;

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(user.id, &user.username)
        .detail("2FA disabled")
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(TwoFactorStatusResponse { enabled: false })),
    )
}

/// `GET /api/v1/auth/2fa/status` — Check if 2FA is enabled for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/auth/2fa/status",
    tag = "Authentication",
    summary = "Get 2FA status",
    description = "Returns whether two-factor authentication is enabled for the current user.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "2FA status"),
    )
)]
pub async fn two_factor_status(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<TwoFactorStatusResponse>>) {
    let state_guard = state.read().await;
    let enabled = is_2fa_enabled(&state_guard, auth_user.user_id).await;
    (
        StatusCode::OK,
        Json(ApiResponse::success(TwoFactorStatusResponse { enabled })),
    )
}

/// Check if 2FA is enabled for a user.
pub async fn is_2fa_enabled(state: &crate::AppState, user_id: Uuid) -> bool {
    let key = format!("totp_enabled:{user_id}");
    matches!(state.db.get_setting(&key).await, Ok(Some(v)) if v == "true")
}

/// Verify a TOTP code for a user during login.
pub async fn verify_2fa_code(state: &crate::AppState, user_id: Uuid, email: &str, code: &str) -> bool {
    let totp_key = format!("totp:{user_id}");
    let secret_b32 = match state.db.get_setting(&totp_key).await {
        Ok(Some(s)) if !s.is_empty() => s,
        _ => return false,
    };

    let secret_bytes = match totp_rs::Secret::Encoded(secret_b32).to_bytes() {
        Ok(b) => b,
        Err(_) => return false,
    };

    let totp = match totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("ParkHub".to_string()),
        email.to_string(),
    ) {
        Ok(t) => t,
        Err(_) => return false,
    };

    totp.check_current(code).unwrap_or(false)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PASSWORD POLICY
// ═══════════════════════════════════════════════════════════════════════════════

/// Configurable password policy.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PasswordPolicy {
    pub min_length: u32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_number: bool,
    pub require_special_char: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_number: true,
            require_special_char: false,
        }
    }
}

impl PasswordPolicy {
    /// Check a password against this policy. Returns `Ok(())` or an error message.
    pub fn check(&self, password: &str) -> Result<(), String> {
        if (password.len() as u32) < self.min_length {
            return Err(format!(
                "Password must be at least {} characters",
                self.min_length
            ));
        }
        if self.require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
            return Err("Password must contain at least one uppercase letter".to_string());
        }
        if self.require_lowercase && !password.chars().any(|c| c.is_ascii_lowercase()) {
            return Err("Password must contain at least one lowercase letter".to_string());
        }
        if self.require_number && !password.chars().any(|c| c.is_ascii_digit()) {
            return Err("Password must contain at least one digit".to_string());
        }
        if self.require_special_char
            && !password
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && c.is_ascii())
        {
            return Err("Password must contain at least one special character".to_string());
        }
        Ok(())
    }
}

/// Load password policy from DB settings.
pub async fn load_password_policy(db: &crate::db::Database) -> PasswordPolicy {
    let mut policy = PasswordPolicy::default();
    if let Ok(Some(val)) = db.get_setting("password_policy").await {
        if let Ok(p) = serde_json::from_str::<PasswordPolicy>(&val) {
            policy = p;
        }
    }
    policy
}

/// Check a password against the stored password policy.
pub async fn check_password_policy(db: &crate::db::Database, password: &str) -> Result<(), String> {
    load_password_policy(db).await.check(password)
}

/// `GET /api/v1/admin/settings/password-policy` — Get current password policy.
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/password-policy",
    tag = "Admin",
    summary = "Get password policy",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Password policy"),
    )
)]
pub async fn get_password_policy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<PasswordPolicy>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }
    let policy = load_password_policy(&state_guard.db).await;
    (StatusCode::OK, Json(ApiResponse::success(policy)))
}

/// `PUT /api/v1/admin/settings/password-policy` — Update password policy.
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/password-policy",
    tag = "Admin",
    summary = "Update password policy",
    security(("bearer_auth" = [])),
    request_body = PasswordPolicy,
    responses(
        (status = 200, description = "Password policy updated"),
    )
)]
pub async fn update_password_policy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(policy): Json<PasswordPolicy>,
) -> (StatusCode, Json<ApiResponse<PasswordPolicy>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    // Validate policy constraints
    if policy.min_length < 4 || policy.min_length > 128 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_POLICY",
                "Minimum length must be between 4 and 128",
            )),
        );
    }

    let json = serde_json::to_string(&policy).unwrap_or_default();
    if let Err(e) = state_guard.db.set_setting("password_policy", &json).await {
        tracing::error!("Failed to save password policy: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save policy")),
        );
    }

    AuditEntry::new(AuditEventType::SettingsChanged)
        .detail("Password policy updated")
        .log();

    (StatusCode::OK, Json(ApiResponse::success(policy)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOGIN HISTORY
// ═══════════════════════════════════════════════════════════════════════════════

/// A single login history entry.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LoginHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub success: bool,
}

/// Login history for a user (stored as JSON in settings).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoginHistory {
    pub entries: Vec<LoginHistoryEntry>,
}

impl LoginHistory {
    const MAX_ENTRIES: usize = 10;

    pub fn add(&mut self, entry: LoginHistoryEntry) {
        self.entries.insert(0, entry);
        self.entries.truncate(Self::MAX_ENTRIES);
    }
}

/// Record a login attempt in history.
pub async fn record_login(
    db: &crate::db::Database,
    user_id: Uuid,
    ip: &str,
    user_agent: &str,
    success: bool,
) {
    let key = format!("login_history:{user_id}");
    let mut history = match db.get_setting(&key).await {
        Ok(Some(val)) => serde_json::from_str::<LoginHistory>(&val).unwrap_or_default(),
        _ => LoginHistory::default(),
    };

    history.add(LoginHistoryEntry {
        timestamp: Utc::now(),
        ip_address: ip.to_string(),
        user_agent: user_agent.to_string(),
        success,
    });

    let json = serde_json::to_string(&history).unwrap_or_default();
    if let Err(e) = db.set_setting(&key, &json).await {
        tracing::warn!("Failed to record login history: {}", e);
    }
}

/// `GET /api/v1/auth/login-history` — Get login history for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/auth/login-history",
    tag = "Authentication",
    summary = "Get login history",
    description = "Returns the last 10 login attempts for the current user.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Login history"),
    )
)]
pub async fn get_login_history(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<LoginHistoryEntry>>>) {
    let state_guard = state.read().await;
    let key = format!("login_history:{}", auth_user.user_id);
    let history = match state_guard.db.get_setting(&key).await {
        Ok(Some(val)) => {
            serde_json::from_str::<LoginHistory>(&val)
                .unwrap_or_default()
                .entries
        }
        _ => Vec::new(),
    };
    (StatusCode::OK, Json(ApiResponse::success(history)))
}

/// `GET /api/v1/admin/users/{id}/login-history` — Get login history for any user (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/users/{id}/login-history",
    tag = "Admin",
    summary = "Get user login history (admin)",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "User UUID")),
    responses(
        (status = 200, description = "Login history"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_get_login_history(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse<Vec<LoginHistoryEntry>>>) {
    let state_guard = state.read().await;
    if check_admin(&state_guard, &auth_user).await.is_err() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let key = format!("login_history:{user_id}");
    let history = match state_guard.db.get_setting(&key).await {
        Ok(Some(val)) => {
            serde_json::from_str::<LoginHistory>(&val)
                .unwrap_or_default()
                .entries
        }
        _ => Vec::new(),
    };
    (StatusCode::OK, Json(ApiResponse::success(history)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SESSION MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Active session info for the user.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionInfo {
    /// Session ID (the access token prefix — not the full token, for security)
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

/// `GET /api/v1/auth/sessions` — List active sessions for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    tag = "Authentication",
    summary = "List active sessions",
    description = "Returns all active sessions for the current user.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Active sessions"),
    )
)]
pub async fn list_sessions(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    req: axum::extract::Request,
) -> (StatusCode, Json<ApiResponse<Vec<SessionInfo>>>) {
    let current_token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();

    let state_guard = state.read().await;
    let sessions = match state_guard
        .db
        .list_sessions_by_user(auth_user.user_id)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list sessions: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list sessions")),
            );
        }
    };

    let infos: Vec<SessionInfo> = sessions
        .into_iter()
        .map(|(token, session)| {
            let is_current = token == current_token;
            SessionInfo {
                id: format!("{}...", &token[..8.min(token.len())]),
                username: session.username,
                role: session.role,
                created_at: session.created_at,
                expires_at: session.expires_at,
                is_current,
            }
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(infos)))
}

/// `DELETE /api/v1/auth/sessions/{id}` — Revoke a specific session.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/sessions/{id}",
    tag = "Authentication",
    summary = "Revoke session",
    description = "Revokes a specific active session by its ID prefix.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Session ID prefix")),
    responses(
        (status = 200, description = "Session revoked"),
        (status = 404, description = "Session not found"),
    )
)]
pub async fn revoke_session(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Find the full token matching the prefix for this user
    let sessions = match state_guard
        .db
        .list_sessions_by_user(auth_user.user_id)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list sessions: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list sessions")),
            );
        }
    };

    // The session_id is a prefix like "ab12cd34..."
    let prefix = session_id.trim_end_matches("...");
    let token = sessions
        .iter()
        .find(|(t, _)| t.starts_with(prefix))
        .map(|(t, _)| t.clone());

    match token {
        Some(full_token) => {
            if let Err(e) = state_guard.db.delete_session(&full_token).await {
                tracing::error!("Failed to revoke session: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to revoke session")),
                );
            }
            AuditEntry::new(AuditEventType::LoginFailed)
                .user(auth_user.user_id, "")
                .detail("Session revoked manually")
                .log();
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Session not found")),
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API KEY SUPPORT
// ═══════════════════════════════════════════════════════════════════════════════

/// Stored API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    /// The key hash (Argon2) — the actual key is only shown once on creation
    pub key_hash: String,
    /// Key prefix for identification (first 8 chars)
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Response when creating an API key (includes the full key — shown only once).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    /// The full API key — shown only once
    pub api_key: String,
    pub key_prefix: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to create an API key.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    /// Optional expiry in days (None = never expires)
    pub expires_in_days: Option<u32>,
}

/// API key listing (without the actual key).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// `POST /api/v1/auth/api-keys` — Create a new API key.
#[utoipa::path(
    post,
    path = "/api/v1/auth/api-keys",
    tag = "Authentication",
    summary = "Create API key",
    description = "Creates a new API key. The key is shown only once in the response.",
    security(("bearer_auth" = [])),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn create_api_key(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateApiKeyRequest>,
) -> (StatusCode, Json<ApiResponse<CreateApiKeyResponse>>) {
    if req.name.is_empty() || req.name.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Name must be between 1 and 100 characters",
            )),
        );
    }

    let state_guard = state.read().await;

    // Generate the API key (phk_ prefix + 32 random bytes hex)
    let raw_key = format!("phk_{}", generate_access_token());
    let key_prefix = raw_key[..12].to_string();

    // Hash the key for storage
    let key_to_hash = raw_key.clone();
    let key_hash = match super::hash_password_simple(&key_to_hash).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash API key: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to create API key")),
            );
        }
    };

    let now = Utc::now();
    let expires_at = req
        .expires_in_days
        .map(|d| now + chrono::Duration::days(i64::from(d)));

    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        name: req.name.clone(),
        key_hash,
        key_prefix: key_prefix.clone(),
        created_at: now,
        expires_at,
        last_used_at: None,
        is_active: true,
    };

    // Store in settings as a list
    let keys_key = format!("api_keys:{}", auth_user.user_id);
    let mut keys: Vec<ApiKey> = match state_guard.db.get_setting(&keys_key).await {
        Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
        _ => Vec::new(),
    };
    keys.push(api_key.clone());

    let json = serde_json::to_string(&keys).unwrap_or_default();
    if let Err(e) = state_guard.db.set_setting(&keys_key, &json).await {
        tracing::error!("Failed to save API key: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save API key")),
        );
    }

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(auth_user.user_id, "")
        .detail(&format!("API key created: {}", req.name))
        .log();

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(CreateApiKeyResponse {
            id: api_key.id.to_string(),
            name: req.name,
            api_key: raw_key,
            key_prefix,
            expires_at,
        })),
    )
}

/// `GET /api/v1/auth/api-keys` — List API keys for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/auth/api-keys",
    tag = "Authentication",
    summary = "List API keys",
    description = "Returns all API keys for the current user (without the actual key values).",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "API keys"),
    )
)]
pub async fn list_api_keys(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<ApiKeyInfo>>>) {
    let state_guard = state.read().await;
    let keys_key = format!("api_keys:{}", auth_user.user_id);
    let keys: Vec<ApiKey> = match state_guard.db.get_setting(&keys_key).await {
        Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
        _ => Vec::new(),
    };

    let infos: Vec<ApiKeyInfo> = keys
        .into_iter()
        .filter(|k| k.is_active)
        .map(|k| ApiKeyInfo {
            id: k.id.to_string(),
            name: k.name,
            key_prefix: k.key_prefix,
            created_at: k.created_at,
            expires_at: k.expires_at,
            last_used_at: k.last_used_at,
            is_active: k.is_active,
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(infos)))
}

/// `DELETE /api/v1/auth/api-keys/{id}` — Revoke an API key.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/api-keys/{id}",
    tag = "Authentication",
    summary = "Revoke API key",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "API key UUID")),
    responses(
        (status = 200, description = "API key revoked"),
        (status = 404, description = "API key not found"),
    )
)]
pub async fn revoke_api_key(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Path(key_id): axum::extract::Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    let keys_key = format!("api_keys:{}", auth_user.user_id);
    let mut keys: Vec<ApiKey> = match state_guard.db.get_setting(&keys_key).await {
        Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
        _ => Vec::new(),
    };

    let found = keys.iter_mut().find(|k| k.id.to_string() == key_id);
    match found {
        Some(key) => {
            let name = key.name.clone();
            key.is_active = false;
            let json = serde_json::to_string(&keys).unwrap_or_default();
            if let Err(e) = state_guard.db.set_setting(&keys_key, &json).await {
                tracing::error!("Failed to revoke API key: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to revoke key")),
                );
            }
            AuditEntry::new(AuditEventType::UserUpdated)
                .user(auth_user.user_id, "")
                .detail(&format!("API key revoked: {name}"))
                .log();
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "API key not found")),
        ),
    }
}

/// Validate an API key from the `X-API-Key` header.
/// Returns the user_id if valid.
pub async fn validate_api_key(
    db: &crate::db::Database,
    api_key: &str,
) -> Option<Uuid> {
    // API keys are stored per-user; scan all users' keys
    // This is acceptable for moderate scale; for high-scale, a separate index table would be needed.
    let users = match db.list_users().await {
        Ok(u) => u,
        Err(_) => return None,
    };

    for user in &users {
        let keys_key = format!("api_keys:{}", user.id);
        let keys: Vec<ApiKey> = match db.get_setting(&keys_key).await {
            Ok(Some(val)) => serde_json::from_str(&val).unwrap_or_default(),
            _ => continue,
        };

        for key in &keys {
            if !key.is_active {
                continue;
            }
            // Check expiry
            if let Some(expires_at) = key.expires_at {
                if expires_at < Utc::now() {
                    continue;
                }
            }
            // Check prefix first (fast path)
            if !api_key.starts_with(&key.key_prefix) {
                continue;
            }
            // Verify the full key against hash
            if super::verify_password(api_key, &key.key_hash).await {
                return Some(user.id);
            }
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// QR CODE HELPER
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a QR code as base64-encoded PNG.
fn generate_qr_base64(data: &str) -> Result<String, String> {
    use image::Luma;
    use qrcode::QrCode;
    use std::io::Cursor;

    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("QR generation failed: {e}"))?;
    let image = code.render::<Luma<u8>>().min_dimensions(300, 300).build();

    let mut buf: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encoding failed: {e}"))?;

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &buf,
    ))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Password Policy Tests ───────────────────────────────────────────

    #[test]
    fn test_default_password_policy() {
        let policy = PasswordPolicy::default();
        assert_eq!(policy.min_length, 8);
        assert!(policy.require_uppercase);
        assert!(policy.require_lowercase);
        assert!(policy.require_number);
        assert!(!policy.require_special_char);
    }

    #[test]
    fn test_password_policy_check_valid() {
        let policy = PasswordPolicy::default();
        assert!(policy.check("Password1").is_ok());
        assert!(policy.check("MySecure123").is_ok());
    }

    #[test]
    fn test_password_policy_check_too_short() {
        let policy = PasswordPolicy {
            min_length: 12,
            ..Default::default()
        };
        let err = policy.check("Short1Aa").unwrap_err();
        assert!(err.contains("at least 12"));
    }

    #[test]
    fn test_password_policy_check_no_uppercase() {
        let policy = PasswordPolicy::default();
        assert!(policy.check("lowercase123").is_err());
    }

    #[test]
    fn test_password_policy_check_no_lowercase() {
        let policy = PasswordPolicy::default();
        assert!(policy.check("UPPERCASE123").is_err());
    }

    #[test]
    fn test_password_policy_check_no_digit() {
        let policy = PasswordPolicy::default();
        assert!(policy.check("NoDigitsHere").is_err());
    }

    #[test]
    fn test_password_policy_check_special_char_required() {
        let policy = PasswordPolicy {
            require_special_char: true,
            ..Default::default()
        };
        assert!(policy.check("Password1").is_err());
        assert!(policy.check("Password1!").is_ok());
    }

    #[test]
    fn test_password_policy_check_all_disabled() {
        let policy = PasswordPolicy {
            min_length: 4,
            require_uppercase: false,
            require_lowercase: false,
            require_number: false,
            require_special_char: false,
        };
        assert!(policy.check("abcd").is_ok());
        assert!(policy.check("abc").is_err());
    }

    #[test]
    fn test_password_policy_serialization_roundtrip() {
        let policy = PasswordPolicy {
            min_length: 10,
            require_uppercase: true,
            require_lowercase: false,
            require_number: true,
            require_special_char: true,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let back: PasswordPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.min_length, 10);
        assert!(back.require_uppercase);
        assert!(!back.require_lowercase);
        assert!(back.require_number);
        assert!(back.require_special_char);
    }

    // ─── Login History Tests ─────────────────────────────────────────────

    #[test]
    fn test_login_history_add() {
        let mut history = LoginHistory::default();
        for i in 0..15 {
            history.add(LoginHistoryEntry {
                timestamp: Utc::now(),
                ip_address: format!("10.0.0.{i}"),
                user_agent: "test".to_string(),
                success: true,
            });
        }
        // Should be truncated to MAX_ENTRIES
        assert_eq!(history.entries.len(), 10);
        // Most recent should be first
        assert_eq!(history.entries[0].ip_address, "10.0.0.14");
    }

    #[test]
    fn test_login_history_serialization() {
        let mut history = LoginHistory::default();
        history.add(LoginHistoryEntry {
            timestamp: Utc::now(),
            ip_address: "192.168.1.1".to_string(),
            user_agent: "Mozilla/5.0".to_string(),
            success: true,
        });
        let json = serde_json::to_string(&history).unwrap();
        let back: LoginHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].ip_address, "192.168.1.1");
    }

    #[test]
    fn test_login_history_entry_failed() {
        let entry = LoginHistoryEntry {
            timestamp: Utc::now(),
            ip_address: "10.0.0.1".to_string(),
            user_agent: "curl/7.0".to_string(),
            success: false,
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["ip_address"], "10.0.0.1");
    }

    // ─── API Key Tests ───────────────────────────────────────────────────

    #[test]
    fn test_api_key_serialization() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            key_hash: "hash".to_string(),
            key_prefix: "phk_abcdef12".to_string(),
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            last_used_at: None,
            is_active: true,
        };
        let json = serde_json::to_string(&key).unwrap();
        let back: ApiKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Test Key");
        assert!(back.is_active);
        assert!(back.expires_at.is_some());
    }

    #[test]
    fn test_api_key_info_from_key() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "My Key".to_string(),
            key_hash: "hash".to_string(),
            key_prefix: "phk_12345678".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
            is_active: true,
        };
        let info = ApiKeyInfo {
            id: key.id.to_string(),
            name: key.name.clone(),
            key_prefix: key.key_prefix.clone(),
            created_at: key.created_at,
            expires_at: key.expires_at,
            last_used_at: key.last_used_at,
            is_active: key.is_active,
        };
        assert_eq!(info.name, "My Key");
        assert_eq!(info.key_prefix, "phk_12345678");
    }

    #[test]
    fn test_create_api_key_request_deserialize() {
        let json = r#"{"name":"CI/CD Key","expires_in_days":90}"#;
        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "CI/CD Key");
        assert_eq!(req.expires_in_days, Some(90));
    }

    #[test]
    fn test_create_api_key_request_no_expiry() {
        let json = r#"{"name":"Permanent Key"}"#;
        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Permanent Key");
        assert!(req.expires_in_days.is_none());
    }

    // ─── 2FA Request/Response Tests ──────────────────────────────────────

    #[test]
    fn test_two_factor_verify_request_deserialize() {
        let json = r#"{"code":"123456"}"#;
        let req: TwoFactorVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "123456");
    }

    #[test]
    fn test_two_factor_disable_request_deserialize() {
        let json = r#"{"current_password":"mypassword"}"#;
        let req: TwoFactorDisableRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.current_password, "mypassword");
    }

    #[test]
    fn test_two_factor_status_response() {
        let resp = TwoFactorStatusResponse { enabled: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["enabled"], true);
    }

    #[test]
    fn test_two_factor_setup_response_serialization() {
        let resp = TwoFactorSetupResponse {
            secret: "JBSWY3DPEHPK3PXP".to_string(),
            otpauth_uri: "otpauth://totp/ParkHub:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=ParkHub".to_string(),
            qr_code_base64: "iVBORw0KGgo=".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["secret"].as_str().unwrap().contains("JBSWY3DPEHPK3PXP"));
        assert!(json["otpauth_uri"].as_str().unwrap().contains("otpauth://"));
    }

    // ─── Session Info Tests ──────────────────────────────────────────────

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            id: "ab12cd34...".to_string(),
            username: "testuser".to_string(),
            role: "user".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            is_current: true,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["username"], "testuser");
        assert_eq!(json["is_current"], true);
    }

    // ─── QR Code Generation Tests ────────────────────────────────────────

    #[test]
    fn test_generate_qr_base64() {
        let result = generate_qr_base64("https://example.com");
        assert!(result.is_ok());
        let b64 = result.unwrap();
        assert!(!b64.is_empty());
        // Should be valid base64
        assert!(base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &b64,
        )
        .is_ok());
    }

    #[test]
    fn test_generate_qr_base64_otpauth_uri() {
        let uri = "otpauth://totp/ParkHub:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=ParkHub";
        let result = generate_qr_base64(uri);
        assert!(result.is_ok());
    }

    // ─── 2FA Login Request Tests ─────────────────────────────────────────

    #[test]
    fn test_two_factor_login_request_deserialize() {
        let json = r#"{"temp_token":"abc123","code":"654321"}"#;
        let req: TwoFactorLoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.temp_token, "abc123");
        assert_eq!(req.code, "654321");
    }
}
