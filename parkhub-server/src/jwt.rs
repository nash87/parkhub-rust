//! JWT Authentication
//!
//! Provides stateless token-based authentication using JSON Web Tokens.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::error::AppError;

/// In-memory token revocation list.
///
/// Stores revoked JWT IDs (`jti`) so that even unexpired tokens can be
/// invalidated (e.g. on logout or password change). Callers share this via
/// `Arc<TokenRevocationList>`.
#[derive(Debug, Default)]
pub struct TokenRevocationList {
    revoked: Mutex<HashSet<String>>,
}

impl TokenRevocationList {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Revoke a token by its `jti` claim.
    pub fn revoke(&self, jti: &str) {
        if let Ok(mut set) = self.revoked.lock() {
            set.insert(jti.to_string());
        }
    }

    /// Returns `true` if the given `jti` has been revoked.
    pub fn is_revoked(&self, jti: &str) -> bool {
        self.revoked
            .lock()
            .map(|set| set.contains(jti))
            .unwrap_or(false)
    }
}

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
        // Generate a 256-bit (32-byte) cryptographically random secret and hex-encode it.
        // This provides ~128 bits of effective security vs. a UUID which has only 122 bits
        // of entropy and a fixed structure that reduces its unpredictability as a HMAC key.
        let secret = {
            let mut rng = rand::rng();
            let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
            hex::encode(bytes)
        };
        Self {
            secret,
            access_token_expiry_hours: 1,
            refresh_token_expiry_days: 7,
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
    /// JWT ID — unique per token, used for revocation
    pub jti: String,
}

/// Token type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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
            jti: Uuid::new_v4().to_string(),
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| AppError::InvalidInput(format!("Failed to create token: {e}")))?;

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
            jti: Uuid::new_v4().to_string(),
        };

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| AppError::InvalidInput(format!("Failed to create token: {e}")))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_expiry_hours * 3600,
        })
    }

    /// Validate a token and return the claims.
    ///
    /// Pass `Some(revocation_list)` to also check whether the token has been
    /// explicitly revoked via [`TokenRevocationList::revoke`].
    pub fn validate_token(
        &self,
        token: &str,
        revocation_list: Option<&TokenRevocationList>,
    ) -> Result<Claims, AppError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        // Check revocation list if provided
        if let Some(rl) = revocation_list {
            if rl.is_revoked(&token_data.claims.jti) {
                return Err(AppError::InvalidToken);
            }
        }

        Ok(token_data.claims)
    }

    /// Refresh tokens using a refresh token.
    ///
    /// Optionally checks the revocation list before issuing new tokens.
    pub fn refresh_tokens(
        &self,
        refresh_token: &str,
        revocation_list: Option<&TokenRevocationList>,
    ) -> Result<TokenPair, AppError> {
        let claims = self.validate_token(refresh_token, revocation_list)?;

        if claims.token_type != TokenType::Refresh {
            return Err(AppError::InvalidToken);
        }

        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::InvalidToken)?;

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

        // Validate token (optionally check revocation list if present)
        let revocation_list = parts.extensions.get::<Arc<TokenRevocationList>>();
        let claims =
            jwt_manager.validate_token(token, revocation_list.map(std::convert::AsRef::as_ref))?;

        // Ensure it's an access token
        if claims.token_type != TokenType::Access {
            return Err(AppError::InvalidToken);
        }

        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::InvalidToken)?;

        Ok(Self {
            user_id,
            username: claims.username,
            role: claims.role,
        })
    }
}

