//! Parking API Client
//!
//! Main HTTP client for communicating with the parking backend.

use reqwest::{header, Client};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::endpoints::paths;
use super::error::{ApiError, ApiResult};
use super::models::*;

/// Configuration for the API client
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.securanido.local".to_string(),
            timeout_secs: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Main API client for the parking system
pub struct ParkingApiClient {
    client: Client,
    config: ApiConfig,
    auth_tokens: Arc<RwLock<Option<AuthTokens>>>,
    current_user: Arc<RwLock<Option<User>>>,
}

impl ParkingApiClient {
    /// Create a new API client with the given configuration
    pub fn new(config: ApiConfig) -> ApiResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .build()
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        Ok(Self {
            client,
            config,
            auth_tokens: Arc::new(RwLock::new(None)),
            current_user: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a client with default configuration
    pub fn with_defaults() -> ApiResult<Self> {
        Self::new(ApiConfig::default())
    }

    /// Get the full URL for an endpoint
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url, path)
    }

    /// Get authorization header if authenticated
    async fn auth_header(&self) -> Option<String> {
        let tokens = self.auth_tokens.read().await;
        tokens
            .as_ref()
            .map(|t| format!("Bearer {}", t.access_token))
    }

    /// Check if the client is authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.auth_tokens.read().await.is_some()
    }

    /// Get current user
    pub async fn current_user(&self) -> Option<User> {
        self.current_user.read().await.clone()
    }

    /// Set authentication tokens
    pub async fn set_tokens(&self, tokens: AuthTokens) {
        *self.auth_tokens.write().await = Some(tokens);
    }

    /// Clear authentication
    pub async fn clear_auth(&self) {
        *self.auth_tokens.write().await = None;
        *self.current_user.write().await = None;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AUTHENTICATION METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Login with OAuth provider
    pub async fn login(&self, provider: &str, token: &str) -> ApiResult<LoginResponse> {
        let request = LoginRequest {
            provider: provider.to_string(),
            token: token.to_string(),
        };

        let response = self
            .client
            .post(self.url(&paths::auth_login()))
            .json(&request)
            .send()
            .await?;

        let result = self.handle_response::<LoginResponse>(response).await?;

        // Store tokens and user
        *self.auth_tokens.write().await = Some(result.tokens.clone());
        *self.current_user.write().await = Some(result.user.clone());

        info!("User logged in: {}", result.user.email);
        Ok(result)
    }

    /// Logout current user
    pub async fn logout(&self) -> ApiResult<()> {
        if let Some(auth) = self.auth_header().await {
            let _ = self
                .client
                .post(self.url(&paths::auth_logout()))
                .header(header::AUTHORIZATION, auth)
                .send()
                .await;
        }

        self.clear_auth().await;
        info!("User logged out");
        Ok(())
    }

    /// Refresh access token
    pub async fn refresh_token(&self) -> ApiResult<AuthTokens> {
        let tokens = self.auth_tokens.read().await;
        let refresh_token = tokens
            .as_ref()
            .map(|t| t.refresh_token.clone())
            .ok_or(ApiError::Unauthorized)?;
        drop(tokens);

        let response = self
            .client
            .post(self.url(&paths::auth_refresh()))
            .json(&serde_json::json!({ "refresh_token": refresh_token }))
            .send()
            .await?;

        let new_tokens = self.handle_response::<AuthTokens>(response).await?;
        *self.auth_tokens.write().await = Some(new_tokens.clone());

        debug!("Token refreshed");
        Ok(new_tokens)
    }

    /// Get current authenticated user info
    pub async fn get_me(&self) -> ApiResult<User> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::auth_me()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        let user = self.handle_response::<User>(response).await?;
        *self.current_user.write().await = Some(user.clone());

        Ok(user)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // USER METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Update user preferences
    pub async fn update_preferences(
        &self,
        preferences: UserPreferences,
    ) -> ApiResult<UserPreferences> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .put(self.url(&paths::user_preferences()))
            .header(header::AUTHORIZATION, auth)
            .json(&preferences)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get user's saved vehicles
    pub async fn get_vehicles(&self) -> ApiResult<Vec<Vehicle>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::user_vehicles()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Add a new vehicle
    pub async fn add_vehicle(&self, vehicle: Vehicle) -> ApiResult<Vehicle> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::user_vehicles()))
            .header(header::AUTHORIZATION, auth)
            .json(&vehicle)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Delete a vehicle
    pub async fn delete_vehicle(&self, id: &str) -> ApiResult<()> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .delete(self.url(&paths::user_vehicle(id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_empty_response(response).await
    }

    /// Get user statistics
    pub async fn get_statistics(&self) -> ApiResult<UserStatistics> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::user_statistics()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PARKING LOT METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get all available parking lots
    pub async fn get_lots(&self) -> ApiResult<Vec<ParkingLot>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::lots()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get a specific parking lot by ID
    pub async fn get_lot(&self, lot_id: &str) -> ApiResult<ParkingLot> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::lot(lot_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get all slots for a parking lot
    pub async fn get_lot_slots(&self, lot_id: &str) -> ApiResult<Vec<ParkingSlot>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::lot_slots(lot_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get slots for a specific floor
    pub async fn get_floor_slots(
        &self,
        lot_id: &str,
        floor_id: &str,
    ) -> ApiResult<Vec<ParkingSlot>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::lot_slots_by_floor(lot_id, floor_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get real-time availability for a lot
    pub async fn get_availability(&self, lot_id: &str) -> ApiResult<ParkingLot> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::lot_availability(lot_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get pricing for a lot
    pub async fn get_pricing(&self, lot_id: &str) -> ApiResult<PricingInfo> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::lot_pricing(lot_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BOOKING METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a new booking
    pub async fn create_booking(&self, request: CreateBookingRequest) -> ApiResult<Booking> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::bookings()))
            .header(header::AUTHORIZATION, auth)
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get a specific booking
    pub async fn get_booking(&self, booking_id: &str) -> ApiResult<Booking> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::booking(booking_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get all active bookings for current user
    pub async fn get_active_bookings(&self) -> ApiResult<Vec<Booking>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::active_bookings()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get booking history with optional filters
    pub async fn get_booking_history(
        &self,
        filters: BookingFilters,
    ) -> ApiResult<PaginatedResponse<Booking>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let mut request = self
            .client
            .get(self.url(&paths::booking_history()))
            .header(header::AUTHORIZATION, auth);

        // Add query parameters for filters
        if let Some(status) = &filters.status {
            request = request.query(&[("status", format!("{:?}", status).to_lowercase())]);
        }
        if let Some(page) = filters.page {
            request = request.query(&[("page", page.to_string())]);
        }
        if let Some(per_page) = filters.per_page {
            request = request.query(&[("per_page", per_page.to_string())]);
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    /// Extend an existing booking
    pub async fn extend_booking(
        &self,
        booking_id: &str,
        additional_minutes: i32,
    ) -> ApiResult<Booking> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let request = ExtendBookingRequest { additional_minutes };

        let response = self
            .client
            .post(self.url(&paths::booking_extend(booking_id)))
            .header(header::AUTHORIZATION, auth)
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Cancel a booking
    pub async fn cancel_booking(&self, booking_id: &str) -> ApiResult<Booking> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::booking_cancel(booking_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Check in to a booking
    pub async fn checkin(&self, booking_id: &str) -> ApiResult<Booking> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::booking_checkin(booking_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Check out from a booking
    pub async fn checkout(&self, booking_id: &str) -> ApiResult<Booking> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::booking_checkout(booking_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get QR code for a booking
    pub async fn get_booking_qrcode(&self, booking_id: &str) -> ApiResult<String> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::booking_qrcode(booking_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        #[derive(serde::Deserialize)]
        struct QrResponse {
            qr_code: String,
        }

        let result: QrResponse = self.handle_response(response).await?;
        Ok(result.qr_code)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NOTIFICATION METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get user notifications
    pub async fn get_notifications(&self) -> ApiResult<Vec<Notification>> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .get(self.url(&paths::notifications()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Mark a notification as read
    pub async fn mark_notification_read(&self, notification_id: &str) -> ApiResult<()> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::notification_read(notification_id)))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_empty_response(response).await
    }

    /// Mark all notifications as read
    pub async fn mark_all_notifications_read(&self) -> ApiResult<()> {
        let auth = self.auth_header().await.ok_or(ApiError::Unauthorized)?;

        let response = self
            .client
            .post(self.url(&paths::notifications_read_all()))
            .header(header::AUTHORIZATION, auth)
            .send()
            .await?;

        self.handle_empty_response(response).await
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HEALTH CHECK
    // ═══════════════════════════════════════════════════════════════════════════

    /// Check if the API server is healthy
    pub async fn health_check(&self) -> ApiResult<bool> {
        let response = self.client.get(self.url(&paths::health())).send().await?;

        Ok(response.status().is_success())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RESPONSE HANDLING
    // ═══════════════════════════════════════════════════════════════════════════

    /// Handle API response and deserialize
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> ApiResult<T> {
        let status = response.status();

        if status.is_success() {
            let data = response.json::<ApiResponse<T>>().await?;
            if data.success {
                data.data
                    .ok_or(ApiError::Unknown("Empty response data".to_string()))
            } else {
                let error = data.error.unwrap_or(ApiErrorResponse {
                    code: "UNKNOWN".to_string(),
                    message: "Unknown error".to_string(),
                    details: None,
                });
                Err(self.map_error_code(&error.code, &error.message))
            }
        } else {
            self.handle_error_status(status.as_u16(), response).await
        }
    }

    /// Handle empty response
    async fn handle_empty_response(&self, response: reqwest::Response) -> ApiResult<()> {
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            self.handle_error_status(status.as_u16(), response).await
        }
    }

    /// Handle error status codes
    async fn handle_error_status<T>(
        &self,
        status: u16,
        response: reqwest::Response,
    ) -> ApiResult<T> {
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        match status {
            401 => Err(ApiError::Unauthorized),
            403 => Err(ApiError::Unauthorized),
            404 => Err(ApiError::NotFound(message)),
            409 => Err(ApiError::SlotUnavailable),
            422 => Err(ApiError::ValidationError(message)),
            429 => Err(ApiError::RateLimited { retry_after: 60 }),
            _ => Err(ApiError::ServerError { status, message }),
        }
    }

    /// Map error codes to ApiError
    fn map_error_code(&self, code: &str, message: &str) -> ApiError {
        match code {
            "UNAUTHORIZED" => ApiError::Unauthorized,
            "NOT_FOUND" => ApiError::NotFound(message.to_string()),
            "SLOT_UNAVAILABLE" => ApiError::SlotUnavailable,
            "BOOKING_LIMIT_REACHED" => ApiError::BookingLimitReached,
            "INVALID_BOOKING_TIME" => ApiError::InvalidBookingTime(message.to_string()),
            "PAYMENT_REQUIRED" => ApiError::PaymentRequired,
            "VALIDATION_ERROR" => ApiError::ValidationError(message.to_string()),
            _ => ApiError::ServerError {
                status: 400,
                message: message.to_string(),
            },
        }
    }
}

impl Clone for ParkingApiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            auth_tokens: self.auth_tokens.clone(),
            current_user: self.current_user.clone(),
        }
    }
}
