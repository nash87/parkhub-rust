//! JWT Authentication
//!
//! Provides stateless token-based authentication using JSON Web Tokens.
//!
//! Revocation is pluggable via the [`RevocationStore`] trait. Two concrete
//! implementations are provided:
//!
//! * [`InMemoryRevocationList`] — default. Process-local `HashMap` keyed by
//!   JWT `jti`. Fine for single-replica deploys; state is lost on restart.
//! * [`RedisRevocationList`] — behind the `redis-revocation` cargo feature.
//!   Persists revocations in Redis so logouts and theft-response family
//!   revocations survive pod restarts and propagate across replicas.
//!
//! Refresh tokens additionally carry a `family_id` claim. When a refresh
//! token whose `jti` has already been revoked is replayed, we treat it as
//! a theft attempt and revoke the entire family — every child access and
//! refresh token issued from the same login is immediately invalidated.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant};
use uuid::Uuid;

use crate::error::AppError;

/// Maximum age of a revocation entry before it is garbage-collected.
/// Entries older than this are pruned on the next `is_revoked` call.
///
/// Kept at the refresh-token lifetime so a revoked refresh token stays
/// revoked for its entire remaining life.
const REVOCATION_TTL: StdDuration = StdDuration::from_secs(7 * 24 * 3600);

/// Pluggable backend for JWT revocation state.
///
/// All methods are async: the Redis implementation must issue network I/O,
/// and the in-memory implementation is cheap enough that awaiting it is free.
/// Implementors MUST be cheap to clone (they are typically shared via
/// `Arc<dyn RevocationStore>`).
#[async_trait::async_trait]
pub trait RevocationStore: Send + Sync + std::fmt::Debug {
    /// Record that the token identified by `jti` is revoked.
    async fn revoke(&self, jti: &str);

    /// Returns `true` if the given `jti` has been revoked.
    async fn is_revoked(&self, jti: &str) -> bool;

    /// Record that every token in the refresh-token family `family_id` is
    /// revoked. Used when a replayed-after-rotation refresh token is detected,
    /// i.e. a theft response.
    async fn revoke_family(&self, family_id: &str);

    /// Returns `true` if the given `family_id` has been family-revoked.
    async fn is_family_revoked(&self, family_id: &str) -> bool;
}

/// In-memory revocation backend.
///
/// Stores revoked JWT IDs (`jti`) and revoked family IDs alongside the
/// [`Instant`] they were inserted, so stale entries are pruned on access
/// and memory is bounded.
#[derive(Debug, Default)]
pub struct InMemoryRevocationList {
    revoked: Mutex<HashMap<String, Instant>>,
    revoked_families: Mutex<HashMap<String, Instant>>,
}

impl InMemoryRevocationList {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait::async_trait]
impl RevocationStore for InMemoryRevocationList {
    async fn revoke(&self, jti: &str) {
        if let Ok(mut map) = self.revoked.lock() {
            map.insert(jti.to_string(), Instant::now());
        }
    }

    async fn is_revoked(&self, jti: &str) -> bool {
        self.revoked
            .lock()
            .map(|mut map| {
                map.retain(|_, ts| ts.elapsed() < REVOCATION_TTL);
                map.contains_key(jti)
            })
            .unwrap_or(false)
    }

    async fn revoke_family(&self, family_id: &str) {
        if let Ok(mut map) = self.revoked_families.lock() {
            map.insert(family_id.to_string(), Instant::now());
        }
    }

    async fn is_family_revoked(&self, family_id: &str) -> bool {
        self.revoked_families
            .lock()
            .map(|mut map| {
                map.retain(|_, ts| ts.elapsed() < REVOCATION_TTL);
                map.contains_key(family_id)
            })
            .unwrap_or(false)
    }
}

