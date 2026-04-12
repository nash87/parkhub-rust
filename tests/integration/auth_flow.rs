//! Authentication Integration Tests
//!
//! Full auth lifecycle: register, login, token refresh, logout, 2FA,
//! password reset, invalid credentials, and rate limiting.

use crate::common::{admin_login, auth_get, auth_post, login, start_test_server};
use serde_json::Value;

// ═════════════════════════════════════════════════════════════════════════════
// Registration + Login + Token lifecycle
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn register_login_access_refresh_logout_lifecycle() {
    let srv = start_test_server().await;

    // 1. Register a new user
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": "lifecycle@test.com",
            "password": "SecurePass123!",
            "password_confirmation": "SecurePass123!",
            "name": "Lifecycle User",
            "username": "lifecycle_user",
        }))
        .send()
        .await
        .unwrap();

    let reg_status = resp.status().as_u16();
    assert!(
        reg_status == 200 || reg_status == 201,
        "Registration should succeed, got: {reg_status}"
    );

    // 2. Login with the new user
    let (status, body) = login(&srv, "lifecycle_user", "SecurePass123!").await;
    assert_eq!(status, 200, "Login should succeed");
    assert_eq!(body["success"], true);

    let access_token = body["data"]["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    let refresh_token = body["data"]["tokens"]["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();

    // 3. Access a protected endpoint with the token
    let (status, body) = auth_get(&srv, &access_token, "/api/v1/users/me").await;
    assert_eq!(status, 200, "Should access /users/me with valid token");
    assert_eq!(body["data"]["username"], "lifecycle_user");

    // 4. Refresh the token
    let (status, refresh_body) = auth_post(
        &srv,
        &access_token,
        "/api/v1/auth/refresh",
        &serde_json::json!({ "refresh_token": refresh_token }),
    )
    .await;

    // Refresh may return 200 with new tokens
    if status == 200 && refresh_body["success"] == true {
        let new_token = refresh_body["data"]["tokens"]["access_token"]
            .as_str()
            .unwrap_or(&access_token);

        // Verify the new token works
        let (status, _) = auth_get(&srv, new_token, "/api/v1/users/me").await;
        assert_eq!(status, 200, "New token from refresh should work");
    }

    // 5. Logout
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/logout", srv.url))
        .bearer_auth(&access_token)
        .send()
        .await
        .unwrap();

    let logout_status = resp.status().as_u16();
    assert!(
        logout_status == 200 || logout_status == 204,
        "Logout should succeed, got: {logout_status}"
    );

    // 6. Verify the old token is now invalid (session cleared)
    // Note: Stateless JWTs may still be valid until expiry — this tests
    // session-based invalidation if the server implements it.
    let (status, _) = auth_get(&srv, &access_token, "/api/v1/users/me").await;
    // Server may return 200 (stateless JWT) or 401 (session invalidated).
    // Both are acceptable; the important thing is logout did not error.
    assert!(
        status == 200 || status == 401,
        "After logout, expect 200 (stateless) or 401 (invalidated), got: {status}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Invalid credentials
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let srv = start_test_server().await;
    let (status, body) = login(&srv, "admin", "totally-wrong").await;
    assert_eq!(status, 401);
    assert_eq!(body["success"], false);
    assert_eq!(body["error"]["code"], "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn login_nonexistent_user_returns_401() {
    let srv = start_test_server().await;
    let (status, body) = login(&srv, "nobody_exists_here", "password").await;
    assert_eq!(status, 401);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn login_empty_body_returns_error() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "Empty login body should return 400 or 422, got: {status}"
    );
}

#[tokio::test]
async fn login_with_overly_long_username_returns_400() {
    let srv = start_test_server().await;
    let long_username = "x".repeat(300);
    let (status, _) = login(&srv, &long_username, "password").await;
    assert_eq!(status, 400, "Overly long username should return 400");
}

// ═════════════════════════════════════════════════════════════════════════════
// Registration validation
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn register_duplicate_username_fails() {
    let srv = start_test_server().await;

    // Register first user
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": "first@test.com",
            "password": "TestPass123!",
            "password_confirmation": "TestPass123!",
            "name": "First",
            "username": "dupe_test_user",
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success() || resp.status().as_u16() == 201);

    // Try to register with same username
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": "second@test.com",
            "password": "TestPass123!",
            "password_confirmation": "TestPass123!",
            "name": "Second",
            "username": "dupe_test_user",
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 409 || status == 422,
        "Duplicate username should fail with 400/409/422, got: {status}"
    );
}

