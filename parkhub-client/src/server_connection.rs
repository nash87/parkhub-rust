//! Server Connection
//!
//! Handles HTTP API communication with the ParkHub server.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use parkhub_common::{
    models::UserPreferences, ApiResponse, AuthTokens, Booking, CreateBookingRequest,
    HandshakeRequest, HandshakeResponse, LoginRequest, LoginResponse, PaginatedResponse,
    ParkingLot, ParkingSlot, RegisterRequest, ServerInfo, User, UserRole, PROTOCOL_VERSION,
};

/// Connection to a ParkHub server
pub struct ServerConnection {
    client: Client,
    base_url: String,
    server_info: ServerInfo,
    auth_tokens: Option<AuthTokens>,
}

#[derive(Debug, Deserialize)]
struct AdminUserRecord {
    id: String,
    username: String,
    email: String,
    name: String,
    role: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct DataImportError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct DataImportResult {
    imported: usize,
    skipped: usize,
    errors: Vec<DataImportError>,
}

fn parse_admin_role(role: &str) -> UserRole {
    match role.to_ascii_lowercase().as_str() {
        "premium" => UserRole::Premium,
        "admin" => UserRole::Admin,
        "superadmin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    }
}

impl From<AdminUserRecord> for User {
    fn from(value: AdminUserRecord) -> Self {
        Self {
            id: uuid::Uuid::parse_str(&value.id).unwrap_or_else(|_| uuid::Uuid::nil()),
            username: value.username,
            email: value.email,
            name: value.name,
            password_hash: String::new(),
            role: parse_admin_role(&value.role),
            is_active: value.is_active,
            phone: None,
            picture: None,
            preferences: UserPreferences::default(),
            credits_balance: 0,
            credits_monthly_quota: 0,
            credits_last_refilled: None,
            created_at: value.created_at,
            updated_at: value.created_at,
            last_login: None,
            tenant_id: None,
            accessibility_needs: None,
            cost_center: None,
            department: None,
        }
    }
}

impl ServerConnection {
    /// Connect to a server
    // NOTE: Uses danger_accept_invalid_certs for self-signed server certificates.
    // For production use, call connect_with_cert() with the server's CA certificate.
    pub async fn connect(server_info: ServerInfo) -> Result<Self> {
        let scheme = if server_info.tls { "https" } else { "http" };
        let base_url = format!("{}://{}:{}", scheme, server_info.host, server_info.port);

        // Build HTTP client
        // For LAN connections to self-signed certs, accept any cert by default.
        // In production, provide a CA cert via connect_with_cert() instead.
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to create HTTP client")?;

        let conn = Self {
            client,
            base_url,
            server_info,
            auth_tokens: None,
        };

        // Perform handshake
        conn.handshake().await?;

        Ok(conn)
    }

    /// Connect to a server with a custom CA certificate (for self-signed certs).
    /// This is more secure than accepting any certificate.
    pub async fn connect_with_cert(server_info: ServerInfo, ca_cert_pem: &[u8]) -> Result<Self> {
        let scheme = if server_info.tls { "https" } else { "http" };
        let base_url = format!("{}://{}:{}", scheme, server_info.host, server_info.port);

        let cert =
            reqwest::Certificate::from_pem(ca_cert_pem).context("Invalid CA certificate PEM")?;

        let client = Client::builder()
            .add_root_certificate(cert)
            .build()
            .context("Failed to create HTTP client with custom cert")?;

        let conn = Self {
            client,
            base_url,
            server_info,
            auth_tokens: None,
        };

        conn.handshake().await?;
        Ok(conn)
    }

    /// Perform protocol handshake
    async fn handshake(&self) -> Result<HandshakeResponse> {
        let request = HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION.to_string(),
        };

        let response: ApiResponse<HandshakeResponse> = self
            .client
            .post(format!("{}/handshake", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to connect to server")?
            .json()
            .await
            .context("Invalid handshake response")?;

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("Handshake failed: {:?}", response.error))
    }

    /// Login with username and password
    pub async fn login(&mut self, username: &str, password: &str) -> Result<User> {
        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response: ApiResponse<LoginResponse> = self
            .client
            .post(format!("{}/api/v1/auth/login", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Login request failed")?
            .json()
            .await
            .context("Invalid login response")?;

        let login_response = response.data.ok_or_else(|| {
            let error_msg = response
                .error
                .map_or_else(|| "Login failed".to_string(), |e| e.message);
            anyhow::anyhow!(error_msg)
        })?;

        self.auth_tokens = Some(login_response.tokens);
        Ok(login_response.user)
    }

    /// Register a new user
    pub async fn register(
        &mut self,
        _username: &str,
        password: &str,
        email: &str,
        name: &str,
    ) -> Result<User> {
        let request = RegisterRequest {
            email: email.to_string(),
            password: password.to_string(),
            password_confirmation: password.to_string(),
            name: name.to_string(),
        };

        let response: ApiResponse<LoginResponse> = self
            .client
            .post(format!("{}/api/v1/auth/register", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Registration request failed")?
            .json()
            .await
            .context("Invalid registration response")?;

        let login_response = response.data.ok_or_else(|| {
            let error_msg = response
                .error
                .map_or_else(|| "Registration failed".to_string(), |e| e.message);
            anyhow::anyhow!(error_msg)
        })?;

        self.auth_tokens = Some(login_response.tokens);
        Ok(login_response.user)
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get authorization header
    fn auth_header(&self) -> Option<String> {
        self.auth_tokens
            .as_ref()
            .map(|t| format!("Bearer {}", t.access_token))
    }

    /// Get current user
    pub async fn get_current_user(&self) -> Result<User> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/users/me", self.base_url));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<User> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("Failed: {:?}", response.error))
    }

    /// List parking lots
    pub async fn list_lots(&self) -> Result<Vec<ParkingLot>> {
        let mut request = self.client.get(format!("{}/api/v1/lots", self.base_url));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<Vec<ParkingLot>> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        Ok(response.data.unwrap_or_default())
    }

    /// Get slots for a parking lot
    pub async fn get_lot_slots(&self, lot_id: &str) -> Result<Vec<ParkingSlot>> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/lots/{}/slots", self.base_url, lot_id));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<Vec<ParkingSlot>> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        Ok(response.data.unwrap_or_default())
    }

    /// List bookings
    pub async fn list_bookings(&self) -> Result<Vec<Booking>> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/bookings", self.base_url));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<Vec<Booking>> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        Ok(response.data.unwrap_or_default())
    }

    /// Create a booking
    pub async fn create_booking(&self, request: CreateBookingRequest) -> Result<Booking> {
        let mut req = self
            .client
            .post(format!("{}/api/v1/bookings", self.base_url))
            .json(&request);

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let response: ApiResponse<Booking> = req
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("Failed: {:?}", response.error))
    }

    /// Cancel a booking
    pub async fn cancel_booking(&self, booking_id: &str) -> Result<()> {
        let mut request = self
            .client
            .delete(format!("{}/api/v1/bookings/{}", self.base_url, booking_id));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<()> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        if response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed: {:?}", response.error))
        }
    }

    // ==================== ADMIN: User Management ====================

    /// List all users (admin only)
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let mut request = self.client.get(format!(
            "{}/api/v1/admin/users?page=1&per_page=1000",
            self.base_url
        ));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<PaginatedResponse<AdminUserRecord>> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        Ok(response
            .data
            .map(|page| page.items.into_iter().map(User::from).collect())
            .unwrap_or_default())
    }

    /// Get a specific user (admin only)
    pub async fn get_user(&self, user_id: &str) -> Result<User> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/users/{}", self.base_url, user_id));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<User> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("User not found: {:?}", response.error))
    }

