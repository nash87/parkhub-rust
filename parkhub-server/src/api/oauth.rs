//! OAuth / Social Login handlers: Google and GitHub.
//!
//! Self-service: each ParkHub installation configures its own OAuth apps.
//! Buttons only appear when the corresponding `OAUTH_*` env vars are set.
//!
//! Endpoints:
//! - `GET /api/v1/auth/oauth/providers`        — list available OAuth providers (public, no secrets)
//! - `GET /api/v1/auth/oauth/google`           — redirect to Google consent screen
//! - `GET /api/v1/auth/oauth/google/callback`  — exchange code, create/link user
//! - `GET /api/v1/auth/oauth/github`           — redirect to GitHub consent screen
//! - `GET /api/v1/auth/oauth/github/callback`  — exchange code, create/link user

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, AuthTokens, LoginResponse, User, UserPreferences, UserRole};

use crate::audit::{AuditEntry, AuditEventType};
use crate::db::Session;
use crate::metrics;

use super::{generate_access_token, hash_password_simple, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// OAuth configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub redirect_base_url: String,
}

impl OAuthConfig {
    /// Load configuration from environment variables.
    /// Returns `None` if any required variable is missing.
    pub fn from_env() -> Option<Self> {
        Some(Self {
            google_client_id: std::env::var("OAUTH_GOOGLE_CLIENT_ID").ok()?,
            google_client_secret: std::env::var("OAUTH_GOOGLE_CLIENT_SECRET").ok()?,
            github_client_id: std::env::var("OAUTH_GITHUB_CLIENT_ID").ok()?,
            github_client_secret: std::env::var("OAUTH_GITHUB_CLIENT_SECRET").ok()?,
            redirect_base_url: std::env::var("APP_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Query / response types
// ─────────────────────────────────────────────────────────────────────────────

/// Query parameters returned by the OAuth provider callback.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    /// CSRF state parameter — validated against the `oauth_state` cookie.
    #[serde(default)]
    pub state: Option<String>,
}

/// Google token exchange response.
#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    /// ID token from Google — reserved for future JWT verification.
    #[serde(default)]
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// Google user info from the userinfo endpoint.
#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    #[serde(default)]
    sub: String,
    email: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    picture: Option<String>,
}

/// GitHub token exchange response.
#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
}

/// GitHub user info from the API.
#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: i64,
    login: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

/// GitHub email from the user/emails API.
#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

/// OAuth provider info stored alongside the user record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    pub provider: String,
    pub provider_user_id: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// URL builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build the Google OAuth consent URL including the CSRF `state` nonce.
pub fn google_auth_url(config: &OAuthConfig, state: &str) -> String {
    let redirect_uri = format!(
        "{}/api/v1/auth/oauth/google/callback",
        config.redirect_base_url
    );
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={}&\
         redirect_uri={}&\
         response_type=code&\
         scope=openid%20email%20profile&\
         access_type=offline&\
         prompt=consent&\
         state={}",
        urlencoding(&config.google_client_id),
        urlencoding(&redirect_uri),
        urlencoding(state),
    )
}

/// Build the GitHub OAuth consent URL including the CSRF `state` nonce.
pub fn github_auth_url(config: &OAuthConfig, state: &str) -> String {
    let redirect_uri = format!(
        "{}/api/v1/auth/oauth/github/callback",
        config.redirect_base_url
    );
    format!(
        "https://github.com/login/oauth/authorize?\
         client_id={}&\
         redirect_uri={}&\
         scope={}&\
         state={}",
        urlencoding(&config.github_client_id),
        urlencoding(&redirect_uri),
        urlencoding("user:email"),
        urlencoding(state),
    )
}

/// Simple percent-encoding for URL query parameters.
fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// CSRF state cookie helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Cookie name used to store the OAuth CSRF state nonce.
const OAUTH_STATE_COOKIE: &str = "oauth_state";
/// Lifetime of the CSRF state cookie in seconds (10 minutes).
const OAUTH_STATE_MAX_AGE: u32 = 600;

/// Build a `Set-Cookie` header value for the OAuth CSRF state nonce.
fn build_oauth_state_cookie(state_nonce: &str) -> String {
    let secure_flag = std::env::var("APP_URL")
        .map(|u| !u.starts_with("http://localhost") && !u.starts_with("http://127.0.0.1"))
        .unwrap_or(false);
    let mut cookie = format!(
        "{OAUTH_STATE_COOKIE}={state_nonce}; HttpOnly; SameSite=Lax; Path=/api/v1/auth/oauth; Max-Age={OAUTH_STATE_MAX_AGE}"
    );
    if secure_flag {
        cookie.push_str("; Secure");
    }
    cookie
}

/// Extract the OAuth CSRF state nonce from the incoming `Cookie` header.
fn extract_oauth_state_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{OAUTH_STATE_COOKIE}="))
                    .map(std::string::ToString::to_string)
            })
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// Response type for the providers endpoint.
#[derive(Debug, Serialize)]
pub struct OAuthProvidersResponse {
    pub google: bool,
    pub github: bool,
}

