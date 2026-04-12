//! Integration and simulation test suite for ParkHub.
//!
//! Run integration tests:
//!   cargo test -p parkhub-server --test integration integration
//!
//! Run simulation (small only by default, campus/enterprise are #[ignore]):
//!   cargo test -p parkhub-server --test integration simulation
//!
//! Run all including ignored:
//!   cargo test -p parkhub-server --test integration -- --include-ignored

mod common;
mod integration;
mod simulation;
