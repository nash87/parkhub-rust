//! ParkHub Client
//!
//! Desktop application for parking lot management.
//! Connects to ParkHub Server via HTTP API with autodiscovery.

#![windows_subsystem = "windows"]

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod discovery;
mod server_connection;

slint::include_modules!();

/// Application state
struct AppState {
    /// Connected server (if any)
    server: Option<server_connection::ServerConnection>,
    /// Discovered servers on the network
    discovered_servers: Vec<parkhub_common::ServerInfo>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set DPI awareness before creating any windows (Windows-specific)
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    // Force software renderer for compatibility
    std::env::set_var("SLINT_BACKEND", "winit-software");

    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting ParkHub Client v{}", env!("CARGO_PKG_VERSION"));

    // Create application state
    let state = Arc::new(RwLock::new(AppState {
        server: None,
        discovered_servers: vec![],
    }));

    // Start server discovery in background
    let discovery_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = discovery::discover_servers(discovery_state).await {
            warn!("Server discovery error: {}", e);
        }
    });

    // Create UI
    let ui = MainWindow::new().context("Failed to create main window")?;

    // TODO: Set up UI callbacks and connect to server

    // Run UI event loop
    ui.run().context("UI event loop error")?;

    Ok(())
}
