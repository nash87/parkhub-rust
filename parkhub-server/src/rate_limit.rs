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
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc};

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

    /// Create a per-IP rate limiter
    pub fn create_ip_rate_limiter(requests_per_minute: u32) -> Arc<IpRateLimiter> {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
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

/// Specific rate limiters for different endpoints
pub struct EndpointRateLimiters {
    /// Login attempts (stricter)
    pub login: Arc<per_ip::IpRateLimiter>,
    /// Registration (stricter)
    pub register: Arc<per_ip::IpRateLimiter>,
    /// General API (relaxed)
    pub general: Arc<GlobalRateLimiter>,
}

impl EndpointRateLimiters {
    pub fn new() -> Self {
        Self {
            // 5 login attempts per minute per IP
            login: per_ip::create_ip_rate_limiter(5),
            // 3 registrations per minute per IP
            register: per_ip::create_ip_rate_limiter(3),
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
