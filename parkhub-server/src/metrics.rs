//! Prometheus Metrics
//!
//! Exposes application metrics in Prometheus format.

use axum::{http::StatusCode, response::IntoResponse};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

/// Initialize the Prometheus metrics exporter.
///
/// Returns a cached handle on subsequent calls — safe to invoke from multiple
/// test threads without triggering a global-recorder conflict.
pub fn init_metrics() -> PrometheusHandle {
    static HANDLE: std::sync::OnceLock<PrometheusHandle> = std::sync::OnceLock::new();
    HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .expect("Failed to install Prometheus recorder")
        })
        .clone()
}

/// Metrics endpoint handler
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Monitoring",
    responses(
        (status = 200, description = "Prometheus metrics", content_type = "text/plain"),
    )
)]
pub async fn metrics_handler(handle: PrometheusHandle) -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        handle.render(),
    )
}

// === Metric Recording Helpers ===

/// Record an HTTP request
pub fn record_http_request(method: &str, path: &str, status: u16, duration: std::time::Duration) {
    let labels = [
        ("method", method.to_string()),
        ("path", path.to_string()),
        ("status", status.to_string()),
    ];

    counter!("http_requests_total", &labels).increment(1);
    histogram!("http_request_duration_seconds", &labels).record(duration.as_secs_f64());
}

/// Record a database operation
pub fn record_db_operation(
    operation: &str,
    table: &str,
    duration: std::time::Duration,
    success: bool,
) {
    let labels = [
        ("operation", operation.to_string()),
        ("table", table.to_string()),
        ("success", success.to_string()),
    ];

    counter!("db_operations_total", &labels).increment(1);
    histogram!("db_operation_duration_seconds", &labels).record(duration.as_secs_f64());
}

/// Record active sessions
pub fn record_active_sessions(count: u64) {
    gauge!("active_sessions").set(count as f64);
}

/// Record active bookings
pub fn record_active_bookings(count: u64) {
    gauge!("active_bookings").set(count as f64);
}

/// Record parking lot occupancy
pub fn record_lot_occupancy(lot_id: &str, lot_name: &str, total: u64, occupied: u64) {
    let labels = [
        ("lot_id", lot_id.to_string()),
        ("lot_name", lot_name.to_string()),
    ];

    gauge!("parking_lot_total_slots", &labels).set(total as f64);
    gauge!("parking_lot_occupied_slots", &labels).set(occupied as f64);

    if total > 0 {
        let occupancy_rate = (occupied as f64 / total as f64) * 100.0;
        gauge!("parking_lot_occupancy_percent", &labels).set(occupancy_rate);
    }
}

/// Record authentication events
pub fn record_auth_event(event_type: &str, success: bool) {
    let labels = [
        ("event", event_type.to_string()),
        ("success", success.to_string()),
    ];

    counter!("auth_events_total", &labels).increment(1);
}

/// Record booking events
pub fn record_booking_event(event_type: &str) {
    let labels = [("event", event_type.to_string())];
    counter!("booking_events_total", &labels).increment(1);
}

/// Timer for measuring operation duration
pub struct MetricsTimer {
    start: Instant,
    operation: String,
    table: String,
}

impl MetricsTimer {
    pub fn new(operation: &str, table: &str) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.to_string(),
            table: table.to_string(),
        }
    }

    pub fn finish(self, success: bool) {
        record_db_operation(&self.operation, &self.table, self.start.elapsed(), success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_timer() {
        let timer = MetricsTimer::new("read", "users");
        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.finish(true);
        // Timer should complete without panic
    }

    #[test]
    fn test_metrics_timer_failure() {
        let timer = MetricsTimer::new("write", "bookings");
        timer.finish(false);
    }

    #[test]
    fn test_metrics_timer_stores_fields() {
        let timer = MetricsTimer::new("delete", "sessions");
        assert_eq!(timer.operation, "delete");
        assert_eq!(timer.table, "sessions");
    }

    #[test]
    fn test_record_http_request_no_panic() {
        record_http_request(
            "GET",
            "/api/lots",
            200,
            std::time::Duration::from_millis(50),
        );
        record_http_request(
            "POST",
            "/api/bookings",
            201,
            std::time::Duration::from_millis(120),
        );
        record_http_request(
            "GET",
            "/api/missing",
            404,
            std::time::Duration::from_millis(5),
        );
        record_http_request("POST", "/api/login", 500, std::time::Duration::from_secs(2));
    }

    #[test]
    fn test_record_auth_event_no_panic() {
        record_auth_event("login", true);
        record_auth_event("login", false);
        record_auth_event("token_refresh", true);
        record_auth_event("logout", true);
    }

    #[test]
    fn test_record_booking_event_no_panic() {
        record_booking_event("created");
        record_booking_event("cancelled");
        record_booking_event("extended");
        record_booking_event("checked_in");
        record_booking_event("checked_out");
    }

    #[test]
    fn test_record_lot_occupancy_no_panic() {
        record_lot_occupancy("lot-1", "Main Lot", 100, 75);
        record_lot_occupancy("lot-2", "Overflow", 50, 0);
        record_lot_occupancy("lot-3", "VIP", 10, 10);
    }

    #[test]
    fn test_record_lot_occupancy_zero_total() {
        // Should not divide by zero
        record_lot_occupancy("empty-lot", "Empty", 0, 0);
    }

    #[test]
    fn test_record_db_operation_no_panic() {
        record_db_operation("insert", "users", std::time::Duration::from_millis(5), true);
        record_db_operation(
            "select",
            "bookings",
            std::time::Duration::from_millis(15),
            true,
        );
        record_db_operation(
            "update",
            "slots",
            std::time::Duration::from_millis(8),
            false,
        );
    }

    #[test]
    fn test_record_active_sessions_no_panic() {
        record_active_sessions(0);
        record_active_sessions(42);
        record_active_sessions(1000);
    }

    #[test]
    fn test_record_active_bookings_no_panic() {
        record_active_bookings(0);
        record_active_bookings(100);
    }
}
