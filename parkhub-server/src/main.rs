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

#[cfg(feature = "gui")]
slint::include_modules!();

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
            run_setup_wizard(&data_dir)?
        }
        #[cfg(not(feature = "gui"))]
        {
            // Create default configuration in headless mode
            warn!("Running in headless mode - using default configuration");
            let mut config = ServerConfig::default();
            // Generate a random password for headless mode
            config.admin_password_hash = hash_password("admin")?;
            config.save(&config_path)?;
            info!("Default config saved. Admin credentials: admin/admin");
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
    let state = Arc::new(RwLock::new(AppState {
        config: config.clone(),
        db,
        mdns,
    }));

    // Build the API router
    let app = api::create_router(state.clone());

    // Determine bind address
    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    info!("Server listening on {}", addr);
    info!(
        "Access URL: {}://{}:{}",
        if config.enable_tls { "https" } else { "http" },
        get_local_ip().unwrap_or_else(|| "localhost".to_string()),
        config.port
    );

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
    let exe_dir = std::env::current_exe()?
        .parent()
        .unwrap()
        .to_path_buf();
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

/// Get local IP address
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

/// Hash a password using Argon2
fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;

    Ok(hash.to_string())
}

#[cfg(feature = "gui")]
fn run_setup_wizard(data_dir: &PathBuf) -> Result<ServerConfig> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Get local IP for display
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Create the setup wizard window
    let ui = SetupWizard::new().context("Failed to create setup wizard")?;

    // Set initial values
    ui.set_local_ip(local_ip.into());
    ui.set_port(parkhub_common::DEFAULT_PORT as i32);

    // Store the result
    let result: Rc<RefCell<Option<ServerConfig>>> = Rc::new(RefCell::new(None));
    let result_clone = result.clone();
    let data_dir_clone = data_dir.clone();

    // Handle finish setup
    ui.on_finish_setup(move || {
        let ui = SetupWizard::new().unwrap();

        // Get values from UI
        let server_name = ui.get_server_name().to_string();
        let admin_username = ui.get_admin_username().to_string();
        let admin_password = ui.get_admin_password().to_string();
        let port = ui.get_port() as u16;
        let enable_tls = ui.get_enable_tls();
        let enable_mdns = ui.get_enable_mdns();

        // Hash the password
        let password_hash = match hash_password(&admin_password) {
            Ok(hash) => hash,
            Err(e) => {
                eprintln!("Failed to hash password: {}", e);
                return;
            }
        };

        // Create config
        let config = ServerConfig {
            server_name,
            port,
            enable_tls,
            enable_mdns,
            admin_username,
            admin_password_hash: password_hash,
        };

        // Save config
        let config_path = data_dir_clone.join("config.toml");
        if let Err(e) = config.save(&config_path) {
            eprintln!("Failed to save config: {}", e);
            return;
        }

        *result_clone.borrow_mut() = Some(config);
        slint::quit_event_loop().unwrap();
    });

    // Handle cancel
    ui.on_cancel_setup(|| {
        slint::quit_event_loop().unwrap();
    });

    // Run the UI
    ui.run().context("Setup wizard failed")?;

    // Return the result
    let config = result.borrow().clone();
    config.ok_or_else(|| anyhow::anyhow!("Setup was cancelled"))
}