    /// Update a user (admin only)
    pub async fn update_user(&self, user_id: &str, updates: serde_json::Value) -> Result<()> {
        let mut request = self
            .client
            .put(format!(
                "{}/api/v1/admin/users/{}/update",
                self.base_url, user_id
            ))
            .json(&updates);

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<serde_json::Value> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        if response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Update failed: {:?}", response.error))
        }
    }

    /// Create a user (admin only)
    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        name: &str,
        role: &str,
        password: &str,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "format": "json",
            "data": serde_json::to_string(&vec![serde_json::json!({
                "username": username,
                "email": email,
                "name": name,
                "role": role,
                "password": password,
            })]).context("Failed to encode user import payload")?,
        });

        let mut request = self
            .client
            .post(format!("{}/api/v1/admin/import/users", self.base_url))
            .json(&payload);

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<DataImportResult> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        if !response.success {
            return Err(anyhow::anyhow!("Create failed: {:?}", response.error));
        }

        let result = response
            .data
            .ok_or_else(|| anyhow::anyhow!("Create failed: missing import result"))?;

        if result.imported == 1 {
            return Ok(());
        }

        if let Some(first_error) = result.errors.first() {
            return Err(anyhow::anyhow!(first_error.message.clone()));
        }

        if result.skipped > 0 {
            return Err(anyhow::anyhow!(
                "User already exists or was skipped by import validation"
            ));
        }

        Err(anyhow::anyhow!("User creation did not import any records"))
    }

    /// Delete a user (admin only)
    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        let mut request = self
            .client
            .delete(format!("{}/api/v1/admin/users/{}", self.base_url, user_id));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<()> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        if response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Delete failed: {:?}", response.error))
        }
    }

    /// Reset user password (admin only)
    pub async fn reset_user_password(&self, user_id: &str, new_password: &str) -> Result<()> {
        let mut request = self
            .client
            .post(format!(
                "{}/api/v1/admin/users/{}/reset-password",
                self.base_url, user_id
            ))
            .json(&serde_json::json!({ "new_password": new_password }));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<()> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        if response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Password reset failed: {:?}",
                response.error
            ))
        }
    }

    // ==================== ADMIN: Server Config ====================

    /// Get server configuration (admin only)
    pub async fn get_server_config(&self) -> Result<serde_json::Value> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/admin/config", self.base_url));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<serde_json::Value> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("Failed to get config: {:?}", response.error))
    }

    /// Update server configuration (admin only)
    pub async fn update_server_config(&self, updates: serde_json::Value) -> Result<()> {
        let mut request = self
            .client
            .patch(format!("{}/api/v1/admin/config", self.base_url))
            .json(&updates);

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<()> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        if response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Config update failed: {:?}",
                response.error
            ))
        }
    }

    /// Get database statistics (admin only)
    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/admin/stats", self.base_url));

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response: ApiResponse<serde_json::Value> = request
            .send()
            .await
            .context("Request failed")?
            .json()
            .await
            .context("Invalid response")?;

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("Failed to get stats: {:?}", response.error))
    }
}
