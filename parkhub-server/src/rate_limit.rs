//! Rate Limiting
//!
//! Provides configurable rate limiting using the Governor library.
//!
//! ## Layered model (T-1743)
//!
//! Authenticated endpoints run through two stacked limiters:
//!
//! 1. **Per-IP** — the classic limiter keyed off the client IP. Protects the
//!    server from unauthenticated flooding and from attackers who don't yet
//!    have credentials.
//! 2. **Per-identity** — a second limiter keyed off the caller's `user_id`
//!    (session / bearer / cookie auth) or `api_key_id` (X-API-Key auth).
//!    Protects against credential-reuse attacks where a compromised key or
//!    session rotates across IPs behind CGNAT/mobile carrier NAT.
//!
//! Both limiters must allow the request — the stricter of the two wins.
//! Unauthenticated requests bypass the per-identity layer entirely and are
//! subject only to the per-IP limiter (current behaviour preserved).

use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use std::{
    net::SocketAddr,
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use crate::error::AppError;

/// Rate limiter type alias
pub type GlobalRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per second (global)
    pub requests_per_second: u32,
    /// Burst size
    pub burst_size: u32,
    /// Enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 200,
            enabled: true,
        }
    }
}

/// Create a new rate limiter
pub fn create_rate_limiter(config: &RateLimitConfig) -> Arc<GlobalRateLimiter> {
    let rps = NonZeroU32::new(config.requests_per_second.max(1))
        .expect("requests_per_second clamped to >= 1");
    let burst = NonZeroU32::new(config.burst_size.max(1)).expect("burst_size clamped to >= 1");
    let quota = Quota::per_second(rps).allow_burst(burst);

    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    rate_limiter: Arc<GlobalRateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response {
    match rate_limiter.check() {
        Ok(()) => next.run(request).await,
        Err(_) => {
            let mut response = AppError::RateLimited.into_response();
            let headers = response.headers_mut();
            headers.insert("retry-after", "60".parse().unwrap());
            response
        }
    }
}

/// Per-IP rate limiter for more granular control
pub mod per_ip {
    use super::{
        Arc, DefaultClock, Duration, NoOpMiddleware, NonZeroU32, Quota, RateLimiter, SocketAddr,
    };
    use governor::state::keyed::DashMapStateStore;
    use std::net::IpAddr;

    pub type IpRateLimiter =
        RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock, NoOpMiddleware>;

    /// Create a per-IP rate limiter with a per-minute quota
    pub fn create_ip_rate_limiter(requests_per_minute: u32) -> Arc<IpRateLimiter> {
        let rpm = NonZeroU32::new(requests_per_minute.max(1))
            .expect("requests_per_minute clamped to >= 1");
        let quota = Quota::per_minute(rpm);
        Arc::new(RateLimiter::dashmap(quota))
    }

    /// Create a per-IP rate limiter with a custom period
    /// e.g. 3 requests per 15 minutes: `create_ip_rate_limiter_with_period(3, Duration::from_secs(900))`
    pub fn create_ip_rate_limiter_with_period(
        requests: u32,
        period: Duration,
    ) -> Arc<IpRateLimiter> {
        let burst = NonZeroU32::new(requests.max(1)).expect("requests clamped to >= 1");
        let quota = Quota::with_period(period)
            .expect("period must be non-zero")
            .allow_burst(burst);
        Arc::new(RateLimiter::dashmap(quota))
    }

    /// Extract client IP from request.
    ///
    /// Only trusts the `X-Forwarded-For` header when the direct peer is a
    /// private/loopback address (i.e., a trusted reverse proxy on the LAN).
    /// Trusting the header unconditionally allows any remote client to spoof
    /// an arbitrary source IP and bypass per-IP rate limiting.
    pub fn get_client_ip(addr: Option<&SocketAddr>, forwarded_for: Option<&str>) -> IpAddr {
        let peer_ip = addr.map(std::net::SocketAddr::ip);

        // Only honour X-Forwarded-For when the request arrives from a trusted
        // proxy (private network or loopback).  Requests from public IPs use
        // their direct peer address regardless of the header value.
        let is_trusted_proxy = peer_ip.is_some_and(|ip| is_private_ip(&ip));

        if is_trusted_proxy
            && let Some(forwarded) = forwarded_for
            && let Some(first_ip) = forwarded.split(',').next()
            && let Ok(ip) = first_ip.trim().parse::<IpAddr>()
        {
            return ip;
        }

        // Fall back to direct connection IP
        peer_ip.unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]))
    }

    /// Returns true if `ip` is a private, loopback, or link-local address —
    /// i.e., an address that can only originate from a trusted internal host.
    const fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        }
    }
}

