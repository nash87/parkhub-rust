//! Server Connection
//!
//! Handles HTTP API communication with the ParkHub server.

use anyhow::{Context, Result};
use reqwest::Client;
use std::sync::Arc;

use parkhub_common::{
    ApiResponse, AuthTokens, Booking, CreateBookingRequest, HandshakeRequest, HandshakeResponse,
    LoginRequest, LoginResponse, ParkingLot, ParkingSlot, ServerInfo, User, PROTOCOL_VERSION,
};

/// Connection to a ParkHub server
pub struct ServerConnection {
    client: Client,
    base_url: String,
    server_info: ServerInfo,
    auth_tokens: Option<AuthTokens>,
}

impl ServerConnection {
    /// Connect to a server
    pub async fn connect(server_info: ServerInfo) -> Result<Self> {
        let scheme = if server_info.tls { "https" } else { "http" };
        let base_url = format!("{}://{}:{}", scheme, server_info.host, server_info.port);

        // Build HTTP client
        let client = Client::builder()
            .danger_accept_invalid_certs(true) // TODO: Proper cert validation
            .build()
            .context("Failed to create HTTP client")?;

        let mut conn = Self {
            client,
            base_url,
            server_info,
            auth_tokens: None,
        };

        // Perform handshake
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

        let login_response = response
            .data
            .ok_or_else(|| anyhow::anyhow!("Login failed: {:?}", response.error))?;

        self.auth_tokens = Some(login_response.tokens);
        Ok(login_response.user)
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
}
