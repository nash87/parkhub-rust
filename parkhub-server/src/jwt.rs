//! JWT Authentication
//!
//! Provides stateless token-based authentication using JSON Web Tokens.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    RequestPartsExt,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// JWT configuration
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,
    /// Access token expiration in hours
    pub access_token_expiry_hours: i64,
    /// Refresh token expiration in days
    pub refresh_token_expiry_days: i64,
    /// Token issuer
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: Uuid::new_v4().to_string(), // Generate random secret
            access_token_expiry_hours: 24,
            refresh_token_expiry_days: 30,
            issuer: "parkhub".to_string(),
        }
    }
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Issuer
    pub iss: String,
    /// Token type (access/refresh)
    pub token_type: TokenType,
}

/// Token type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

/// Token pair (access + refresh)
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// JWT Manager for creating and validating tokens
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    /// Create a new JWT manager with the given config
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Create a new JWT manager with a random secret
    pub fn with_random_secret() -> Self {
        Self::new(JwtConfig::default())
    }

    /// Generate a token pair for a user
    pub fn generate_tokens(
        &self,
        user_id: &Uuid,
        username: &str,
        role: &str,
    ) -> Result<TokenPair, AppError> {
        let now = Utc::now();

        // Access token
        let access_exp = now + Duration::hours(self.config.access_token_expiry_hours);
        let access_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            iat: now.timestamp(),
            exp: access_exp.timestamp(),
            iss: self.config.issuer.clone(),
            token_type: TokenType::Access,
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| AppError::InvalidInput(format!("Failed to create token: {}", e)))?;

        // Refresh token
        let refresh_exp = now + Duration::days(self.config.refresh_token_expiry_days);
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
            iss: self.config.issuer.clone(),
            token_type: TokenType::Refresh,
        };

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| AppError::InvalidInput(format!("Failed to create token: {}", e)))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_expiry_hours * 3600,
        })
    }

    /// Validate a token and return the claims
    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        Ok(token_data.claims)
    }

    /// Refresh tokens using a refresh token
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair, AppError> {
        let claims = self.validate_token(refresh_token)?;

        if claims.token_type != TokenType::Refresh {
            return Err(AppError::InvalidToken);
        }

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::InvalidToken)?;

        self.generate_tokens(&user_id, &claims.username, &claims.role)
    }
}

/// Authenticated user extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
}

impl AuthUser {
    /// Check if user has admin role
    pub fn is_admin(&self) -> bool {
        self.role == "admin" || self.role == "superadmin"
    }
}

/// Extractor for authenticated requests
#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        // Extract bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::InvalidToken)?;

        // Get JWT manager from extensions
        let jwt_manager = parts
            .extensions
            .get::<JwtManager>()
            .ok_or(AppError::Internal)?;

        // Validate token
        let claims = jwt_manager.validate_token(token)?;

        // Ensure it's an access token
        if claims.token_type != TokenType::Access {
            return Err(AppError::InvalidToken);
        }

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::InvalidToken)?;

        Ok(AuthUser {
            user_id,
            username: claims.username,
            role: claims.role,
        })
    }
}

/// Optional authentication (for endpoints that work with or without auth)
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuthUser(Some(user))),
            Err(_) => Ok(OptionalAuthUser(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_tokens() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();

        let tokens = jwt
            .generate_tokens(&user_id, "testuser", "user")
            .expect("Failed to generate tokens");

        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());
        assert_eq!(tokens.token_type, "Bearer");

        // Validate access token
        let claims = jwt
            .validate_token(&tokens.access_token)
            .expect("Failed to validate token");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "user");
        assert_eq!(claims.token_type, TokenType::Access);
    }

    #[test]
    fn test_refresh_tokens() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();

        let tokens = jwt
            .generate_tokens(&user_id, "testuser", "admin")
            .expect("Failed to generate tokens");

        // Wait 1 second so tokens have different iat/exp
        std::thread::sleep(std::time::Duration::from_secs(1));

        let new_tokens = jwt
            .refresh_tokens(&tokens.refresh_token)
            .expect("Failed to refresh tokens");

        assert!(!new_tokens.access_token.is_empty());
        // Tokens should be different after refresh (different iat timestamp)
        assert_ne!(new_tokens.access_token, tokens.access_token);
    }

    #[test]
    fn test_invalid_token() {
        let jwt = JwtManager::with_random_secret();
        let result = jwt.validate_token("invalid.token.here");
        assert!(result.is_err());
    }
}