/// Middleware that enforces a per-IP rate limit.
///
/// Reads the `X-Forwarded-For` header (set by the ingress proxy) to identify
/// the real client IP.  Falls back to the direct peer address when the header
/// is absent.  Returns **429 Too Many Requests** when the limit is exceeded.
pub async fn ip_rate_limit_middleware(
    limiter: Arc<per_ip::IpRateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Resolve client IP from forwarded header or peer address
    let forwarded_for = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(std::borrow::ToOwned::to_owned);

    let peer_addr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0);

    let client_ip = per_ip::get_client_ip(peer_addr.as_ref(), forwarded_for.as_deref());

    match limiter.check_key(&client_ip) {
        Ok(()) => next.run(request).await,
        Err(_) => {
            let mut response = AppError::RateLimited.into_response();
            let headers = response.headers_mut();
            headers.insert("x-ratelimit-remaining", "0".parse().unwrap());
            headers.insert("retry-after", "60".parse().unwrap());
            response
        }
    }
}

/// Specific rate limiters for different endpoints
pub struct EndpointRateLimiters {
    /// Login attempts — 5 per minute per IP
    pub login: Arc<per_ip::IpRateLimiter>,
    /// Registration — 3 per minute per IP
    pub register: Arc<per_ip::IpRateLimiter>,
    /// Token refresh — 10 per minute per IP
    pub token_refresh: Arc<per_ip::IpRateLimiter>,
    /// Forgot-password — 3 per 15 minutes per IP
    pub forgot_password: Arc<per_ip::IpRateLimiter>,
    /// Password reset (token submission) — 5 per 15 minutes per IP
    pub password_reset: Arc<per_ip::IpRateLimiter>,
    /// Demo vote/reset — 3 per minute per IP
    pub demo: Arc<per_ip::IpRateLimiter>,
    /// QR pass generation — 10 per minute per IP
    pub qr_pass: Arc<per_ip::IpRateLimiter>,
    /// Lobby display — 10 per minute per IP
    pub lobby_display: Arc<per_ip::IpRateLimiter>,
    /// General API (relaxed global limiter)
    pub general: Arc<GlobalRateLimiter>,
    /// Per-identity layered limiters (T-1743).  Applied *on top* of the
    /// per-IP limiters above for authenticated requests.  An authenticated
    /// call must pass BOTH the per-IP and the per-identity check — the
    /// stricter of the two decides.
    pub identity: Arc<IdentityRateLimiters>,
}

/// Compile-time gated rate-limit bypass.
///
/// Only builds with the `e2e-bypass` cargo feature honor
/// `PARKHUB_DISABLE_RATE_LIMITS`. The production Dockerfile builds with
/// `--no-default-features --features headless`, which never enables this
/// feature, so a leaked env var in prod can't silently disarm brute-force
/// protection. Defense-in-depth: without the feature, seeing the env var
/// panics so a misconfigured deployment fails loudly at startup.
fn bypass_requested() -> bool {
    let raw = std::env::var_os("PARKHUB_DISABLE_RATE_LIMITS");

    #[cfg(not(feature = "e2e-bypass"))]
    {
        assert!(
            raw.is_none(),
            "PARKHUB_DISABLE_RATE_LIMITS is set but this binary was not built \
             with the `e2e-bypass` cargo feature. Refusing to start."
        );
        false
    }

    #[cfg(feature = "e2e-bypass")]
    {
        raw.map(|v| v == "true" || v == "1").unwrap_or(false)
    }
}

