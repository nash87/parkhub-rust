//! ParkHub Server
//!
//! Database server with HTTP API and LAN autodiscovery.
//! Can run headless or with a configuration GUI.

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod api;
mod config;
mod db;
mod discovery;
mod tls;

use config::ServerConfig;
use db::Database;
use discovery::MdnsService;

/// Application state shared across handlers
pub struct AppState {
    pub config: ServerConfig,
    pub db: Database,
    pub mdns: Option<MdnsService>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,parkhub_server=debug")
        .init();

    info!("Starting ParkHub Server v{}", env!("CARGO_PKG_VERSION"));

    // Determine data directory
    let data_dir = get_data_directory()?;
    info!("Data directory: {}", data_dir.display());

    // Load or create configuration
    let config_path = data_dir.join("config.toml");
    let config = if config_path.exists() {
        ServerConfig::load(&config_path)?
    } else {
        info!("No configuration found, running setup...");
        #[cfg(feature = "gui")]
        {
            // Run GUI setup wizard
            run_setup_wizard(&data_dir).await?
        }
        #[cfg(not(feature = "gui"))]
        {
            // Create default configuration
            let config = ServerConfig::default();
            config.save(&config_path)?;
            config
        }
    };

    // Initialize database
    let db_path = data_dir.join("parkhub.redb");
    let db = Database::open(&db_path).context("Failed to open database")?;
    info!("Database opened: {}", db_path.display());

    // Start mDNS service for autodiscovery
    let mdns = if config.enable_mdns {
        match MdnsService::new(&config).await {
            Ok(service) => {
                info!("mDNS autodiscovery enabled");
                Some(service)
            }
            Err(e) => {
                warn!("Failed to start mDNS: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create application state
    let state = Arc::new(RwLock::new(AppState { config: config.clone(), db, mdns }));

    // Build the API router
    let app = api::create_router(state.clone());

    // Determine bind address
    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    info!("Server listening on {}", addr);

    // Start server
    if config.enable_tls {
        let tls_config = tls::load_or_create_tls_config(&data_dir).await?;
        info!("TLS enabled");
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        warn!("TLS disabled - connections are not encrypted!");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

/// Get the application data directory
fn get_data_directory() -> Result<PathBuf> {
    // Check for portable mode (data next to executable)
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let portable_data = exe_dir.join("parkhub-data");

    if portable_data.exists() {
        return Ok(portable_data);
    }

    // Use system data directory
    let dirs = directories::ProjectDirs::from("com", "parkhub", "ParkHub Server")
        .context("Could not determine data directory")?;

    let data_dir = dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir)?;

    Ok(data_dir)
}

#[cfg(feature = "gui")]
async fn run_setup_wizard(data_dir: &PathBuf) -> Result<ServerConfig> {
    // TODO: Implement Slint-based setup wizard
    // For now, create default config
    let config = ServerConfig::default();
    let config_path = data_dir.join("config.toml");
    config.save(&config_path)?;
    Ok(config)
}