/// `GET /api/v1/auth/oauth/providers` — list which OAuth providers are configured.
///
/// Returns `{ google: true/false, github: true/false }` based on whether the
/// corresponding environment variables are set. No secrets are exposed.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oauth/providers",
    tag = "OAuth",
    summary = "List available OAuth providers",
    description = "Returns which OAuth providers are configured. No authentication required.",
    responses(
        (status = 200, description = "Provider availability"),
    )
)]
pub async fn oauth_providers() -> Json<ApiResponse<OAuthProvidersResponse>> {
    let google = std::env::var("OAUTH_GOOGLE_CLIENT_ID")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let github = std::env::var("OAUTH_GITHUB_CLIENT_ID")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    Json(ApiResponse::success(OAuthProvidersResponse {
        google,
        github,
    }))
}

/// `GET /api/v1/auth/oauth/google` — redirect to Google OAuth consent screen.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oauth/google",
    tag = "OAuth",
    summary = "Initiate Google OAuth login",
    responses(
        (status = 302, description = "Redirect to Google"),
        (status = 503, description = "OAuth not configured"),
    )
)]
pub async fn oauth_google_redirect() -> Response {
    let Some(config) = OAuthConfig::from_env() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error(
                "OAUTH_NOT_CONFIGURED",
                "Google OAuth is not configured",
            )),
        )
            .into_response();
    };
    let state_nonce = Uuid::new_v4().to_string();
    let cookie = build_oauth_state_cookie(&state_nonce);
    let mut resp = Redirect::temporary(&google_auth_url(&config, &state_nonce)).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    resp
}