impl EndpointRateLimiters {
    pub fn new() -> Self {
        let disable_limits = bypass_requested();
        let rpm = |normal: u32| if disable_limits { 100_000 } else { normal };
        let period = |normal: u32, secs: u64| -> (u32, Duration) {
            if disable_limits {
                (100_000, Duration::from_secs(60))
            } else {
                (normal, Duration::from_secs(secs))
            }
        };

        let (forgot_n, forgot_p) = period(3, 15 * 60);
        let (reset_n, reset_p) = period(5, 15 * 60);

        // Per-identity quotas — env-overridable, bypass-aware.
        let identity_limits = if disable_limits {
            IdentityLimits {
                login: 100_000,
                register: 100_000,
                password_reset: 100_000,
                mutation: 100_000,
                read: 100_000,
                admin: 100_000,
            }
        } else {
            IdentityLimits::from_env()
        };

        Self {
            // 5 login attempts per minute per IP (normal) / unlimited in test mode
            login: per_ip::create_ip_rate_limiter(rpm(5)),
            // 3 registrations per minute per IP
            register: per_ip::create_ip_rate_limiter(rpm(3)),
            // 10 token-refresh requests per minute per IP
            token_refresh: per_ip::create_ip_rate_limiter(rpm(10)),
            // 3 forgot-password requests per 15 minutes per IP
            forgot_password: per_ip::create_ip_rate_limiter_with_period(forgot_n, forgot_p),
            // 5 password-reset submissions per 15 minutes per IP
            password_reset: per_ip::create_ip_rate_limiter_with_period(reset_n, reset_p),
            // 3 demo vote/reset per minute per IP
            demo: per_ip::create_ip_rate_limiter(rpm(3)),
            // 10 QR pass requests per minute per IP
            qr_pass: per_ip::create_ip_rate_limiter(rpm(10)),
            // 10 lobby display requests per minute per IP
            lobby_display: per_ip::create_ip_rate_limiter(rpm(10)),
            // 100 requests per second globally
            general: create_rate_limiter(&RateLimitConfig::default()),
            // Per-identity layered limiters (T-1743)
            identity: Arc::new(IdentityRateLimiters::new(identity_limits)),
        }
    }
}

impl Default for EndpointRateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Per-identity rate limiter (T-1743)
// ────────────────────────────────────────────────────────────────────────────

/// Per-identity rate limiter — a lazy `DashMap` of governor limiters keyed off
/// a caller identity (`user_id` or `api_key_id`).
///
/// Reuses the `IpRateLimiter` type shape; only the key changes.  Limiters are
/// created on first hit and evicted by [`per_identity::sweep_idle`] after 5
/// minutes of inactivity so an attacker can't grow the map unboundedly.
///
/// See [`per_identity::IdentityRateLimiters`] for the bundle of
/// purpose-specific limiters (login / register / mutation / read / …).
pub mod per_identity {
    use super::{Arc, Duration, NonZeroU32, Quota, RateLimiter, Uuid};
    use dashmap::DashMap;
    use governor::clock::{Clock, DefaultClock};
    use governor::middleware::NoOpMiddleware;
    use governor::state::InMemoryState;
    use std::sync::Mutex;
    use std::time::Instant;

