//! Shared test infrastructure for integration and simulation tests.
//!
//! Starts a real `parkhub-server` process on a random port with a temporary
//! database directory.  Every helper talks to the server via HTTP (`reqwest`)
//! — exactly like a real client.

use reqwest::Client;
use serde_json::Value;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// A running test server with its HTTP base URL and temporary data directory.
pub struct TestServer {
    pub url: String,
    pub client: Client,
    _process: Child,
    _tmp_dir: tempfile::TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self._process.kill();
        let _ = self._process.wait();
    }
}

/// Find a free TCP port by binding to port 0.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind to free port");
    listener.local_addr().expect("local_addr").port()
}

/// Locate the `parkhub-server` binary.
///
/// Looks for:
///  1. `target/debug/parkhub-server`
///  2. `target/release/parkhub-server`
fn server_binary() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug = root.join("target/debug/parkhub-server");
    if debug.exists() {
        return debug;
    }
    let release = root.join("target/release/parkhub-server");
    if release.exists() {
        return release;
    }
    // Workspace layout: binary might be at workspace root target
    let ws_debug = root
        .parent()
        .map(|p| p.join("target/debug/parkhub-server"))
        .unwrap_or_default();
    if ws_debug.exists() {
        return ws_debug;
    }
    panic!(
        "parkhub-server binary not found. Build with `cargo build -p parkhub-server` first.\n\
         Searched: {:?}, {:?}, {:?}",
        debug, release, ws_debug
    );
}

/// Start a fresh test server and wait until it responds to `/health`.
///
/// Two-phase start: first launch creates config.toml with defaults, then we
/// patch it to enable self-registration and restart.
pub async fn start_test_server() -> TestServer {
    let port = free_port();
    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let data_dir = tmp_dir.path().to_path_buf();
    let binary = server_binary();
    let config_path = data_dir.join("config.toml");

    // Phase 1: start server so it creates its default config.toml
    let mut child = Command::new(&binary)
        .args([
            "--headless",
            "--unattended",
            "--port",
            &port.to_string(),
            "--data-dir",
            &data_dir.to_string_lossy(),
        ])
        .env("DEMO_MODE", "true")
        .env("PARKHUB_ADMIN_PASSWORD", "Admin123!")
        .env("RUST_LOG", "warn")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start server at {:?}: {}", binary, e));

    let url = format!("http://127.0.0.1:{}", port);
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build reqwest client");

    // Wait for first start to create config and become healthy
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if Instant::now() > deadline {
            panic!("Server did not become healthy within 15 seconds on port {port}");
        }
        match client.get(format!("{url}/health")).send().await {
            Ok(resp) if resp.status().is_success() => break,
            _ => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }

    // Phase 2: patch config.toml to enable self-registration, then restart
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).expect("read config");
        let patched = content.replace(
            "allow_self_registration = false",
            "allow_self_registration = true",
        );
        std::fs::write(&config_path, patched).expect("write patched config");

        // Kill and restart
        let _ = child.kill();
        let _ = child.wait();

        let child2 = Command::new(&binary)
            .args([
                "--headless",
                "--unattended",
                "--port",
                &port.to_string(),
                "--data-dir",
                &data_dir.to_string_lossy(),
            ])
            .env("DEMO_MODE", "true")
            .env("PARKHUB_ADMIN_PASSWORD", "Admin123!")
            .env("RUST_LOG", "warn")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to restart server: {}", e));

        child = child2;

        // Wait for restart
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if Instant::now() > deadline {
                panic!("Server did not restart within 15 seconds on port {port}");
            }
            match client.get(format!("{url}/health")).send().await {
                Ok(resp) if resp.status().is_success() => break,
                _ => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    }

    TestServer {
        url,
        client,
        _process: child,
        _tmp_dir: tmp_dir,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Log in as the admin user. Returns `(access_token, user_id)`.
pub async fn admin_login(srv: &TestServer) -> (String, String) {
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .json(&serde_json::json!({
            "username": "admin",
            "password": "Admin123!",
        }))
        .send()
        .await
        .expect("admin login request");

    assert!(
        resp.status().is_success(),
        "admin login failed: {}",
        resp.status()
    );
    let body: Value = resp.json().await.expect("parse login json");
    let token = body["data"]["tokens"]["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    let user_id = body["data"]["user"]["id"]
        .as_str()
        .expect("user id")
        .to_string();
    (token, user_id)
}

/// Register a new user.  Returns `(access_token, user_id, username)`.
///
/// The server generates the username from the email prefix (part before @).
pub async fn create_test_user(srv: &TestServer, suffix: &str) -> (String, String, String) {
    let email = format!("testuser_{suffix}@example.com");
    let password = "TestPass123!";
    // Server derives username from email prefix
    let expected_username = format!("testuser_{suffix}");

    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/register", srv.url))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "password_confirmation": password,
            "name": format!("Test User {suffix}"),
        }))
        .send()
        .await
        .expect("register request");

    let status = resp.status();
    assert!(
        status.is_success() || status.as_u16() == 201,
        "register failed: {} body: {:?}",
        status,
        resp.text().await.unwrap_or_default()
    );

    // Now log in to get a token (username = email prefix)
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .json(&serde_json::json!({
            "username": expected_username,
            "password": password,
        }))
        .send()
        .await
        .expect("login after register");

    assert!(
        resp.status().is_success(),
        "login after register failed for username '{expected_username}'"
    );
    let body: Value = resp.json().await.expect("parse json");
    let token = body["data"]["tokens"]["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    let user_id = body["data"]["user"]["id"]
        .as_str()
        .expect("user id")
        .to_string();
    (token, user_id, expected_username)
}

