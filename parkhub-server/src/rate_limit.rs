//! Rate Limiting
//!
//! Provides configurable rate limiting using the Governor library.

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

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
    if rate_limiter.check() == Ok(()) {
        next.run(request).await
    } else {
        // Return 429 Too Many Requests
        let error = AppError::RateLimited;
        error.into_response()
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

        if is_trusted_proxy {
            if let Some(forwarded) = forwarded_for {
                if let Some(first_ip) = forwarded.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
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
        Err(_) => AppError::RateLimited.into_response(),
    }
}

/// Specific rate limiters for different endpoints
pub struct EndpointRateLimiters {
    /// Login attempts — 5 per minute per IP
    pub login: Arc<per_ip::IpRateLimiter>,
    /// Registration — 3 per minute per IP
    pub register: Arc<per_ip::IpRateLimiter>,
    /// Forgot-password — 3 per 15 minutes per IP
    pub forgot_password: Arc<per_ip::IpRateLimiter>,
    /// Demo vote/reset — 3 per minute per IP
    pub demo: Arc<per_ip::IpRateLimiter>,
    /// QR pass generation — 10 per minute per IP
    pub qr_pass: Arc<per_ip::IpRateLimiter>,
    /// General API (relaxed global limiter)
    pub general: Arc<GlobalRateLimiter>,
}

impl EndpointRateLimiters {
    pub fn new() -> Self {
        Self {
            // 5 login attempts per minute per IP
            login: per_ip::create_ip_rate_limiter(5),
            // 3 registrations per minute per IP
            register: per_ip::create_ip_rate_limiter(3),
            // 3 forgot-password requests per 15 minutes per IP
            forgot_password: per_ip::create_ip_rate_limiter_with_period(
                3,
                Duration::from_secs(15 * 60),
            ),
            // 3 demo vote/reset per minute per IP
            demo: per_ip::create_ip_rate_limiter(3),
            // 10 QR pass requests per minute per IP
            qr_pass: per_ip::create_ip_rate_limiter(10),
            // 100 requests per second globally
            general: create_rate_limiter(&RateLimitConfig::default()),
        }
    }
}

impl Default for EndpointRateLimiters {
    fn default() -> Self {
        Self::new()
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
        // All limiters should be created
        assert!(limiters
            .login
            .check_key(&"10.0.0.1".parse().unwrap())
            .is_ok());
        assert!(limiters
            .register
            .check_key(&"10.0.0.1".parse().unwrap())
            .is_ok());
        assert!(limiters
            .forgot_password
            .check_key(&"10.0.0.1".parse().unwrap())
            .is_ok());
        assert!(limiters.general.check().is_ok());
    }
}
