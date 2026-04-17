//! Server bootstrap helpers extracted from `main.rs`.
//!
//! This module hosts the ancillary functions invoked by `async fn main()`
//! at startup time — CLI parsing, data-directory resolution, password
//! hashing, first-run seeding, the standalone health-check probe,
//! revocation-store wiring, and the GUI status / setup-wizard windows.
//!
//! `main.rs` keeps the top-level `#[tokio::main]` entry point plus the
//! shared [`crate::AppState`] struct; everything else lives here to keep
//! the binary entry point focused on wiring.

pub(crate) mod cli;
pub(crate) mod health;
pub(crate) mod paths;
pub(crate) mod revocation;
pub(crate) mod seed;

#[cfg(feature = "gui")]
pub(crate) mod setup_wizard;
#[cfg(feature = "gui")]
pub(crate) mod status_gui;

#[cfg(test)]
mod tests;