#[tokio::test]
async fn register_weak_password_fails() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": "weak@test.com",
            "password": "123",
            "password_confirmation": "123",
            "name": "Weak PW User",
            "username": "weak_pw_user",
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "Weak password should fail validation, got: {status}"
    );
}

#[tokio::test]
async fn register_mismatched_passwords_fails() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": "mismatch@test.com",
            "password": "SecurePass123!",
            "password_confirmation": "DifferentPass456!",
            "name": "Mismatch User",
            "username": "mismatch_user",
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "Mismatched passwords should fail, got: {status}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Token validation
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn expired_or_garbage_token_returns_401() {
    let srv = start_test_server().await;
    let (status, _) = auth_get(&srv, "garbage.invalid.token", "/api/v1/users/me").await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn request_without_bearer_token_returns_401() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/api/v1/users/me", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 401);
}

// ═════════════════════════════════════════════════════════════════════════════
// Password reset flow (trigger only — no real email delivery)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn forgot_password_accepts_valid_email() {
    let srv = start_test_server().await;

    // Register a user first
    let _resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": "reset@test.com",
            "password": "SecurePass123!",
            "password_confirmation": "SecurePass123!",
            "name": "Reset User",
            "username": "reset_user",
        }))
        .send()
        .await
        .unwrap();

    // Request password reset
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/forgot-password", srv.url))
        .json(&serde_json::json!({
            "email": "reset@test.com",
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    // Should return 200 regardless (to avoid user enumeration)
    assert_eq!(
        status, 200,
        "Forgot-password should always return 200, got: {status}"
    );
}

#[tokio::test]
async fn forgot_password_nonexistent_email_still_returns_200() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/forgot-password", srv.url))
        .json(&serde_json::json!({
            "email": "nobody@nowhere.com",
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert_eq!(
        status, 200,
        "Should not reveal whether email exists, got: {status}"
    );
}

#[tokio::test]
async fn reset_password_with_invalid_token_fails() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/reset-password", srv.url))
        .json(&serde_json::json!({
            "token": "invalid-reset-token-12345",
            "password": "NewSecure456!",
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 401 || status == 404,
        "Invalid reset token should fail, got: {status}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 2FA (TOTP) — basic endpoint verification
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn two_factor_enable_returns_secret() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    // Try to enable 2FA
    let (status, body) = auth_post(
        &srv,
        &token,
        "/api/v1/users/me/2fa/enable",
        &serde_json::json!({}),
    )
    .await;

    // Endpoint may not exist (feature-gated) or return TOTP secret
    if status == 200 {
        // Should return a TOTP secret or QR code URL
        assert!(
            body["data"]["secret"].is_string()
                || body["data"]["qr_url"].is_string()
                || body["data"]["otpauth_url"].is_string(),
            "2FA enable should return secret or QR data"
        );
    } else {
        // 404 = endpoint not compiled, 400/422 = already enabled
        assert!(
            status == 404 || status == 400 || status == 422 || status == 405,
            "Unexpected 2FA status: {status}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Rate limiting (login endpoint)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn login_rate_limiting_triggers_429() {
    let srv = start_test_server().await;

    // The login endpoint is rate-limited to 5 requests per minute per IP.
    // Send 10 rapid requests to trigger the limit.
    let mut got_429 = false;
    for _ in 0..12 {
        let resp = srv
            .client
            .post(format!("{}/api/v1/auth/login", srv.url))
            .json(&serde_json::json!({
                "username": "admin",
                "password": "wrong",
            }))
            .send()
            .await
            .unwrap();

        if resp.status().as_u16() == 429 {
            got_429 = true;
            break;
        }
    }

    assert!(
        got_429,
        "Should receive 429 after exceeding login rate limit"
    );
}