/// Redis-backed revocation store.
///
/// Uses two key shapes:
///
/// * `jwt_revoked:{jti}` — set via `SETEX` with the refresh-token TTL so the
///   entry self-cleans when the token would have expired anyway.
/// * `jwt_family_revoked:{family_id}` — same shape, marks every refresh/access
///   token derived from the same login as revoked.
///
/// Connection errors during `revoke` are logged but do not propagate — the
/// caller is in the middle of logging a user out and we don't want to tie
/// request latency to transient Redis blips. For `is_revoked` we fail-safe
/// **closed** (return `true`) so a Redis outage locks out users rather than
/// silently allowing replayed tokens.
#[cfg(feature = "redis-revocation")]
#[derive(Clone)]
pub struct RedisRevocationList {
    manager: redis::aio::ConnectionManager,
    ttl_secs: u64,
}

#[cfg(feature = "redis-revocation")]
impl std::fmt::Debug for RedisRevocationList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `ConnectionManager` does not implement Debug, so we intentionally
        // elide it and mark the struct as non-exhaustive.
        f.debug_struct("RedisRevocationList")
            .field("ttl_secs", &self.ttl_secs)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "redis-revocation")]
impl RedisRevocationList {
    /// Open a Redis connection manager against `url`. The manager transparently
    /// reconnects on failure.
    pub async fn connect(url: &str, ttl_secs: u64) -> Result<Arc<Self>, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let manager = redis::aio::ConnectionManager::new(client).await?;
        Ok(Arc::new(Self { manager, ttl_secs }))
    }

    /// Construct from environment. Panics with a clear message when the
    /// `redis-revocation` feature is on but `PARKHUB_REDIS_URL` is unset,
    /// so a misconfigured production deploy fails loudly at startup.
    pub async fn from_env() -> Arc<Self> {
        let url = std::env::var("PARKHUB_REDIS_URL").unwrap_or_else(|_| {
            panic!(
                "PARKHUB_REDIS_URL is not set but the `redis-revocation` \
                 feature is enabled. Either set PARKHUB_REDIS_URL=redis://host:6379 \
                 or rebuild without --features redis-revocation."
            )
        });
        let ttl = 7 * 24 * 3600;
        Self::connect(&url, ttl)
            .await
            .unwrap_or_else(|e| panic!("failed to connect to PARKHUB_REDIS_URL: {e}"))
    }
}

#[cfg(feature = "redis-revocation")]
#[async_trait::async_trait]
impl RevocationStore for RedisRevocationList {
    async fn revoke(&self, jti: &str) {
        use redis::AsyncCommands;
        let mut conn = self.manager.clone();
        let key = format!("jwt_revoked:{jti}");
        let res: redis::RedisResult<()> = conn.set_ex(key, "1", self.ttl_secs).await;
        if let Err(e) = res {
            tracing::warn!(error = %e, jti = %jti, "redis revoke SETEX failed");
        }
    }

    async fn is_revoked(&self, jti: &str) -> bool {
        use redis::AsyncCommands;
        let mut conn = self.manager.clone();
        let key = format!("jwt_revoked:{jti}");
        match conn.exists::<_, bool>(key).await {
            Ok(exists) => exists,
            Err(e) => {
                // Fail closed — treat Redis outages as "revoked" so we never
                // accept a token we couldn't verify against the shared list.
                tracing::error!(error = %e, jti = %jti, "redis EXISTS failed — failing closed");
                true
            }
        }
    }

    async fn revoke_family(&self, family_id: &str) {
        use redis::AsyncCommands;
        let mut conn = self.manager.clone();
        let key = format!("jwt_family_revoked:{family_id}");
        let res: redis::RedisResult<()> = conn.set_ex(key, "1", self.ttl_secs).await;
        if let Err(e) = res {
            tracing::warn!(error = %e, family_id = %family_id, "redis revoke_family SETEX failed");
        }
    }

    async fn is_family_revoked(&self, family_id: &str) -> bool {
        use redis::AsyncCommands;
        let mut conn = self.manager.clone();
        let key = format!("jwt_family_revoked:{family_id}");
        match conn.exists::<_, bool>(key).await {
            Ok(exists) => exists,
            Err(e) => {
                tracing::error!(error = %e, family_id = %family_id, "redis family EXISTS failed — failing closed");
                true
            }
        }
    }
}

