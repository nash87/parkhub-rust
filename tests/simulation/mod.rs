//! 1-Month Booking Simulation Engine
//!
//! Exercises the full API over simulated time periods with realistic
//! traffic patterns.  Three profiles: small, campus, enterprise.

pub mod generator;
pub mod injector;
pub mod profiles;
pub mod reporter;
pub mod verifier;

use crate::common::start_test_server;
use profiles::SimProfile;
use reporter::SimulationReport;

/// Run a full simulation with the given profile.
pub async fn run_simulation(profile: &SimProfile) -> SimulationReport {
    let srv = start_test_server().await;
    let start = std::time::Instant::now();

    // Phase 1: Setup — create admin, lots, slots, users
    let ctx = injector::setup_infrastructure(&srv, profile).await;

    // Phase 2: Inject bookings over simulated days
    let results = injector::inject_bookings(&srv, &ctx, profile).await;

    // Phase 3: Verify consistency
    let verification = verifier::verify_consistency(&srv, &ctx, profile, &results).await;

    // Phase 4: Generate report
    reporter::generate_report(profile, start.elapsed(), &ctx, &results, &verification)
}

// ═════════════════════════════════════════════════════════════════════════════
// Simulation test entries
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn simulation_small_profile() {
    let report = run_simulation(&profiles::SMALL).await;

    assert!(report.errors == 0, "Small simulation had {} errors", report.errors);
    assert!(report.verification_passed, "Small simulation verification failed");
    assert_eq!(report.double_bookings, 0, "No double bookings allowed");

    // Print report
    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("=== SMALL PROFILE REPORT ===\n{json}");
}

#[tokio::test]
#[ignore] // Run explicitly: cargo test simulation_campus -- --ignored
async fn simulation_campus_profile() {
    let report = run_simulation(&profiles::CAMPUS).await;

    assert!(report.errors == 0, "Campus simulation had {} errors", report.errors);
    assert!(report.verification_passed, "Campus simulation verification failed");
    assert_eq!(report.double_bookings, 0);

    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("=== CAMPUS PROFILE REPORT ===\n{json}");
}

#[tokio::test]
#[ignore] // Run explicitly: cargo test simulation_enterprise -- --ignored
async fn simulation_enterprise_profile() {
    let report = run_simulation(&profiles::ENTERPRISE).await;

    assert!(report.errors == 0, "Enterprise simulation had {} errors", report.errors);
    assert!(report.verification_passed, "Enterprise simulation verification failed");
    assert_eq!(report.double_bookings, 0);

    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("=== ENTERPRISE PROFILE REPORT ===\n{json}");
}