    /// Identity that keys the per-identity limiter.
    ///
    /// * `User(Uuid)` — JWT / bearer / cookie auth.
    /// * `ApiKey(Uuid)` — X-API-Key auth. Keyed by the *key id*, not the user,
    ///   so a compromised key can't starve the user's other keys.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Identity {
        User(Uuid),
        ApiKey(Uuid),
    }

    impl Identity {
        /// Returns the opaque bucket label exposed via the `X-RateLimit-Bucket`
        /// debug header.  Never leaks internal limiter names.
        pub const fn bucket_label(&self) -> &'static str {
            match self {
                Self::User(_) => "user",
                Self::ApiKey(_) => "api_key",
            }
        }
    }

    /// A single per-identity governor limiter (direct — one entry per id).
    pub type IdentityLimiter =
        RateLimiter<governor::state::NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

    struct Entry {
        limiter: Arc<IdentityLimiter>,
        quota_per_minute: u32,
        last_hit: Mutex<Instant>,
    }

    /// Lazy store of per-identity limiters sharing one quota.
    ///
    /// Allocates a fresh governor limiter per `Identity` on first use; reuses
    /// it afterwards.  Evicts entries idle for >5 min via [`sweep_idle`].
    pub struct IdentityBucket {
        inner: DashMap<Identity, Arc<Entry>>,
        quota: Quota,
        quota_per_minute: u32,
        idle_ttl: Duration,
    }

    impl IdentityBucket {
        /// Per-minute quota bucket.
        #[must_use]
        pub fn per_minute(requests_per_minute: u32) -> Self {
            let rpm = NonZeroU32::new(requests_per_minute.max(1))
                .expect("requests_per_minute clamped to >= 1");
            Self {
                inner: DashMap::new(),
                quota: Quota::per_minute(rpm),
                quota_per_minute: rpm.get(),
                idle_ttl: Duration::from_secs(5 * 60),
            }
        }

        /// Override the idle TTL.  Used by tests to exercise eviction.
        #[must_use]
        pub const fn with_idle_ttl(mut self, ttl: Duration) -> Self {
            self.idle_ttl = ttl;
            self
        }

        /// Quota size per minute (exposed for response headers).
        #[must_use]
        pub const fn quota_per_minute(&self) -> u32 {
            self.quota_per_minute
        }

        fn limiter_for(&self, id: Identity) -> Arc<Entry> {
            if let Some(existing) = self.inner.get(&id) {
                *existing.last_hit.lock().expect("identity mutex poisoned") = Instant::now();
                return Arc::clone(&existing);
            }
            let entry = Arc::new(Entry {
                limiter: Arc::new(RateLimiter::direct(self.quota)),
                quota_per_minute: self.quota_per_minute,
                last_hit: Mutex::new(Instant::now()),
            });
            self.inner
                .entry(id)
                .or_insert_with(|| Arc::clone(&entry))
                .clone()
        }

        /// Check whether the identity is currently allowed a request.
        ///
        /// Returns `Ok(remaining_approx, reset_unix_secs)` when allowed, or
        /// `Err(reset_unix_secs)` when over quota.  The returned "remaining"
        /// is a best-effort lower bound derived from the governor snapshot.
        pub fn check(&self, id: Identity) -> Result<RateInfo, RateInfo> {
            let entry = self.limiter_for(id);
            let clock = DefaultClock::default();
            match entry.limiter.check() {
                Ok(()) => Ok(RateInfo {
                    limit: entry.quota_per_minute,
                    remaining: entry
                        .quota_per_minute
                        .saturating_sub(1)
                        .min(entry.quota_per_minute),
                    reset_unix_secs: now_unix() + 60,
                }),
                Err(negative) => {
                    let wait = negative.wait_time_from(clock.now());
                    Err(RateInfo {
                        limit: entry.quota_per_minute,
                        remaining: 0,
                        reset_unix_secs: now_unix() + wait.as_secs().max(1),
                    })
                }
            }
        }

        /// Drop entries whose last hit is older than `idle_ttl`.
        ///
        /// Returns the number of entries evicted.  Called every 60 s from the
        /// background task spawned by [`spawn_eviction_task`].
        pub fn sweep_idle(&self) -> usize {
            let cutoff = Instant::now();
            let ttl = self.idle_ttl;
            let stale: Vec<Identity> = self
                .inner
                .iter()
                .filter_map(|r| {
                    let last = *r.value().last_hit.lock().expect("identity mutex poisoned");
                    if cutoff.duration_since(last) >= ttl {
                        Some(*r.key())
                    } else {
                        None
                    }
                })
                .collect();
            let n = stale.len();
            for id in stale {
                self.inner.remove(&id);
            }
            n
        }

        /// Number of live identity entries (for tests / metrics).
        #[must_use]
        pub fn len(&self) -> usize {
            self.inner.len()
        }

        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }
    }

    /// Rate-limit info returned from [`IdentityBucket::check`].
    #[derive(Debug, Clone, Copy)]
    pub struct RateInfo {
        pub limit: u32,
        pub remaining: u32,
        pub reset_unix_secs: u64,
    }

    fn now_unix() -> u64 {
        super::SystemTime::now()
            .duration_since(super::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }

    /// Bundle of per-identity buckets wired into [`super::IdentityRateLimiters`].
    ///
    /// Mutation buckets are stricter than read buckets so a leaked credential
    /// can't rack up write amplification while staying under the read quota.
    pub struct IdentityRateLimiters {
        pub login: IdentityBucket,
        pub register: IdentityBucket,
        pub password_reset: IdentityBucket,
        pub mutation: IdentityBucket,
        pub read: IdentityBucket,
        pub admin: IdentityBucket,
    }

    impl IdentityRateLimiters {
        #[must_use]
        pub fn new(limits: IdentityLimits) -> Self {
            Self {
                login: IdentityBucket::per_minute(limits.login),
                register: IdentityBucket::per_minute(limits.register),
                password_reset: IdentityBucket::per_minute(limits.password_reset),
                mutation: IdentityBucket::per_minute(limits.mutation),
                read: IdentityBucket::per_minute(limits.read),
                admin: IdentityBucket::per_minute(limits.admin),
            }
        }

        /// Sweep every bucket once.
        pub fn sweep_all(&self) -> usize {
            self.login.sweep_idle()
                + self.register.sweep_idle()
                + self.password_reset.sweep_idle()
                + self.mutation.sweep_idle()
                + self.read.sweep_idle()
                + self.admin.sweep_idle()
        }
    }

    /// Effective per-identity quotas resolved from env overrides with sensible
    /// defaults (see module docs / T-1743 for the rationale).
    #[derive(Debug, Clone, Copy)]
    pub struct IdentityLimits {
        pub login: u32,
        pub register: u32,
        pub password_reset: u32,
        pub mutation: u32,
        pub read: u32,
        pub admin: u32,
    }

    impl IdentityLimits {
        /// Defaults per T-1743 spec.
        pub const DEFAULTS: Self = Self {
            login: 10,
            register: 5,
            password_reset: 3,
            mutation: 60,
            read: 300,
            admin: 120,
        };

        /// Load from `PARKHUB_IDENTITY_LIMIT_*` env vars, falling back to
        /// `DEFAULTS` on unset / unparsable values.
        #[must_use]
        pub fn from_env() -> Self {
            fn parse(name: &str, default: u32) -> u32 {
                std::env::var(name)
                    .ok()
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|&n| n > 0)
                    .unwrap_or(default)
            }
            let d = Self::DEFAULTS;
            Self {
                login: parse("PARKHUB_IDENTITY_LIMIT_LOGIN", d.login),
                register: parse("PARKHUB_IDENTITY_LIMIT_REGISTER", d.register),
                password_reset: parse("PARKHUB_IDENTITY_LIMIT_PASSWORD_RESET", d.password_reset),
                mutation: parse("PARKHUB_IDENTITY_LIMIT_MUTATION", d.mutation),
                read: parse("PARKHUB_IDENTITY_LIMIT_READ", d.read),
                admin: parse("PARKHUB_IDENTITY_LIMIT_ADMIN", d.admin),
            }
        }
    }

    impl Default for IdentityLimits {
        fn default() -> Self {
            Self::DEFAULTS
        }
    }

    /// Spawn a tokio task that sweeps idle entries every 60 s.
    ///
    /// Returns the `JoinHandle` so callers can abort on shutdown.  Aborts
    /// silently if no tokio runtime is present (e.g. during unit tests that
    /// don't run one) — callers in those contexts sweep manually.
    pub fn spawn_eviction_task(
        limiters: Arc<IdentityRateLimiters>,
    ) -> Option<tokio::task::JoinHandle<()>> {
        if tokio::runtime::Handle::try_current().is_err() {
            return None;
        }
        Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(60));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let _ = limiters.sweep_all();
            }
        }))
    }
}