/// Thin public facade over an `Arc<dyn RevocationStore>`.
///
/// Callers continue to pull an `Arc<TokenRevocationList>` out of axum
/// extensions exactly like before — the backend (in-memory vs Redis) is
/// chosen once at startup and hidden behind this wrapper.
#[derive(Clone, Debug)]
pub struct TokenRevocationList {
    inner: Arc<dyn RevocationStore>,
}

impl Default for TokenRevocationList {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl TokenRevocationList {
    /// Build a new in-memory-backed revocation list. Equivalent to the
    /// legacy `TokenRevocationList::new()` factory — kept for callers that
    /// don't care which backend they get.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::in_memory())
    }

    /// Build a new in-memory-backed revocation list (explicit).
    pub fn in_memory() -> Self {
        Self {
            inner: Arc::new(InMemoryRevocationList::default()),
        }
    }

    /// Wrap an already-constructed backend.
    pub fn from_store(store: Arc<dyn RevocationStore>) -> Arc<Self> {
        Arc::new(Self { inner: store })
    }

    pub async fn revoke(&self, jti: &str) {
        self.inner.revoke(jti).await;
    }

    pub async fn is_revoked(&self, jti: &str) -> bool {
        self.inner.is_revoked(jti).await
    }

    pub async fn revoke_family(&self, family_id: &str) {
        self.inner.revoke_family(family_id).await;
    }

    pub async fn is_family_revoked(&self, family_id: &str) -> bool {
        self.inner.is_family_revoked(family_id).await
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
    /// Refresh-token family identifier.
    ///
    /// All access + refresh tokens descending from the same login share the
    /// same `family_id`. When a revoked refresh token is replayed, the whole
    /// family is marked revoked so every child token becomes invalid in a
    /// single call. `Option<_>` so pre-existing tokens (minted before T-1742)
    /// still parse cleanly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
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

    /// Generate a token pair for a user.
    ///
    /// A fresh `family_id` UUID is minted — both the access and refresh token
    /// carry it so a later theft-response can revoke the whole family.
    pub fn generate_tokens(
        &self,
        user_id: &Uuid,
        username: &str,
        role: &str,
    ) -> Result<TokenPair, AppError> {
        let family_id = Uuid::new_v4().to_string();
        self.generate_tokens_in_family(user_id, username, role, &family_id)
    }

    /// Generate a token pair that continues an existing refresh-token family.
    ///
    /// Used during refresh rotation so the new access/refresh pair is still
    /// part of the same theft-response blast radius as the original login.
    pub fn generate_tokens_in_family(
        &self,
        user_id: &Uuid,
        username: &str,
        role: &str,
        family_id: &str,
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
            family_id: Some(family_id.to_string()),
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
            family_id: Some(family_id.to_string()),
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
    /// Pass `Some(revocation_list)` to also check whether the token — or its
    /// whole refresh-token family — has been revoked.
    pub async fn validate_token(
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
            if rl.is_revoked(&token_data.claims.jti).await {
                return Err(AppError::InvalidToken);
            }
            if let Some(family_id) = token_data.claims.family_id.as_deref()
                && rl.is_family_revoked(family_id).await
            {
                return Err(AppError::InvalidToken);
            }
        }

        Ok(token_data.claims)
    }

    /// Refresh tokens using a refresh token.
    ///
    /// Family-rotation semantics (RFC 6749 §10.4 mitigation): if the presented
    /// refresh token's `jti` is already revoked, the refresh token has been
    /// replayed after rotation — treat it as a theft attempt, revoke the
    /// entire family, and reject the request. Otherwise rotate: revoke the
    /// old `jti` and mint a new pair still bound to the same `family_id`.
    pub async fn refresh_tokens(
        &self,
        refresh_token: &str,
        revocation_list: Option<&TokenRevocationList>,
    ) -> Result<TokenPair, AppError> {
        // Decode without using the revocation list so we can distinguish
        // "already-consumed jti" (theft) from "never-seen jti" (happy path).
        // validate_token WITH the list would conflate those two cases — both
        // return `InvalidToken` — which would prevent the family revocation.
        let claims = self.validate_token(refresh_token, None).await?;

        if claims.token_type != TokenType::Refresh {
            return Err(AppError::InvalidToken);
        }

        if let Some(rl) = revocation_list {
            // If the whole family is already revoked, reject unconditionally.
            if let Some(family_id) = claims.family_id.as_deref()
                && rl.is_family_revoked(family_id).await
            {
                return Err(AppError::InvalidToken);
            }

            // Theft detection: this refresh token's jti is already in the
            // per-token list, so some OTHER actor already consumed it.
            // Burn the whole family.
            if rl.is_revoked(&claims.jti).await {
                if let Some(family_id) = claims.family_id.as_deref() {
                    rl.revoke_family(family_id).await;
                    tracing::warn!(
                        jti = %claims.jti,
                        family_id = %family_id,
                        user = %claims.username,
                        "refresh-token replay after rotation — family revoked"
                    );
                }
                return Err(AppError::InvalidToken);
            }
        }

        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::InvalidToken)?;

        // Rotate: revoke the consumed refresh token so any replay now triggers
        // the family-revocation path above.
        if let Some(rl) = revocation_list {
            rl.revoke(&claims.jti).await;
        }

        // Re-mint inside the same family (legacy pre-T-1742 tokens promote
        // into a fresh one).
        if let Some(family_id) = claims.family_id.as_deref() {
            self.generate_tokens_in_family(&user_id, &claims.username, &claims.role, family_id)
        } else {
            self.generate_tokens(&user_id, &claims.username, &claims.role)
        }
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
        let revocation_list = parts.extensions.get::<Arc<TokenRevocationList>>().cloned();
        let claims = jwt_manager
            .validate_token(token, revocation_list.as_deref())
            .await?;

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

    #[tokio::test]
    async fn test_generate_and_validate_tokens() {
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
            .await
            .expect("Failed to validate token");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "user");
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(claims.family_id.is_some());
    }