/// `GET /api/v1/auth/oauth/google/callback` — exchange code for token, create/link user.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oauth/google/callback",
    tag = "OAuth",
    summary = "Google OAuth callback",
    params(("code" = String, Query, description = "Authorization code")),
    responses(
        (status = 200, description = "Login successful"),
        (status = 400, description = "Missing code"),
        (status = 503, description = "OAuth not configured"),
    )
)]
pub async fn oauth_google_callback(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(params): Query<OAuthCallbackParams>,
) -> Response {
    let Some(config) = OAuthConfig::from_env() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error(
                "OAUTH_NOT_CONFIGURED",
                "Google OAuth is not configured",
            )),
        )
            .into_response();
    };

    // Validate CSRF state: the `state` query param must match the `oauth_state` cookie.
    let stored_state = extract_oauth_state_cookie(&headers);
    match (&stored_state, &params.state) {
        (Some(stored), Some(received)) if stored == received => {}
        _ => return oauth_error_response("Invalid or missing CSRF state parameter"),
    }

    let redirect_uri = format!(
        "{}/api/v1/auth/oauth/google/callback",
        config.redirect_base_url
    );

    // Exchange authorization code for access token
    let client = reqwest::Client::new();
    let token_res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", params.code.as_str()),
            ("client_id", &config.google_client_id),
            ("client_secret", &config.google_client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await;

    let token_data: GoogleTokenResponse = match token_res {
        Ok(res) if res.status().is_success() => match res.json().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to parse Google token response: {e}");
                return oauth_error_response("Failed to exchange authorization code");
            }
        },
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            tracing::error!("Google token exchange failed: {status} — {body}");
            return oauth_error_response("Google token exchange failed");
        }
        Err(e) => {
            tracing::error!("Google token request failed: {e}");
            return oauth_error_response("Failed to contact Google");
        }
    };

    // Fetch user info
    let userinfo_res = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token_data.access_token)
        .send()
        .await;

    let user_info: GoogleUserInfo = match userinfo_res {
        Ok(res) if res.status().is_success() => match res.json().await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("Failed to parse Google userinfo: {e}");
                return oauth_error_response("Failed to get user info from Google");
            }
        },
        _ => {
            return oauth_error_response("Failed to get user info from Google");
        }
    };

    let provider = OAuthProvider {
        provider: "google".to_string(),
        provider_user_id: user_info.sub.clone(),
    };

    complete_oauth_login(
        state,
        &user_info.email,
        &user_info.name,
        user_info.picture.as_deref(),
        &provider,
    )
    .await
}

/// `GET /api/v1/auth/oauth/github` — redirect to GitHub OAuth consent screen.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oauth/github",
    tag = "OAuth",
    summary = "Initiate GitHub OAuth login",
    responses(
        (status = 302, description = "Redirect to GitHub"),
        (status = 503, description = "OAuth not configured"),
    )
)]
pub async fn oauth_github_redirect() -> Response {
    let Some(config) = OAuthConfig::from_env() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error(
                "OAUTH_NOT_CONFIGURED",
                "GitHub OAuth is not configured",
            )),
        )
            .into_response();
    };
    let state_nonce = Uuid::new_v4().to_string();
    let cookie = build_oauth_state_cookie(&state_nonce);
    let mut resp = Redirect::temporary(&github_auth_url(&config, &state_nonce)).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    resp
}