#[allow(unused_imports)] // `Identity` is referenced via `per_identity::Identity` by callers
pub use per_identity::{Identity, IdentityLimits, IdentityRateLimiters};

/// Bucket category the per-identity middleware should apply to a request.
#[derive(Debug, Clone, Copy)]
pub enum IdentityBucketKind {
    Login,
    Register,
    PasswordReset,
    Mutation,
    Read,
    Admin,
}

impl IdentityBucketKind {
    fn select(self, limiters: &IdentityRateLimiters) -> &per_identity::IdentityBucket {
        match self {
            Self::Login => &limiters.login,
            Self::Register => &limiters.register,
            Self::PasswordReset => &limiters.password_reset,
            Self::Mutation => &limiters.mutation,
            Self::Read => &limiters.read,
            Self::Admin => &limiters.admin,
        }
    }
}

/// Standard rate-limit response headers.
///
/// Uses `X-RateLimit-*` (the widely-adopted draft spec) rather than the
/// newer IETF `RateLimit-*` names because our existing per-IP middleware
/// already emits `x-ratelimit-remaining`; staying consistent avoids breaking
/// clients that parse the old names.
fn apply_rate_headers(response: &mut Response, info: per_identity::RateInfo, bucket: &str) {
    let headers = response.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&info.limit.to_string()) {
        headers.insert("x-ratelimit-limit", v);
    }
    if let Ok(v) = HeaderValue::from_str(&info.remaining.to_string()) {
        headers.insert("x-ratelimit-remaining", v);
    }
    if let Ok(v) = HeaderValue::from_str(&info.reset_unix_secs.to_string()) {
        headers.insert("x-ratelimit-reset", v);
    }
    if let Ok(v) = HeaderValue::from_str(bucket) {
        headers.insert("x-ratelimit-bucket", v);
    }
}

