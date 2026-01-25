//! ParkHub Client
//!
//! Desktop application for parking lot management.
//! Connects to ParkHub Server via HTTP API with autodiscovery.

#![windows_subsystem = "windows"]

use anyhow::{Context, Result};
use slint::{ModelRc, SharedString, VecModel};
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
    /// Whether we're currently scanning
    is_scanning: bool,
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
        is_scanning: false,
    }));

    // Create UI
    let ui = MainWindow::new().context("Failed to create main window")?;

    // Set up periodic UI update timer to sync discovered servers
    let ui_weak = ui.as_weak();
    let state_for_timer = state.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(500),
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                // Update UI with discovered servers from state
                let state = state_for_timer.blocking_read();
                let servers: Vec<DiscoveredServer> = state
                    .discovered_servers
                    .iter()
                    .map(|s| DiscoveredServer {
                        id: SharedString::from(&s.name),
                        name: SharedString::from(&s.name),
                        host: SharedString::from(&s.host),
                        port: s.port as i32,
                        tls: s.tls,
                        version: SharedString::from(&s.version),
                    })
                    .collect();
                ui.set_discovered_servers(ModelRc::new(VecModel::from(servers)));
                ui.set_is_scanning_servers(state.is_scanning);
            }
        },
    );

    // Start server discovery in background
    let discovery_state = state.clone();
    tokio::spawn(async move {
        {
            let mut state = discovery_state.write().await;
            state.is_scanning = true;
        }
        if let Err(e) = discovery::discover_servers(discovery_state.clone()).await {
            warn!("Server discovery error: {}", e);
        }
        {
            let mut state = discovery_state.write().await;
            state.is_scanning = false;
        }
    });

    // Set up refresh servers callback
    let state_for_refresh = state.clone();
    ui.on_refresh_servers(move || {
        info!("Refreshing server list...");
        let state = state_for_refresh.clone();
        tokio::spawn(async move {
            {
                let mut state = state.write().await;
                state.discovered_servers.clear();
                state.is_scanning = true;
            }
            if let Err(e) = discovery::discover_servers(state.clone()).await {
                warn!("Server discovery error: {}", e);
            }
            {
                let mut state = state.write().await;
                state.is_scanning = false;
            }
        });
    });

    // Set up connect to discovered server callback
    let ui_weak2 = ui.as_weak();
    let state_for_connect = state.clone();
    ui.on_connect_to_server(move |server_id| {
        let server_id = server_id.to_string();
        info!("Connecting to server: {}", server_id);

        if let Some(ui) = ui_weak2.upgrade() {
            ui.set_is_connecting_to_server(true);
            ui.set_connection_error(SharedString::from(""));

            let state = state_for_connect.clone();
            let ui_weak = ui.as_weak();

            tokio::spawn(async move {
                // Find the server info
                let server_info = {
                    let state = state.read().await;
                    state.discovered_servers.iter().find(|s| s.name == server_id).cloned()
                };

                if let Some(info) = server_info {
                    match server_connection::ServerConnection::connect(info).await {
                        Ok(conn) => {
                            {
                                let mut state = state.write().await;
                                state.server = Some(conn);
                            }
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_connecting_to_server(false);
                                ui.set_is_connected(true);
                                ui.set_current_view(AppView::Login);
                            }
                        }
                        Err(e) => {
                            warn!("Connection failed: {}", e);
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_connecting_to_server(false);
                                ui.set_connection_error(SharedString::from(format!("Connection failed: {}", e)));
                            }
                        }
                    }
                } else {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_is_connecting_to_server(false);
                        ui.set_connection_error(SharedString::from("Server not found"));
                    }
                }
            });
        }
    });

    // Set up manual connection callback
    let ui_weak3 = ui.as_weak();
    let state_for_manual = state.clone();
    ui.on_connect_manual(move |host, port, tls| {
        let host = host.to_string();
        info!("Connecting manually to {}:{} (TLS: {})", host, port, tls);

        if let Some(ui) = ui_weak3.upgrade() {
            ui.set_is_connecting_to_server(true);
            ui.set_connection_error(SharedString::from(""));

            let state = state_for_manual.clone();
            let ui_weak = ui.as_weak();

            tokio::spawn(async move {
                let server_info = parkhub_common::ServerInfo {
                    name: format!("{}:{}", host, port),
                    version: "unknown".to_string(),
                    protocol_version: parkhub_common::PROTOCOL_VERSION.to_string(),
                    host,
                    port: port as u16,
                    tls,
                    fingerprint: None,
                };

                match server_connection::ServerConnection::connect(server_info).await {
                    Ok(conn) => {
                        {
                            let mut state = state.write().await;
                            state.server = Some(conn);
                        }
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_is_connecting_to_server(false);
                            ui.set_is_connected(true);
                            ui.set_current_view(AppView::Login);
                        }
                    }
                    Err(e) => {
                        warn!("Connection failed: {}", e);
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_is_connecting_to_server(false);
                            ui.set_connection_error(SharedString::from(format!("Connection failed: {}", e)));
                        }
                    }
                }
            });
        }
    });

    // Set up disconnect callback
    let ui_weak4 = ui.as_weak();
    let state_for_disconnect = state.clone();
    ui.on_disconnect_from_server(move || {
        info!("Disconnecting from server");
        if let Some(ui) = ui_weak4.upgrade() {
            let state = state_for_disconnect.clone();
            tokio::spawn(async move {
                let mut state = state.write().await;
                state.server = None;
            });
            ui.set_is_connected(false);
            ui.set_current_view(AppView::Connect);
        }
    });

    // Run UI event loop
    ui.run().context("UI event loop error")?;

    Ok(())
}
