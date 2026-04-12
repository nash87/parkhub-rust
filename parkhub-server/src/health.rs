//! Health Check Endpoints
//!
//! Provides /health and /ready endpoints for monitoring and orchestration.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

use crate::db::Database;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Overall health status
    pub status: HealthStatus,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Individual component checks
    pub checks: Vec<ComponentHealth>,
}

/// Health status
#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual component health
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: HealthStatus,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,
}

/// Readiness response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadyResponse {
    /// Ready to accept traffic
    pub ready: bool,
    /// Reason if not ready
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Application state for health checks
pub struct AppHealth {
    pub start_time: std::time::Instant,
    pub db: Arc<RwLock<Database>>,
}

/// Liveness probe - is the service alive?
///
/// Returns 200 if the service is running, regardless of dependencies.
/// Used by orchestrators to detect crashed services.
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "Health",
    responses(
        (status = 200, description = "Service is alive"),
    )
)]
pub fn liveness() -> impl IntoResponse {
    StatusCode::OK
}

/// Readiness probe - is the service ready to accept traffic?
///
/// Checks all dependencies (database, etc.) before returning ready.
/// Used by load balancers to route traffic.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready", body = ReadyResponse),
        (status = 503, description = "Service is not ready", body = ReadyResponse),
    )
)]
pub async fn readiness(State(health): State<Arc<AppHealth>>) -> impl IntoResponse {
    // Check database
    let db = health.db.read().await;
    let db_check = db.stats().await;
    drop(db);

    match db_check {
        Ok(_) => (
            StatusCode::OK,
            Json(ReadyResponse {
                ready: true,
                reason: None,
            }),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                ready: false,
                reason: Some(format!("Database unavailable: {e}")),
            }),
        ),
    }
}

/// Full health check - detailed status of all components
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Health check response", body = HealthResponse),
    )
)]
pub async fn health_check(State(health): State<Arc<AppHealth>>) -> Json<HealthResponse> {
    let mut checks = Vec::new();
    let mut overall_status = HealthStatus::Healthy;

    // Database check
    let db_start = std::time::Instant::now();
    let db = health.db.read().await;
    let db_check = db.stats().await;
    drop(db);
    #[allow(clippy::cast_possible_truncation)]
    let db_response_time = db_start.elapsed().as_millis() as u64;

    match db_check {
        Ok(stats) => {
            checks.push(ComponentHealth {
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                message: Some(format!(
                    "OK - {} users, {} bookings",
                    stats.users, stats.bookings
                )),
                response_time_ms: Some(db_response_time),
            });
        }
        Err(e) => {
            checks.push(ComponentHealth {
                name: "database".to_string(),
                status: HealthStatus::Unhealthy,
                message: Some(e.to_string()),
                response_time_ms: Some(db_response_time),
            });
            overall_status = HealthStatus::Unhealthy;
        }
    }

    // Memory check (warn if > 500MB)
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            if let Some(line) = status.lines().find(|l| l.starts_with("VmRSS:")) {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        let mb = kb / 1024;
                        let status = if mb > 500 {
                            HealthStatus::Degraded
                        } else {
                            HealthStatus::Healthy
                        };
                        if status == HealthStatus::Degraded
                            && overall_status == HealthStatus::Healthy
                        {
                            overall_status = HealthStatus::Degraded;
                        }
                        checks.push(ComponentHealth {
                            name: "memory".to_string(),
                            status,
                            message: Some(format!("{mb} MB")),
                            response_time_ms: None,
                        });
                    }
                }
            }
        }
    }

    let uptime = health.start_time.elapsed().as_secs();

    Json(HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Degraded).unwrap(),
            "\"degraded\""
        );
    }

    #[test]
    fn test_health_status_unhealthy_serialization() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Unhealthy).unwrap(),
            "\"unhealthy\""
        );
    }

    #[test]
    fn test_health_status_deserialization() {
        let h: HealthStatus = serde_json::from_str("\"healthy\"").unwrap();
        assert_eq!(h, HealthStatus::Healthy);
        let d: HealthStatus = serde_json::from_str("\"degraded\"").unwrap();
        assert_eq!(d, HealthStatus::Degraded);
        let u: HealthStatus = serde_json::from_str("\"unhealthy\"").unwrap();
        assert_eq!(u, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_status_roundtrip() {
        for status in [
            HealthStatus::Healthy,
            HealthStatus::Degraded,
            HealthStatus::Unhealthy,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: HealthStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn test_health_response_serialization() {
        let resp = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks: vec![ComponentHealth {
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                message: Some("OK".to_string()),
                response_time_ms: Some(5),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["uptime_seconds"], 3600);
        assert_eq!(json["checks"][0]["name"], "database");
        assert_eq!(json["checks"][0]["response_time_ms"], 5);
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{
            "status": "degraded",
            "version": "2.0.0",
            "uptime_seconds": 120,
            "checks": []
        }"#;
        let resp: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, HealthStatus::Degraded);
        assert_eq!(resp.version, "2.0.0");
        assert_eq!(resp.uptime_seconds, 120);
        assert!(resp.checks.is_empty());
    }

    #[test]
    fn test_component_health_skips_none_fields() {
        let comp = ComponentHealth {
            name: "memory".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            response_time_ms: None,
        };
        let json = serde_json::to_value(&comp).unwrap();
        assert!(json.get("message").is_none());
        assert!(json.get("response_time_ms").is_none());
        assert_eq!(json["name"], "memory");
    }

    #[test]
    fn test_component_health_includes_present_fields() {
        let comp = ComponentHealth {
            name: "db".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some("Connection refused".to_string()),
            response_time_ms: Some(1000),
        };
        let json = serde_json::to_value(&comp).unwrap();
        assert_eq!(json["message"], "Connection refused");
        assert_eq!(json["response_time_ms"], 1000);
        assert_eq!(json["status"], "unhealthy");
    }

    #[test]
    fn test_ready_response_ready() {
        let resp = ReadyResponse {
            ready: true,
            reason: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ready"], true);
        assert!(json.get("reason").is_none());
    }

    #[test]
    fn test_ready_response_not_ready() {
        let resp = ReadyResponse {
            ready: false,
            reason: Some("Database down".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ready"], false);
        assert_eq!(json["reason"], "Database down");
    }

    #[test]
    fn test_health_response_with_multiple_checks() {
        let resp = HealthResponse {
            status: HealthStatus::Degraded,
            version: "0.5.0".to_string(),
            uptime_seconds: 60,
            checks: vec![
                ComponentHealth {
                    name: "database".to_string(),
                    status: HealthStatus::Healthy,
                    message: Some("OK - 10 users, 5 bookings".to_string()),
                    response_time_ms: Some(2),
                },
                ComponentHealth {
                    name: "memory".to_string(),
                    status: HealthStatus::Degraded,
                    message: Some("600 MB".to_string()),
                    response_time_ms: None,
                },
            ],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["checks"].as_array().unwrap().len(), 2);
        assert_eq!(json["checks"][1]["status"], "degraded");
    }
}