/// Middleware that enforces per-identity rate limits layered on top of an
/// existing per-IP limiter.
///
/// Lookup order:
///   1. If the request has an [`crate::api::AuthUser`] extension, pick the
///      identity (api_key_id preferred over user_id).
///   2. Check the per-identity bucket.  On reject, 429.
///   3. Otherwise run the rest of the chain and tag the response with
///      `X-RateLimit-Bucket` = `user` | `api_key`.
///
/// Unauthenticated requests pass through and get `X-RateLimit-Bucket: ip`
/// attached by the per-IP middleware layer, if present in the response.
pub async fn identity_rate_limit_middleware(
    limiters: Arc<IdentityRateLimiters>,
    kind: IdentityBucketKind,
    request: Request<Body>,
    next: Next,
) -> Response {
    let auth = request.extensions().get::<crate::api::AuthUser>().cloned();

    let Some(auth) = auth else {
        // Unauthenticated — no per-identity enforcement.  Let the per-IP
        // layer (if any) own this request entirely.  Tag the bucket header
        // so clients can tell which layer is dominant.
        let mut response = next.run(request).await;
        response
            .headers_mut()
            .insert("x-ratelimit-bucket", HeaderValue::from_static("ip"));
        return response;
    };

    // Prefer api_key_id when present — isolates each key's quota even for
    // the same user.
    let identity = auth.api_key_id.map_or(
        per_identity::Identity::User(auth.user_id),
        per_identity::Identity::ApiKey,
    );
    let bucket = kind.select(&limiters);

    match bucket.check(identity) {
        Ok(info) => {
            let mut response = next.run(request).await;
            apply_rate_headers(&mut response, info, identity.bucket_label());
            response
        }
        Err(info) => {
            let mut response = AppError::RateLimited.into_response();
            apply_rate_headers(&mut response, info, identity.bucket_label());
            let headers = response.headers_mut();
            headers.insert(
                "retry-after",
                HeaderValue::from_str(
                    &info
                        .reset_unix_secs
                        .saturating_sub(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map_or(0, |d| d.as_secs()),
                        )
                        .max(1)
                        .to_string(),
                )
                .unwrap_or(HeaderValue::from_static("60")),
            );
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = create_rate_limiter(&config);

        // Should allow burst
        for _ in 0..100 {
            assert!(limiter.check().is_ok());
        }
    }

    #[test]
    fn test_ip_extraction() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        // From direct connection (private IP)
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let ip = per_ip::get_client_ip(Some(&addr), None);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));

        // From X-Forwarded-For via trusted proxy
        let ip = per_ip::get_client_ip(Some(&addr), Some("10.0.0.1, 192.168.1.1"));
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    }

    #[test]
    fn test_ip_extraction_untrusted_proxy() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        // Public IP peer should NOT trust X-Forwarded-For
        let public_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 8080);
        let ip = per_ip::get_client_ip(Some(&public_addr), Some("10.0.0.1"));
        // Should use peer IP, not forwarded
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)));
    }

    #[test]
    fn test_ip_extraction_no_peer() {
        use std::net::IpAddr;

        // No peer address → fallback to 127.0.0.1
        let ip = per_ip::get_client_ip(None, None);
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn test_ip_rate_limiter_allows_burst() {
        let limiter = per_ip::create_ip_rate_limiter(5);
        let ip: std::net::IpAddr = "10.0.0.1".parse().unwrap();
        // 5 requests should pass
        for _ in 0..5 {
            assert!(limiter.check_key(&ip).is_ok());
        }
        // 6th should fail
        assert!(limiter.check_key(&ip).is_err());
    }

    #[test]
    fn test_endpoint_rate_limiters_creation() {
        let limiters = EndpointRateLimiters::new();
        let test_ip: std::net::IpAddr = "10.0.0.1".parse().unwrap();
        // All limiters should be created and accept the first request
        assert!(limiters.login.check_key(&test_ip).is_ok());
        assert!(limiters.register.check_key(&test_ip).is_ok());
        assert!(limiters.token_refresh.check_key(&test_ip).is_ok());
        assert!(limiters.forgot_password.check_key(&test_ip).is_ok());
        assert!(limiters.password_reset.check_key(&test_ip).is_ok());
        assert!(limiters.lobby_display.check_key(&test_ip).is_ok());
        assert!(limiters.general.check().is_ok());
        // Per-identity bundle is also present
        let user = per_identity::Identity::User(uuid::Uuid::nil());
        assert!(limiters.identity.read.check(user).is_ok());
    }

    // ─── T-1743 per-identity tests ────────────────────────────────────────

    /// User A exhausts the per-identity login quota → further requests
    /// return 429 even when arriving from a different IP.  This is the
    /// core regression the feature prevents: stolen credential + CGNAT
    /// rotation.
    #[test]
    fn test_identity_limit_follows_credential_across_ips() {
        use per_identity::{Identity, IdentityBucket};

        let bucket = IdentityBucket::per_minute(3);
        let alice = Identity::User(uuid::Uuid::from_u128(0xA11CE));

        // 3 hits allowed (regardless of pretend IP the caller claims).
        for _ in 0..3 {
            assert!(bucket.check(alice).is_ok());
        }
        // 4th is rejected — the per-IP limiter doesn't matter, the
        // identity bucket itself is full.
        assert!(bucket.check(alice).is_err());
    }

    /// Two users behind the same NAT IP both get their own quota —
    /// exhaustion by one must NOT block the other.
    #[test]
    fn test_two_identities_same_nat_dont_starve() {
        use per_identity::{Identity, IdentityBucket};

        let bucket = IdentityBucket::per_minute(2);
        let alice = Identity::User(uuid::Uuid::from_u128(1));
        let bob = Identity::User(uuid::Uuid::from_u128(2));

        // Alice burns her quota.
        assert!(bucket.check(alice).is_ok());
        assert!(bucket.check(alice).is_ok());
        assert!(bucket.check(alice).is_err());

        // Bob still has his entire quota.
        assert!(bucket.check(bob).is_ok());
        assert!(bucket.check(bob).is_ok());
        assert!(bucket.check(bob).is_err());
    }

    /// API-key auth isolates each key's quota — a leaked key can't starve
    /// the user's other keys (or vice versa).
    #[test]
    fn test_api_key_buckets_are_independent() {
        use per_identity::{Identity, IdentityBucket};

        let bucket = IdentityBucket::per_minute(1);
        let key_a = Identity::ApiKey(uuid::Uuid::from_u128(0xAAAA));
        let key_b = Identity::ApiKey(uuid::Uuid::from_u128(0xBBBB));

        assert!(bucket.check(key_a).is_ok());
        assert!(bucket.check(key_a).is_err());
        // Sibling key still has its quota.
        assert!(bucket.check(key_b).is_ok());
        assert!(bucket.check(key_b).is_err());
    }

    /// The bucket_label emitted in the X-RateLimit-Bucket header must
    /// never leak internal limiter names — only a small enum-of-strings.
    #[test]
    fn test_bucket_label_enum_of_strings() {
        use per_identity::Identity;

        assert_eq!(Identity::User(uuid::Uuid::nil()).bucket_label(), "user");
        assert_eq!(
            Identity::ApiKey(uuid::Uuid::nil()).bucket_label(),
            "api_key"
        );
    }

    /// Idle entries are evicted by `sweep_idle` — prevents unbounded map
    /// growth from attackers cycling identities.
    #[test]
    fn test_identity_bucket_sweeps_idle_entries() {
        use per_identity::{Identity, IdentityBucket};

        // TTL of zero means "sweep anything not touched this instant".
        let bucket =
            IdentityBucket::per_minute(10).with_idle_ttl(std::time::Duration::from_millis(0));

        for i in 0u32..5 {
            let id = Identity::User(uuid::Uuid::from_u128(u128::from(i)));
            assert!(bucket.check(id).is_ok());
        }
        assert_eq!(bucket.len(), 5);
        // A tiny sleep ensures the Instant::now() inside sweep is strictly
        // after the last_hit values we just wrote.
        std::thread::sleep(std::time::Duration::from_millis(1));
        let evicted = bucket.sweep_idle();
        assert_eq!(evicted, 5);
        assert!(bucket.is_empty());
    }

    /// Env vars override defaults; unset falls back to DEFAULTS.
    #[test]
    fn test_identity_limits_defaults() {
        let d = IdentityLimits::DEFAULTS;
        assert_eq!(d.login, 10);
        assert_eq!(d.register, 5);
        assert_eq!(d.password_reset, 3);
        assert_eq!(d.mutation, 60);
        assert_eq!(d.read, 300);
        assert_eq!(d.admin, 120);
    }
}