/// `GET /api/v1/auth/oauth/github/callback` — exchange code for token, create/link user.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oauth/github/callback",
    tag = "OAuth",
    summary = "GitHub OAuth callback",
    params(("code" = String, Query, description = "Authorization code")),
    responses(
        (status = 200, description = "Login successful"),
        (status = 400, description = "Missing code"),
        (status = 503, description = "OAuth not configured"),
    )
)]
pub async fn oauth_github_callback(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(params): Query<OAuthCallbackParams>,
) -> Response {
    let Some(config) = OAuthConfig::from_env() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error(
                "OAUTH_NOT_CONFIGURED",
                "GitHub OAuth is not configured",
            )),
        )
            .into_response();
    };

    // Validate CSRF state: the `state` query param must match the `oauth_state` cookie.
    let stored_state = extract_oauth_state_cookie(&headers);
    match (&stored_state, &params.state) {
        (Some(stored), Some(received)) if stored == received => {}
        _ => return oauth_error_response("Invalid or missing CSRF state parameter"),
    }

    let redirect_uri = format!(
        "{}/api/v1/auth/oauth/github/callback",
        config.redirect_base_url
    );

    // Exchange code for access token
    let client = reqwest::Client::new();
    let token_res = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", config.github_client_id.as_str()),
            ("client_secret", &config.github_client_secret),
            ("code", &params.code),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await;

    let token_data: GitHubTokenResponse = match token_res {
        Ok(res) if res.status().is_success() => match res.json().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to parse GitHub token response: {e}");
                return oauth_error_response("Failed to exchange authorization code");
            }
        },
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            tracing::error!("GitHub token exchange failed: {status} — {body}");
            return oauth_error_response("GitHub token exchange failed");
        }
        Err(e) => {
            tracing::error!("GitHub token request failed: {e}");
            return oauth_error_response("Failed to contact GitHub");
        }
    };

    // Fetch user info
    let userinfo_res = client
        .get("https://api.github.com/user")
        .header("User-Agent", "ParkHub")
        .bearer_auth(&token_data.access_token)
        .send()
        .await;

    let user_info: GitHubUserInfo = match userinfo_res {
        Ok(res) if res.status().is_success() => match res.json().await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("Failed to parse GitHub userinfo: {e}");
                return oauth_error_response("Failed to get user info from GitHub");
            }
        },
        _ => {
            return oauth_error_response("Failed to get user info from GitHub");
        }
    };

    // If email is not public, fetch from /user/emails
    let email = if let Some(ref email) = user_info.email {
        email.clone()
    } else {
        match fetch_github_primary_email(&client, &token_data.access_token).await {
            Some(e) => e,
            None => {
                return oauth_error_response("Could not retrieve email from GitHub. Please make your email public or grant the user:email scope.");
            }
        }
    };

    let provider = OAuthProvider {
        provider: "github".to_string(),
        provider_user_id: user_info.id.to_string(),
    };

    let name = user_info.name.unwrap_or_else(|| user_info.login.clone());

    complete_oauth_login(
        state,
        &email,
        &name,
        user_info.avatar_url.as_deref(),
        &provider,
    )
    .await
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Fetch the primary verified email from GitHub's `/user/emails` endpoint.
async fn fetch_github_primary_email(client: &reqwest::Client, token: &str) -> Option<String> {
    let res = client
        .get("https://api.github.com/user/emails")
        .header("User-Agent", "ParkHub")
        .bearer_auth(token)
        .send()
        .await
        .ok()?;

    let emails: Vec<GitHubEmail> = res.json().await.ok()?;
    emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
}

/// Shared logic: find or create user by email, link OAuth provider, issue session.
async fn complete_oauth_login(
    state: SharedState,
    email: &str,
    name: &str,
    picture: Option<&str>,
    provider: &OAuthProvider,
) -> Response {
    let state_guard = state.read().await;

    // Try to find existing user by email
    let user = match state_guard.db.get_user_by_email(email).await {
        Ok(Some(existing)) => {
            // Link OAuth provider info (store as JSON in settings for now)
            let key = format!("oauth:{}:{}", provider.provider, existing.id);
            let val = serde_json::to_string(provider).unwrap_or_default();
            let _ = state_guard.db.set_setting(&key, &val).await;

            AuditEntry::new(AuditEventType::LoginSuccess)
                .user(existing.id, &existing.username)
                .detail(&format!("oauth:{}", provider.provider))
                .log();
            metrics::record_auth_event("login", true);

            existing
        }
        _ => {
            // Enforce self-registration gate before creating a new account.
            if !state_guard.config.allow_self_registration {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::<()>::error(
                        "REGISTRATION_DISABLED",
                        "Self-registration is disabled. Contact an administrator.",
                    )),
                )
                    .into_response();
            }

            // Create new user
            let username = email.split('@').next().unwrap_or("user").to_string();

            // Deduplicate username
            let mut final_username = username.clone();
            let mut counter = 1;
            while let Ok(Some(_)) = state_guard.db.get_user_by_username(&final_username).await {
                final_username = format!("{username}{counter}");
                counter += 1;
            }

            // Generate a random password hash (user logs in via OAuth, not password)
            let random_pw = Uuid::new_v4().to_string();
            let password_hash = match hash_password_simple(&random_pw).await {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("Failed to hash OAuth placeholder password: {e}");
                    return oauth_error_response("Internal server error");
                }
            };

            let now = Utc::now();
            let new_user = User {
                id: Uuid::new_v4(),
                username: final_username,
                email: email.to_string(),
                password_hash,
                name: name.to_string(),
                picture: picture.map(String::from),
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

            if let Err(e) = state_guard.db.save_user(&new_user).await {
                tracing::error!("Failed to save OAuth user: {e}");
                return oauth_error_response("Failed to create account");
            }

            // Store OAuth provider info
            let key = format!("oauth:{}:{}", provider.provider, new_user.id);
            let val = serde_json::to_string(provider).unwrap_or_default();
            let _ = state_guard.db.set_setting(&key, &val).await;

            let audit = AuditEntry::new(AuditEventType::UserCreated)
                .user(new_user.id, &new_user.username)
                .detail(&format!("oauth:{}", provider.provider))
                .log();
            audit.persist(&state_guard.db).await;
            metrics::record_auth_event("register", true);

            new_user
        }
    };

    // Create session
    let session_hours = i64::from(state_guard.config.session_timeout_minutes).max(60) / 60;
    let role_str = format!("{:?}", user.role).to_lowercase();
    let session = Session::new(user.id, session_hours, &user.username, &role_str);
    let access_token = generate_access_token();

    if let Err(e) = state_guard.db.save_session(&access_token, &session).await {
        tracing::error!("Failed to save OAuth session: {e}");
        return oauth_error_response("Failed to create session");
    }
    drop(state_guard);

    // Build auth cookie
    let cookie_max_age = session_hours * 3600;
    let cookie = super::auth::build_auth_cookie(&access_token, cookie_max_age);

    // Return user data + set cookie
    let mut response_user = user;
    response_user.password_hash = String::new();

    let body = Json(ApiResponse::success(LoginResponse {
        user: response_user,
        tokens: AuthTokens {
            access_token,
            refresh_token: session.refresh_token,
            expires_at: session.expires_at,
            token_type: "Bearer".to_string(),
        },
    }));

    let mut resp = (StatusCode::OK, body).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    resp
}

