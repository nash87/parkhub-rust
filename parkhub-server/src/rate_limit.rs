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
    let quota = Quota::per_second(NonZeroU32::new(config.requests_per_second).unwrap())
        .allow_burst(NonZeroU32::new(config.burst_size).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    rate_limiter: Arc<GlobalRateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response {
    match rate_limiter.check() {
        Ok(_) => next.run(request).await,
        Err(_) => {
            // Return 429 Too Many Requests
            let error = AppError::RateLimited;
            error.into_response()
        }
    }
}

/// Per-IP rate limiter for more granular control
pub mod per_ip {
    use super::*;
    use governor::state::keyed::DashMapStateStore;
    use std::net::IpAddr;

    pub type IpRateLimiter = RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock, NoOpMiddleware>;

    /// Create a per-IP rate limiter with a per-minute quota
    pub fn create_ip_rate_limiter(requests_per_minute: u32) -> Arc<IpRateLimiter> {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        Arc::new(RateLimiter::dashmap(quota))
    }

    /// Create a per-IP rate limiter with a custom period
    /// e.g. 3 requests per 15 minutes: `create_ip_rate_limiter_with_period(3, Duration::from_secs(900))`
    pub fn create_ip_rate_limiter_with_period(requests: u32, period: Duration) -> Arc<IpRateLimiter> {
        let quota = Quota::with_period(period)
            .unwrap()
            .allow_burst(NonZeroU32::new(requests).unwrap());
        Arc::new(RateLimiter::dashmap(quota))
    }

    /// Extract client IP from request
    pub fn get_client_ip(addr: Option<&SocketAddr>, forwarded_for: Option<&str>) -> IpAddr {
        // Check X-Forwarded-For header first (for proxied requests)
        if let Some(forwarded) = forwarded_for {
            if let Some(first_ip) = forwarded.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }

        // Fall back to direct connection IP
        addr.map(|a| a.ip())
            .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]))
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
        .map(|s| s.to_owned());

    let peer_addr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0);

    let client_ip = per_ip::get_client_ip(peer_addr.as_ref(), forwarded_for.as_deref());

    match limiter.check_key(&client_ip) {
        Ok(_) => next.run(request).await,
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

        // From direct connection
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let ip = per_ip::get_client_ip(Some(&addr), None);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));

        // From X-Forwarded-For
        let ip = per_ip::get_client_ip(Some(&addr), Some("10.0.0.1, 192.168.1.1"));
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    }
}
