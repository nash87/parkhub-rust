//! JSON report generation for simulation results.

use crate::simulation::injector::{InjectionResults, SimContext};
use crate::simulation::profiles::SimProfile;
use crate::simulation::verifier::VerificationResult;
use serde::Serialize;
use std::time::Duration;

/// Full simulation report, serializable to JSON.
#[derive(Debug, Serialize)]
pub struct SimulationReport {
    pub profile: String,
    pub duration_seconds: f64,
    pub total_users: usize,
    pub total_lots: usize,
    pub total_slots: usize,
    pub total_booking_attempts: usize,
    pub successful_bookings: usize,
    pub rejected_conflicts: usize,
    pub cancellations: usize,
    pub recurring_instances: usize,
    pub waitlist_entries: usize,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub max_latency_ms: u64,
    pub errors: usize,
    pub double_bookings: usize,
    pub audit_trail_entries: usize,
    pub audit_trail_complete: bool,
    pub verification_passed: bool,
    pub verification_failures: Vec<String>,
}

/// Generate the simulation report from all collected data.
pub fn generate_report(
    profile: &SimProfile,
    elapsed: Duration,
    ctx: &SimContext,
    results: &InjectionResults,
    verification: &VerificationResult,
) -> SimulationReport {
    let total_slots: usize = ctx.lots.iter().map(|(_, slots)| slots.len()).sum();

    // Latency statistics
    let mut sorted_latencies = results.latencies_ms.clone();
    sorted_latencies.sort_unstable();

    let avg_latency = if sorted_latencies.is_empty() {
        0.0
    } else {
        sorted_latencies.iter().sum::<u64>() as f64 / sorted_latencies.len() as f64
    };

    let percentile = |p: f64| -> u64 {
        if sorted_latencies.is_empty() {
            return 0;
        }
        let idx = ((p / 100.0) * sorted_latencies.len() as f64) as usize;
        let idx = idx.min(sorted_latencies.len() - 1);
        sorted_latencies[idx]
    };

    SimulationReport {
        profile: profile.name.to_string(),
        duration_seconds: elapsed.as_secs_f64(),
        total_users: ctx.users.len(),
        total_lots: ctx.lots.len(),
        total_slots,
        total_booking_attempts: results.total_booking_attempts,
        successful_bookings: results.successful_bookings,
        rejected_conflicts: results.rejected_conflicts,
        cancellations: results.cancellations,
        recurring_instances: results.recurring_created,
        waitlist_entries: results.waitlist_entries,
        avg_latency_ms: avg_latency,
        p50_latency_ms: percentile(50.0),
        p95_latency_ms: percentile(95.0),
        p99_latency_ms: percentile(99.0),
        max_latency_ms: sorted_latencies.last().copied().unwrap_or(0),
        errors: results.errors,
        double_bookings: verification.double_bookings_found,
        audit_trail_entries: verification.audit_entries_count,
        audit_trail_complete: verification.audit_matches_operations,
        verification_passed: verification.passed,
        verification_failures: verification.failures.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_serializes_to_json() {
        let report = SimulationReport {
            profile: "test".to_string(),
            duration_seconds: 1.5,
            total_users: 10,
            total_lots: 1,
            total_slots: 20,
            total_booking_attempts: 50,
            successful_bookings: 48,
            rejected_conflicts: 2,
            cancellations: 7,
            recurring_instances: 3,
            waitlist_entries: 1,
            avg_latency_ms: 12.5,
            p50_latency_ms: 10,
            p95_latency_ms: 45,
            p99_latency_ms: 80,
            max_latency_ms: 120,
            errors: 0,
            double_bookings: 0,
            audit_trail_entries: 55,
            audit_trail_complete: true,
            verification_passed: true,
            verification_failures: vec![],
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"profile\": \"test\""));
        assert!(json.contains("\"verification_passed\": true"));
        assert!(json.contains("\"double_bookings\": 0"));
    }

    #[test]
    fn percentile_calculation() {
        let mut latencies: Vec<u64> = (1..=100).collect();
        latencies.sort_unstable();

        let p50_idx = ((50.0 / 100.0) * latencies.len() as f64) as usize;
        assert_eq!(latencies[p50_idx.min(latencies.len() - 1)], 50);

        let p95_idx = ((95.0 / 100.0) * latencies.len() as f64) as usize;
        assert_eq!(latencies[p95_idx.min(latencies.len() - 1)], 95);
    }
}
