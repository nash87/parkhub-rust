//! Prometheus Metrics
//!
//! Exposes application metrics in Prometheus format.

use axum::{http::StatusCode, response::IntoResponse};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

/// Initialize the Prometheus metrics exporter
pub fn init_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder")
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
pub fn record_db_operation(operation: &str, table: &str, duration: std::time::Duration, success: bool) {
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
}