/// Create a parking lot (requires admin token).  Returns `lot_id`.
pub async fn create_test_lot(srv: &TestServer, token: &str, name: &str) -> String {
    let resp = srv
        .client
        .post(format!("{}/api/v1/lots", srv.url))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "name": name,
            "address": "123 Test Street",
            "latitude": 48.137154,
            "longitude": 11.576124,
            "total_slots": 10,
            "available_slots": 10,
            "floors": [],
            "amenities": [],
            "pricing": {
                "currency": "EUR",
                "rates": [{"duration_minutes": 60, "price": 2.50, "label": "1 Hour"}],
                "daily_max": 15.0,
                "monthly_pass": null
            },
            "operating_hours": {
                "is_24h": true,
                "monday": null,
                "tuesday": null,
                "wednesday": null,
                "thursday": null,
                "friday": null,
                "saturday": null,
                "sunday": null
            },
            "images": [],
            "status": "open"
        }))
        .send()
        .await
        .expect("create lot request");

    let status = resp.status();
    let body: Value = resp.json().await.expect("parse create lot json");
    assert!(
        status.is_success() || status.as_u16() == 201,
        "create lot failed: {} body: {}",
        status,
        body
    );
    body["data"]["id"]
        .as_str()
        .expect("lot id in response")
        .to_string()
}

/// Create a parking slot in a lot (requires admin token).  Returns `slot_id`.
pub async fn create_test_slot(
    srv: &TestServer,
    token: &str,
    lot_id: &str,
    slot_number: i32,
) -> String {
    let resp = srv
        .client
        .post(format!("{}/api/v1/lots/{}/slots", srv.url, lot_id))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "slot_number": slot_number,
            "floor_name": "Ground",
            "slot_type": "standard",
            "features": [],
            "row": 1,
            "column": slot_number,
        }))
        .send()
        .await
        .expect("create slot request");

    let status = resp.status();
    let body: Value = resp.json().await.expect("parse create slot json");
    assert!(
        status.is_success() || status.as_u16() == 201,
        "create slot failed: {} body: {}",
        status,
        body
    );
    body["data"]["id"]
        .as_str()
        .expect("slot id in response")
        .to_string()
}

/// Create a booking (requires user token).  Returns `booking_id`.
pub async fn create_test_booking(
    srv: &TestServer,
    token: &str,
    lot_id: &str,
    slot_id: &str,
) -> String {
    let start = chrono::Utc::now() + chrono::Duration::hours(1);

    let resp = srv
        .client
        .post(format!("{}/api/v1/bookings", srv.url))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "start_time": start.to_rfc3339(),
            "duration_minutes": 120,
            "vehicle_id": "00000000-0000-0000-0000-000000000000",
            "license_plate": "M-PH 1234",
            "notes": null
        }))
        .send()
        .await
        .expect("create booking request");

    let status = resp.status();
    let body: Value = resp.json().await.expect("parse booking json");
    assert!(
        status.is_success() || status.as_u16() == 201,
        "create booking failed: {} body: {}",
        status,
        body
    );
    body["data"]["id"]
        .as_str()
        .expect("booking id in response")
        .to_string()
}

/// Login with specific credentials. Returns full response JSON.
pub async fn login(srv: &TestServer, username: &str, password: &str) -> (u16, Value) {
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .json(&serde_json::json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await
        .expect("login request");

    let status = resp.status().as_u16();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    (status, body)
}

/// Make an authenticated GET request and return (status, json).
pub async fn auth_get(srv: &TestServer, token: &str, path: &str) -> (u16, Value) {
    let resp = srv
        .client
        .get(format!("{}{}", srv.url, path))
        .bearer_auth(token)
        .send()
        .await
        .expect("auth GET request");

    let status = resp.status().as_u16();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    (status, body)
}

/// Make an authenticated POST request and return (status, json).
pub async fn auth_post(srv: &TestServer, token: &str, path: &str, body: &Value) -> (u16, Value) {
    let resp = srv
        .client
        .post(format!("{}{}", srv.url, path))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .expect("auth POST request");

    let status = resp.status().as_u16();
    let json: Value = resp.json().await.unwrap_or(Value::Null);
    (status, json)
}

/// Make an authenticated PUT request and return (status, json).
pub async fn auth_put(srv: &TestServer, token: &str, path: &str, body: &Value) -> (u16, Value) {
    let resp = srv
        .client
        .put(format!("{}{}", srv.url, path))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .expect("auth PUT request");

    let status = resp.status().as_u16();
    let json: Value = resp.json().await.unwrap_or(Value::Null);
    (status, json)
}

/// Make an authenticated DELETE request and return (status, json).
pub async fn auth_delete(srv: &TestServer, token: &str, path: &str) -> (u16, Value) {
    let resp = srv
        .client
        .delete(format!("{}{}", srv.url, path))
        .bearer_auth(token)
        .send()
        .await
        .expect("auth DELETE request");

    let status = resp.status().as_u16();
    let json: Value = resp.json().await.unwrap_or(Value::Null);
    (status, json)
}