/// Standard error response for OAuth failures.
fn oauth_error_response(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::<()>::error("OAUTH_ERROR", message)),
    )
        .into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_auth_url_generation() {
        let config = OAuthConfig {
            google_client_id: "test-client-id".to_string(),
            google_client_secret: "test-secret".to_string(),
            github_client_id: "gh-client".to_string(),
            github_client_secret: "gh-secret".to_string(),
            redirect_base_url: "https://app.example.com".to_string(),
        };
        let url = google_auth_url(&config, "test-nonce");
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=openid"));
        assert!(url.contains("google%2Fcallback"));
    }

    #[test]
    fn test_github_auth_url_generation() {
        let config = OAuthConfig {
            google_client_id: "g-client".to_string(),
            google_client_secret: "g-secret".to_string(),
            github_client_id: "gh-test-id".to_string(),
            github_client_secret: "gh-secret".to_string(),
            redirect_base_url: "https://app.example.com".to_string(),
        };
        let url = github_auth_url(&config, "test-nonce");
        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=gh-test-id"));
        assert!(url.contains("scope=user%3Aemail") || url.contains("scope=user:email"));
        assert!(url.contains("github%2Fcallback"));
    }

    #[test]
    fn test_oauth_config_redirect_base_url() {
        let config = OAuthConfig {
            google_client_id: "id".to_string(),
            google_client_secret: "secret".to_string(),
            github_client_id: "id".to_string(),
            github_client_secret: "secret".to_string(),
            redirect_base_url: "https://parkhub.example.com".to_string(),
        };
        let google_url = google_auth_url(&config, "nonce");
        let github_url = github_auth_url(&config, "nonce");
        assert!(google_url.contains("parkhub.example.com"));
        assert!(github_url.contains("parkhub.example.com"));
    }

    #[test]
    fn test_oauth_provider_serde_roundtrip() {
        let provider = OAuthProvider {
            provider: "google".to_string(),
            provider_user_id: "12345".to_string(),
        };
        let json = serde_json::to_string(&provider).unwrap();
        let deserialized: OAuthProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider, "google");
        assert_eq!(deserialized.provider_user_id, "12345");
    }

    #[test]
    fn test_oauth_callback_params_deserialize() {
        let json = r#"{"code":"auth_code_123","state":"random_state"}"#;
        let params: OAuthCallbackParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.code, "auth_code_123");
        assert_eq!(params.state, Some("random_state".to_string()));
    }

    #[test]
    fn test_oauth_callback_params_no_state() {
        let json = r#"{"code":"auth_code_456"}"#;
        let params: OAuthCallbackParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.code, "auth_code_456");
        assert!(params.state.is_none());
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("a@b.com"), "a%40b.com");
        assert_eq!(urlencoding("test"), "test");
    }

    #[test]
    fn test_google_auth_url_contains_required_params() {
        let config = OAuthConfig {
            google_client_id: "my-app.apps.googleusercontent.com".to_string(),
            google_client_secret: "secret".to_string(),
            github_client_id: "gh".to_string(),
            github_client_secret: "gh-s".to_string(),
            redirect_base_url: "http://localhost:3000".to_string(),
        };
        let url = google_auth_url(&config, "some-nonce");
        // Must contain all required OAuth 2.0 params
        assert!(url.contains("client_id="));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope="));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("state=some-nonce"));
    }

    #[test]
    fn test_github_auth_url_contains_email_scope() {
        let config = OAuthConfig {
            google_client_id: "g".to_string(),
            google_client_secret: "gs".to_string(),
            github_client_id: "gh-id".to_string(),
            github_client_secret: "gh-secret".to_string(),
            redirect_base_url: "http://localhost:3000".to_string(),
        };
        let url = github_auth_url(&config, "my-nonce");
        assert!(url.contains("scope=user%3Aemail") || url.contains("scope=user:email"));
        assert!(url.contains("state=my-nonce"));
    }

    #[test]
    fn test_google_auth_url_embeds_state_nonce() {
        let config = OAuthConfig {
            google_client_id: "id".to_string(),
            google_client_secret: "sec".to_string(),
            github_client_id: "gh".to_string(),
            github_client_secret: "ghs".to_string(),
            redirect_base_url: "http://localhost:3000".to_string(),
        };
        let nonce = "abc123-csrf-nonce";
        let url = google_auth_url(&config, nonce);
        assert!(url.contains(&format!("state={nonce}")));
    }

    #[test]
    fn test_github_auth_url_embeds_state_nonce() {
        let config = OAuthConfig {
            google_client_id: "id".to_string(),
            google_client_secret: "sec".to_string(),
            github_client_id: "gh".to_string(),
            github_client_secret: "ghs".to_string(),
            redirect_base_url: "http://localhost:3000".to_string(),
        };
        let nonce = "xyz789-csrf-nonce";
        let url = github_auth_url(&config, nonce);
        assert!(url.contains(&format!("state={nonce}")));
    }

    #[test]
    fn test_build_oauth_state_cookie_format() {
        let cookie = build_oauth_state_cookie("test-nonce-value");
        assert!(cookie.starts_with("oauth_state=test-nonce-value;"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/api/v1/auth/oauth"));
        assert!(cookie.contains(&format!("Max-Age={OAUTH_STATE_MAX_AGE}")));
    }

    #[test]
    fn test_extract_oauth_state_cookie_present() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_static("oauth_state=my-nonce; parkhub_token=tok"),
        );
        let result = extract_oauth_state_cookie(&headers);
        assert_eq!(result, Some("my-nonce".to_string()));
    }

    #[test]
    fn test_extract_oauth_state_cookie_absent() {
        let headers = axum::http::HeaderMap::new();
        let result = extract_oauth_state_cookie(&headers);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_oauth_state_cookie_other_cookies_only() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_static("parkhub_token=abc; some_other=value"),
        );
        let result = extract_oauth_state_cookie(&headers);
        assert!(result.is_none());
    }
}