    #[tokio::test]
    async fn test_refresh_tokens() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();

        let tokens = jwt
            .generate_tokens(&user_id, "testuser", "admin")
            .expect("Failed to generate tokens");

        // Wait 1 second so tokens have different iat/exp
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let new_tokens = jwt
            .refresh_tokens(&tokens.refresh_token, None)
            .await
            .expect("Failed to refresh tokens");

        assert!(!new_tokens.access_token.is_empty());
        // Tokens should be different after refresh (different iat timestamp)
        assert_ne!(new_tokens.access_token, tokens.access_token);
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let jwt = JwtManager::with_random_secret();
        let result = jwt.validate_token("invalid.token.here", None).await;
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

    #[tokio::test]
    async fn test_token_expires_in_matches_config() {
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

    #[tokio::test]
    async fn test_access_token_has_correct_claims() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "admin").unwrap();
        let claims = jwt
            .validate_token(&tokens.access_token, None)
            .await
            .unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.iss, "parkhub");
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(claims.exp > claims.iat);
        assert!(!claims.jti.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_has_correct_type() {
        let jwt = JwtManager::with_random_secret();
        let tokens = jwt.generate_tokens(&Uuid::new_v4(), "bob", "user").unwrap();
        let claims = jwt
            .validate_token(&tokens.refresh_token, None)
            .await
            .unwrap();
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[tokio::test]
    async fn test_refresh_token_lives_longer_than_access() {
        let jwt = JwtManager::with_random_secret();
        let tokens = jwt
            .generate_tokens(&Uuid::new_v4(), "charlie", "user")
            .unwrap();
        let access = jwt
            .validate_token(&tokens.access_token, None)
            .await
            .unwrap();
        let refresh = jwt
            .validate_token(&tokens.refresh_token, None)
            .await
            .unwrap();
        assert!(refresh.exp > access.exp);
    }

    // ── Validation failures ──

    #[tokio::test]
    async fn test_wrong_secret_rejects_token() {
        let jwt1 = JwtManager::with_random_secret();
        let jwt2 = JwtManager::with_random_secret();
        let tokens = jwt1
            .generate_tokens(&Uuid::new_v4(), "eve", "user")
            .unwrap();
        assert!(
            jwt2.validate_token(&tokens.access_token, None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_empty_string_is_invalid() {
        let jwt = JwtManager::with_random_secret();
        assert!(jwt.validate_token("", None).await.is_err());
    }

    #[tokio::test]
    async fn test_refresh_with_access_token_fails() {
        let jwt = JwtManager::with_random_secret();
        let tokens = jwt
            .generate_tokens(&Uuid::new_v4(), "frank", "user")
            .unwrap();
        // Using access token for refresh should fail (wrong token_type)
        let result = jwt.refresh_tokens(&tokens.access_token, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_expired_token_rejected() {
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
            family_id: Some(Uuid::new_v4().to_string()),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"expired-test-secret-key-for-testing"),
        )
        .unwrap();

        let result = jwt.validate_token(&token, None).await;
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

    // ── TokenRevocationList (in-memory impl) ──

    #[tokio::test]
    async fn test_revocation_list_rejects_revoked_token() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "user").unwrap();

        // Token is valid before revocation
        let rl = TokenRevocationList::in_memory();
        assert!(
            jwt.validate_token(&tokens.access_token, Some(&rl))
                .await
                .is_ok()
        );

        // Extract jti and revoke it
        let claims = jwt
            .validate_token(&tokens.access_token, None)
            .await
            .unwrap();
        rl.revoke(&claims.jti).await;

        // Token should now be rejected
        assert!(
            jwt.validate_token(&tokens.access_token, Some(&rl))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_revocation_list_does_not_affect_other_tokens() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let t1 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let t2 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();

        let rl = TokenRevocationList::in_memory();
        let claims1 = jwt.validate_token(&t1.access_token, None).await.unwrap();
        rl.revoke(&claims1.jti).await;

        // t1 revoked, t2 still valid
        assert!(
            jwt.validate_token(&t1.access_token, Some(&rl))
                .await
                .is_err()
        );
        assert!(
            jwt.validate_token(&t2.access_token, Some(&rl))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_jti_unique_per_token() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let t1 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let t2 = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let c1 = jwt.validate_token(&t1.access_token, None).await.unwrap();
        let c2 = jwt.validate_token(&t2.access_token, None).await.unwrap();
        assert_ne!(c1.jti, c2.jti);
    }

    // ── Family rotation (T-1742) ──

    #[tokio::test]
    async fn test_tokens_share_family_id_within_a_login() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let access = jwt
            .validate_token(&tokens.access_token, None)
            .await
            .unwrap();
        let refresh = jwt
            .validate_token(&tokens.refresh_token, None)
            .await
            .unwrap();
        assert!(access.family_id.is_some());
        assert_eq!(access.family_id, refresh.family_id);
    }

    #[tokio::test]
    async fn test_refresh_preserves_family_id() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let original = jwt
            .validate_token(&tokens.refresh_token, None)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let rotated = jwt
            .refresh_tokens(&tokens.refresh_token, None)
            .await
            .unwrap();
        let rotated_claims = jwt
            .validate_token(&rotated.refresh_token, None)
            .await
            .unwrap();

        assert_eq!(original.family_id, rotated_claims.family_id);
        assert_ne!(original.jti, rotated_claims.jti);
    }

    #[tokio::test]
    async fn test_refresh_revokes_old_jti() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let rl = TokenRevocationList::in_memory();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let first_refresh_claims = jwt
            .validate_token(&tokens.refresh_token, None)
            .await
            .unwrap();
        let _ = jwt
            .refresh_tokens(&tokens.refresh_token, Some(&rl))
            .await
            .expect("first refresh rotates cleanly");

        // The consumed refresh jti is now revoked.
        assert!(rl.is_revoked(&first_refresh_claims.jti).await);
    }

    #[tokio::test]
    async fn test_refresh_replay_revokes_whole_family() {
        let jwt = JwtManager::with_random_secret();
        let user_id = Uuid::new_v4();
        let tokens = jwt.generate_tokens(&user_id, "alice", "user").unwrap();
        let rl = TokenRevocationList::in_memory();

        let original = jwt
            .validate_token(&tokens.refresh_token, None)
            .await
            .unwrap();
        let family_id = original.family_id.clone().unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Legitimate first refresh rotates.
        let rotated = jwt
            .refresh_tokens(&tokens.refresh_token, Some(&rl))
            .await
            .expect("first refresh ok");

        // Attacker (or accidental double-submit) replays the already-consumed
        // refresh token — this must burn the whole family.
        let replayed = jwt.refresh_tokens(&tokens.refresh_token, Some(&rl)).await;
        assert!(replayed.is_err(), "replayed refresh token must be rejected");
        assert!(
            rl.is_family_revoked(&family_id).await,
            "family should be revoked after replay"
        );

        // Even the freshly-rotated (previously valid) refresh token is now dead.
        let rotated_after = jwt.validate_token(&rotated.refresh_token, Some(&rl)).await;
        assert!(
            rotated_after.is_err(),
            "rotated token in revoked family must be rejected"
        );

        // And so are any access tokens carrying that family_id.
        let access_after = jwt.validate_token(&rotated.access_token, Some(&rl)).await;
        assert!(access_after.is_err());
    }

    // ── Redis integration tests (opt-in) ──
    //
    // These run ONLY when the `redis-revocation` cargo feature is on AND
    // `PARKHUB_TEST_REDIS_URL` is set in the environment. They are `#[ignore]`
    // so `cargo test` without that env var stays fast and network-free.
    #[cfg(feature = "redis-revocation")]
    mod redis_integration {
        use super::*;

        async fn connect() -> Option<Arc<RedisRevocationList>> {
            let url = std::env::var("PARKHUB_TEST_REDIS_URL").ok()?;
            Some(
                RedisRevocationList::connect(&url, 60)
                    .await
                    .expect("test redis should be reachable"),
            )
        }

        #[tokio::test]
        #[ignore = "requires PARKHUB_TEST_REDIS_URL"]
        async fn redis_revoke_roundtrip() {
            let Some(store) = connect().await else {
                return;
            };
            let jti = format!("test-{}", Uuid::new_v4());
            assert!(!store.is_revoked(&jti).await);
            store.revoke(&jti).await;
            assert!(store.is_revoked(&jti).await);
        }

        #[tokio::test]
        #[ignore = "requires PARKHUB_TEST_REDIS_URL"]
        async fn redis_family_revoke_roundtrip() {
            let Some(store) = connect().await else {
                return;
            };
            let family_id = format!("fam-{}", Uuid::new_v4());
            assert!(!store.is_family_revoked(&family_id).await);
            store.revoke_family(&family_id).await;
            assert!(store.is_family_revoked(&family_id).await);
        }

        #[tokio::test]
        #[ignore = "requires PARKHUB_TEST_REDIS_URL"]
        async fn redis_replay_revokes_family_end_to_end() {
            let Some(store) = connect().await else {
                return;
            };
            let jwt = JwtManager::with_random_secret();
            let rl = TokenRevocationList::from_store(store);
            let tokens = jwt
                .generate_tokens(&Uuid::new_v4(), "alice", "user")
                .unwrap();
            let family_id = jwt
                .validate_token(&tokens.refresh_token, None)
                .await
                .unwrap()
                .family_id
                .unwrap();

            let _ = jwt
                .refresh_tokens(&tokens.refresh_token, Some(&rl))
                .await
                .expect("first refresh ok");
            let replay = jwt.refresh_tokens(&tokens.refresh_token, Some(&rl)).await;
            assert!(replay.is_err());
            assert!(rl.is_family_revoked(&family_id).await);
        }
    }
}