/// Optional authentication (for endpoints that work with or without auth)
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(AuthUser::from_request_parts(parts, state)
            .await
            .map_or(Self(None), |user| Self(Some(user))))
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
            .validate_token(&tokens.access_token, None)
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
            .refresh_tokens(&tokens.refresh_token, None)
            .expect("Failed to refresh tokens");

        assert!(!new_tokens.access_token.is_empty());
        // Tokens should be different after refresh (different iat timestamp)
        assert_ne!(new_tokens.access_token, tokens.access_token);
    }

    #[test]
    fn test_invalid_token() {
        let jwt = JwtManager::with_random_secret();
        let result = jwt.validate_token("invalid.token.here", None);
        assert!(result.is_err());
    }

    // ── JwtConfig defaults ──

    #[test]
    fn test_jwt_config_defaults() {
        let config = JwtConfig::default();
        assert_eq!(config.access_token_expiry_hours, 1);
        assert_eq!(config.refresh_token_expiry_days, 7);
        assert_eq!(config.issuer, "parkhub");
        // Secret should be 64 hex chars (32 bytes)
        assert_eq!(config.secret.len(), 64);
        assert!(config.secret.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_jwt_config_random_secret_is_unique() {
        let c1 = JwtConfig::default();
        let c2 = JwtConfig::default();
        assert_ne!(c1.secret, c2.secret);
    }

    // ── Token generation ──

    #[test]
    fn test_token_expires_in_matches_config() {
        let config = JwtConfig {
            secret: "test-secret-key-for-testing-only-1234".to_string(),
            access_token_expiry_hours: 12,
            refresh_token_expiry_days: 7,
            issuer: "test".to_string(),
        };
        let jwt = JwtManager::new(config);
        let tokens = jwt
            .generate_tokens(&Uuid::new_v4(), "user", "user")
            .unwrap();
        assert_eq!(tokens.expires_in, 12 * 3600);
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[test]
    fn test_access_token_has_correct_claims() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "admin").unwrap();
        let claims = jwt.validate_token(&tokens.access_token, None).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.iss, "parkhub");
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(claims.exp > claims.iat);
        assert!(!claims.jti.is_empty());
    }

    #[test]
    fn test_refresh_token_has_correct_type() {
        let jwt = JwtManager::with_random_secret();
        let tokens = jwt.generate_tokens(&Uuid::new_v4(), "bob", "user").unwrap();
        let claims = jwt.validate_token(&tokens.refresh_token, None).unwrap();
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_refresh_token_lives_longer_than_access() {
        let jwt = JwtManager::with_random_secret();
        let tokens = jwt
            .generate_tokens(&Uuid::new_v4(), "charlie", "user")
            .unwrap();
        let access = jwt.validate_token(&tokens.access_token, None).unwrap();
        let refresh = jwt.validate_token(&tokens.refresh_token, None).unwrap();
        assert!(refresh.exp > access.exp);
    }

    // ── Validation failures ──

    #[test]
    fn test_wrong_secret_rejects_token() {
        let jwt1 = JwtManager::with_random_secret();
        let jwt2 = JwtManager::with_random_secret();
        let tokens = jwt1
            .generate_tokens(&Uuid::new_v4(), "eve", "user")
            .unwrap();
        assert!(jwt2.validate_token(&tokens.access_token, None).is_err());
    }

    #[test]
    fn test_empty_string_is_invalid() {
        let jwt = JwtManager::with_random_secret();
        assert!(jwt.validate_token("", None).is_err());
    }

    #[test]
    fn test_refresh_with_access_token_fails() {
        let jwt = JwtManager::with_random_secret();
        let tokens = jwt
            .generate_tokens(&Uuid::new_v4(), "frank", "user")
            .unwrap();
        // Using access token for refresh should fail (wrong token_type)
        let result = jwt.refresh_tokens(&tokens.access_token, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token_rejected() {
        let config = JwtConfig {
            secret: "expired-test-secret-key-for-testing".to_string(),
            access_token_expiry_hours: 0, // 0 hours = expires immediately
            refresh_token_expiry_days: 0,
            issuer: "parkhub".to_string(),
        };
        let jwt = JwtManager::new(config);
        let user_id = Uuid::new_v4();

        // Manually build an expired token
        let exp = Utc::now() - Duration::hours(1);
        let claims = Claims {
            sub: user_id.to_string(),
            username: "expired".to_string(),
            role: "user".to_string(),
            iat: (Utc::now() - Duration::hours(2)).timestamp(),
            exp: exp.timestamp(),
            iss: "parkhub".to_string(),
            token_type: TokenType::Access,
            jti: Uuid::new_v4().to_string(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"expired-test-secret-key-for-testing"),
        )
        .unwrap();

        let result = jwt.validate_token(&token, None);
        assert!(result.is_err());
    }

    // ── AuthUser ──

    #[test]
    fn test_auth_user_is_admin() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            username: "admin".to_string(),
            role: "admin".to_string(),
        };
        assert!(user.is_admin());
    }

    #[test]
    fn test_auth_user_superadmin_is_admin() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            username: "superadmin".to_string(),
            role: "superadmin".to_string(),
        };
        assert!(user.is_admin());
    }

    #[test]
    fn test_auth_user_regular_is_not_admin() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            username: "regular".to_string(),
            role: "user".to_string(),
        };
        assert!(!user.is_admin());
    }

    #[test]
    fn test_auth_user_empty_role_is_not_admin() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            username: "nobody".to_string(),
            role: "".to_string(),
        };
        assert!(!user.is_admin());
    }

    // ── TokenType serialization ──

    #[test]
    fn test_token_type_serialization() {
        assert_eq!(
            serde_json::to_string(&TokenType::Access).unwrap(),
            "\"access\""
        );
        assert_eq!(
            serde_json::to_string(&TokenType::Refresh).unwrap(),
            "\"refresh\""
        );
    }

    #[test]
    fn test_token_type_deserialization() {
        let a: TokenType = serde_json::from_str("\"access\"").unwrap();
        assert_eq!(a, TokenType::Access);
        let r: TokenType = serde_json::from_str("\"refresh\"").unwrap();
        assert_eq!(r, TokenType::Refresh);
    }

    // ── TokenPair serialization ──

    #[test]
    fn test_token_pair_serialization() {
        let pair = TokenPair {
            access_token: "abc".to_string(),
            refresh_token: "def".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 86400,
        };
        let json = serde_json::to_value(&pair).unwrap();
        assert_eq!(json["access_token"], "abc");
        assert_eq!(json["refresh_token"], "def");
        assert_eq!(json["token_type"], "Bearer");
        assert_eq!(json["expires_in"], 86400);
    }

    // ── TokenRevocationList ──

    #[test]
    fn test_revocation_list_rejects_revoked_token() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "user").unwrap();

        // Token is valid before revocation
        let rl = TokenRevocationList::new();
        assert!(jwt.validate_token(&tokens.access_token, Some(&rl)).is_ok());

        // Extract jti and revoke it
        let claims = jwt.validate_token(&tokens.access_token, None).unwrap();
        rl.revoke(&claims.jti);

        // Token should now be rejected
        assert!(jwt.validate_token(&tokens.access_token, Some(&rl)).is_err());
    }

    #[test]
    fn test_revocation_list_does_not_affect_other_tokens() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let t1 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let t2 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();

        let rl = TokenRevocationList::new();
        let claims1 = jwt.validate_token(&t1.access_token, None).unwrap();
        rl.revoke(&claims1.jti);

        // t1 revoked, t2 still valid
        assert!(jwt.validate_token(&t1.access_token, Some(&rl)).is_err());
        assert!(jwt.validate_token(&t2.access_token, Some(&rl)).is_ok());
    }

    #[test]
    fn test_jti_unique_per_token() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let t1 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let t2 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let c1 = jwt.validate_token(&t1.access_token, None).unwrap();
        let c2 = jwt.validate_token(&t2.access_token, None).unwrap();
        assert_ne!(c1.jti, c2.jti);
    }
}
