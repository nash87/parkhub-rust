//! ParkHub Server
//!
//! Database server with HTTP API and LAN autodiscovery.
//! Can run headless or with a configuration GUI.

// Hide console window on Windows when running with GUI
#![cfg_attr(all(feature = "gui", windows), windows_subsystem = "windows")]

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
use db::{Database, DatabaseConfig};
use discovery::MdnsService;

#[cfg(feature = "gui")]
slint::include_modules!();

// System tray support
#[cfg(all(feature = "gui", windows))]
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon,
};

/// Application state shared across handlers
pub struct AppState {
    pub config: ServerConfig,
    pub db: Database,
    pub mdns: Option<MdnsService>,
}

/// CLI arguments for the server
#[derive(Debug, Clone)]
struct CliArgs {
    /// Show help message
    help: bool,
    /// Run in debug mode with verbose logging
    debug: bool,
    /// Run without GUI (headless mode)
    headless: bool,
    /// Run in unattended mode (auto-configure with defaults)
    unattended: bool,
    /// Custom port to listen on
    port: Option<u16>,
    /// Custom data directory
    data_dir: Option<PathBuf>,
    /// Show version
    version: bool,
}

impl CliArgs {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut cli = CliArgs {
            help: false,
            debug: false,
            headless: false,
            unattended: false,
            port: None,
            data_dir: None,
            version: false,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--help" => cli.help = true,
                "-v" | "--version" => cli.version = true,
                "-d" | "--debug" => cli.debug = true,
                "--headless" => cli.headless = true,
                "--unattended" => cli.unattended = true,
                "-p" | "--port" => {
                    if i + 1 < args.len() {
                        cli.port = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--data-dir" => {
                    if i + 1 < args.len() {
                        cli.data_dir = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        cli
    }

    fn print_help() {
        println!("ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("USAGE:");
        println!("    parkhub-server [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help        Show this help message");
        println!("    -v, --version     Show version information");
        println!("    -d, --debug       Enable debug logging");
        println!("    --headless        Run without GUI (console only)");
        println!("    --unattended      Auto-configure with defaults (no setup wizard)");
        println!("    -p, --port PORT   Set the server port (default: 7878)");
        println!("    --data-dir PATH   Set custom data directory");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    PARKHUB_DB_PASSPHRASE    Database encryption passphrase");
        println!("    RUST_LOG                 Logging filter (e.g., debug,info)");
        println!();
        println!("EXAMPLES:");
        println!("    parkhub-server                    # Start with GUI");
        println!("    parkhub-server --headless         # Start in console mode");
        println!("    parkhub-server --debug            # Start with debug logging");
        println!("    parkhub-server --unattended       # Auto-configure and start");
        println!("    parkhub-server -p 8080            # Use port 8080");
    }

    fn print_version() {
        println!("ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
        println!("Protocol Version: {}", parkhub_common::PROTOCOL_VERSION);
        #[cfg(feature = "gui")]
        println!("GUI: enabled");
        #[cfg(not(feature = "gui"))]
        println!("GUI: disabled");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first
    let cli = CliArgs::parse();

    if cli.help {
        CliArgs::print_help();
        return Ok(());
    }

    if cli.version {
        CliArgs::print_version();
        return Ok(());
    }

    // Set DPI awareness before creating any windows (Windows-specific)
    #[cfg(all(feature = "gui", windows))]
    if !cli.headless {
        use windows_sys::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    // Force software renderer since fonts are embedded for it (EmbedForSoftwareRenderer in build.rs)
    // This ensures the app works on systems without GPU/OpenGL support
    #[cfg(feature = "gui")]
    if !cli.headless {
        std::env::set_var("SLINT_BACKEND", "winit-software");
    }

    // Initialize logging based on debug flag
    let log_filter = if cli.debug {
        "debug,parkhub_server=trace"
    } else {
        "info,parkhub_server=debug"
    };

    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| log_filter.to_string()))
        .init();

    info!("Starting ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
    if cli.debug {
        info!("Debug mode enabled");
    }
    if cli.headless {
        info!("Headless mode enabled");
    }
    if cli.unattended {
        info!("Unattended mode enabled");
    }

    // Determine initial data directory (may change if setup wizard runs)
    let mut data_dir = if let Some(ref dir) = cli.data_dir {
        std::fs::create_dir_all(dir)?;
        dir.clone()
    } else {
        get_data_directory(None)?
    };
    info!("Data directory: {}", data_dir.display());

    // Load or create configuration
    let config_path = data_dir.join("config.toml");
    let mut config = if config_path.exists() {
        ServerConfig::load(&config_path)?
    } else if cli.unattended || cli.headless {
        // Unattended/headless mode - auto-configure with defaults
        info!("Auto-configuring with defaults (unattended mode)...");
        let mut config = ServerConfig::default();
        config.server_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "ParkHub Server".to_string());
        config.admin_password_hash = hash_password("admin")?;
        config.encryption_enabled = false; // Disable encryption for unattended setup
        config.enable_tls = false; // Disable TLS for easier initial setup
        config.generate_dummy_users = true;
        config.save(&config_path)?;
        info!("Default config saved. Admin credentials: admin/admin");
        config
    } else {
        info!("No configuration found, running setup...");
        #[cfg(feature = "gui")]
        {
            let wizard_config = run_setup_wizard()?;
            // Update data directory based on portable mode choice
            data_dir = get_data_directory(Some(wizard_config.portable_mode))?;
            let new_config_path = data_dir.join("config.toml");
            wizard_config.save(&new_config_path)?;
            info!("Configuration saved to: {}", new_config_path.display());
            wizard_config
        }
        #[cfg(not(feature = "gui"))]
        {
            // Create default configuration in headless mode
            warn!("Running in headless mode - using default configuration");
            let mut config = ServerConfig::default();
            // Generate a random password for headless mode
            config.admin_password_hash = hash_password("admin")?;
            // Use environment variable for encryption passphrase in headless mode
            config.encryption_passphrase = std::env::var("PARKHUB_DB_PASSPHRASE").ok();
            if config.encryption_enabled && config.encryption_passphrase.is_none() {
                warn!("Database encryption enabled but PARKHUB_DB_PASSPHRASE not set");
                warn!("Using default passphrase - NOT RECOMMENDED FOR PRODUCTION");
                config.encryption_passphrase = Some("default-dev-passphrase".to_string());
            }
            config.save(&config_path)?;
            info!("Default config saved. Admin credentials: admin/admin");
            config
        }
    };

    // Override port if specified on command line
    if let Some(port) = cli.port {
        config.port = port;
        info!("Port overridden from command line: {}", port);
    }

    // If encryption is enabled but no passphrase, try environment variable
    if config.encryption_enabled && config.encryption_passphrase.is_none() {
        config.encryption_passphrase = std::env::var("PARKHUB_DB_PASSPHRASE").ok();
        if config.encryption_passphrase.is_none() {
            #[cfg(feature = "gui")]
            {
                config.encryption_passphrase = Some(prompt_passphrase_gui()?);
            }
            #[cfg(not(feature = "gui"))]
            {
                anyhow::bail!(
                    "Database encryption enabled but no passphrase provided.\n\
                     Set PARKHUB_DB_PASSPHRASE environment variable."
                );
            }
        }
    }

    // Initialize database with encryption
    let db_config = DatabaseConfig {
        path: data_dir.clone(),
        encryption_enabled: config.encryption_enabled,
        passphrase: config.encryption_passphrase.clone(),
        create_if_missing: true,
    };
    let db = Database::open(db_config).context("Failed to open database")?;
    info!(
        "Database opened: {} (encrypted: {})",
        data_dir.display(),
        db.is_encrypted()
    );

    // Create admin user if database is fresh
    if db.is_fresh().await? {
        info!("Creating admin user...");
        create_admin_user(&db, &config).await?;

        // Also create a sample parking lot
        create_sample_parking_lot(&db).await?;

        // Generate dummy users if requested during setup
        if config.generate_dummy_users {
            let style = match config.username_style {
                0 => UsernameStyle::FirstLastLetter,
                1 => UsernameStyle::FirstDotLast,
                2 => UsernameStyle::InitialLast,
                _ => UsernameStyle::FirstInitial,
            };
            generate_dummy_users(&db, style).await?;
        }
    }

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

    // Start server in background task
    let server_config = config.clone();
    let data_dir_for_server = data_dir.clone();
    tokio::spawn(async move {
        if server_config.enable_tls {
            match tls::load_or_create_tls_config(&data_dir_for_server).await {
                Ok(tls_config) => {
                    info!("TLS enabled");
                    if let Err(e) = axum_server::bind_rustls(addr, tls_config)
                        .serve(app.into_make_service())
                        .await
                    {
                        tracing::error!("Server error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load TLS config: {}", e);
                }
            }
        } else {
            warn!("TLS disabled - connections are not encrypted!");
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    if let Err(e) = axum::serve(listener, app).await {
                        tracing::error!("Server error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to bind server: {}", e);
                }
            }
        }
    });

    // Show status GUI or wait for shutdown signal
    #[cfg(feature = "gui")]
    if !cli.headless {
        match run_status_gui(config, state, data_dir).await {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("GUI error: {}", e);
                // Fall back to headless mode on GUI error
                info!("Falling back to headless mode due to GUI error");
                info!("Server running. Press Ctrl+C to stop.");
                tokio::signal::ctrl_c().await?;
                info!("Shutting down...");
            }
        }
    } else {
        // Headless mode requested via CLI
        info!("Server running in headless mode. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await?;
        info!("Shutting down...");
    }

    #[cfg(not(feature = "gui"))]
    {
        // Headless mode - wait forever
        info!("Server running in headless mode. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await?;
        info!("Shutting down...");
    }

    Ok(())
}

/// Get the application data directory
fn get_data_directory(portable_mode: Option<bool>) -> Result<PathBuf> {
    let exe_dir = std::env::current_exe()?
        .parent()
        .unwrap()
        .to_path_buf();
    let portable_data = exe_dir.join("parkhub-data");

    // If portable_mode is explicitly set (from wizard), use that preference
    if let Some(portable) = portable_mode {
        if portable {
            std::fs::create_dir_all(&portable_data)?;
            return Ok(portable_data);
        } else {
            // Use system data directory
            let dirs = directories::ProjectDirs::from("com", "parkhub", "ParkHub Server")
                .context("Could not determine data directory")?;
            let data_dir = dirs.data_dir().to_path_buf();
            std::fs::create_dir_all(&data_dir)?;
            return Ok(data_dir);
        }
    }

    // Auto-detect: Check for existing portable data first
    if portable_data.exists() {
        return Ok(portable_data);
    }

    // Check if config exists in system directory
    let dirs = directories::ProjectDirs::from("com", "parkhub", "ParkHub Server")
        .context("Could not determine data directory")?;
    let data_dir = dirs.data_dir().to_path_buf();

    if data_dir.join("config.toml").exists() {
        return Ok(data_dir);
    }

    // First-time setup - will be determined by wizard
    // For now, default to system directory but wizard will override
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
fn run_setup_wizard() -> Result<ServerConfig> {
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

    // Get a weak reference to the UI for use in the callback
    let ui_weak = ui.as_weak();

    // Handle finish setup
    ui.on_finish_setup(move || {
        // Upgrade the weak reference to access the UI
        let Some(ui) = ui_weak.upgrade() else {
            eprintln!("Failed to access setup wizard UI");
            return;
        };

        // Get values from UI
        let server_name = ui.get_server_name().to_string();
        let admin_username = ui.get_admin_username().to_string();
        let admin_password = ui.get_admin_password().to_string();
        let port = ui.get_port() as u16;
        let enable_tls = ui.get_enable_tls();
        let enable_mdns = ui.get_enable_mdns();

        // Validate inputs
        if server_name.trim().is_empty() {
            eprintln!("Server name cannot be empty");
            return;
        }
        if admin_username.trim().is_empty() {
            eprintln!("Admin username cannot be empty");
            return;
        }
        if admin_password.is_empty() {
            eprintln!("Admin password cannot be empty");
            return;
        }

        // Hash the password
        let password_hash = match hash_password(&admin_password) {
            Ok(hash) => hash,
            Err(e) => {
                eprintln!("Failed to hash password: {}", e);
                return;
            }
        };

        // Get encryption settings
        let enable_encryption = ui.get_enable_encryption();
        let encryption_passphrase = if enable_encryption {
            let passphrase = ui.get_encryption_passphrase().to_string();
            // Additional validation on Rust side
            if passphrase.len() < 8 {
                eprintln!("Encryption passphrase must be at least 8 characters");
                return;
            }
            Some(passphrase)
        } else {
            None
        };

        // Get portable mode and dummy users settings
        let portable_mode = ui.get_use_portable_mode();
        let generate_dummy_users = ui.get_generate_dummy_users();
        let username_style = ui.get_username_style() as u8;
        let license_plate_display = ui.get_license_plate_display() as u8;

        // Create config with defaults for advanced settings
        // (can be changed later via admin panel)
        let config = ServerConfig {
            server_name,
            port,
            enable_tls,
            enable_mdns,
            encryption_enabled: enable_encryption,
            encryption_passphrase,
            admin_username,
            admin_password_hash: password_hash,
            portable_mode,
            generate_dummy_users,
            username_style,
            license_plate_display,
            session_timeout_minutes: 60,  // 1 hour default
            allow_self_registration: false,
            require_email_verification: false,
            max_concurrent_sessions: 0,  // Unlimited
            auto_backup_enabled: true,
            backup_retention_count: 7,
            audit_logging_enabled: true,
            default_language: "en".to_string(),
            organization_name: String::new(),
            close_behavior: "ask".to_string(),
            theme_mode: 0,
            font_scale: 1.0,
            reduce_motion: false,
        };

        *result_clone.borrow_mut() = Some(config);
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {}", e);
        }
    });

    // Handle cancel
    ui.on_cancel_setup(|| {
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {}", e);
        }
    });

    // Run the UI
    ui.run().context("Setup wizard failed")?;

    // Return the result
    let config = result.borrow().clone();
    config.ok_or_else(|| anyhow::anyhow!("Setup was cancelled"))
}

#[cfg(feature = "gui")]
fn prompt_passphrase_gui() -> Result<String> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Create a simple passphrase dialog
    let ui = PassphraseDialog::new().context("Failed to create passphrase dialog")?;

    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let result_clone = result.clone();

    ui.on_submit(move |passphrase| {
        *result_clone.borrow_mut() = Some(passphrase.to_string());
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {}", e);
        }
    });

    ui.on_cancel(|| {
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {}", e);
        }
    });

    ui.run().context("Passphrase dialog failed")?;

    let passphrase = result.borrow().clone();
    passphrase.ok_or_else(|| anyhow::anyhow!("Passphrase entry was cancelled"))
}

/// Run the server status GUI with system tray support
#[cfg(feature = "gui")]
async fn run_status_gui(
    config: ServerConfig,
    state: Arc<RwLock<AppState>>,
    data_dir: PathBuf,
) -> Result<()> {
    use slint::SharedString;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Create the status window
    let ui = ServerStatus::new().context("Failed to create server status window")?;

    // Set initial values
    let local_ip = get_local_ip().unwrap_or_else(|| "localhost".to_string());
    let server_url = format!(
        "{}://{}:{}",
        if config.enable_tls { "https" } else { "http" },
        local_ip,
        config.port
    );

    ui.set_server_name(SharedString::from(&config.server_name));
    ui.set_server_url(SharedString::from(&server_url));
    ui.set_is_running(true);
    ui.set_tls_enabled(config.enable_tls);
    ui.set_mdns_enabled(config.enable_mdns);
    ui.set_encryption_enabled(config.encryption_enabled);

    // Create system tray icon (Windows only) - with error handling
    #[cfg(all(feature = "gui", windows))]
    let _tray_icon: Option<(tray_icon::TrayIcon, slint::Timer)> = {
        // Helper function to create tray icon
        fn create_tray(
            server_name: &str,
            ui: &ServerStatus,
            data_dir: PathBuf,
        ) -> Result<(tray_icon::TrayIcon, slint::Timer), Box<dyn std::error::Error>> {
            // Create tray menu
            let menu_show = MenuItem::new("Show Server Status", true, None);
            let menu_show_id = menu_show.id().clone();
            let menu_data = MenuItem::new("Open Data Folder", true, None);
            let menu_data_id = menu_data.id().clone();
            let menu_stop = MenuItem::new("Stop Server", true, None);
            let menu_stop_id = menu_stop.id().clone();
            let menu_quit = MenuItem::new("Exit", true, None);
            let menu_quit_id = menu_quit.id().clone();

            let tray_menu = Menu::with_items(&[
                &menu_show,
                &PredefinedMenuItem::separator(),
                &menu_data,
                &PredefinedMenuItem::separator(),
                &menu_stop,
                &menu_quit,
            ])?;

            // Create a simple icon (32x32 blue circle with P)
            let icon_data = create_tray_icon_data();
            let icon = Icon::from_rgba(icon_data, 32, 32)?;

            // Build tray icon
            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip(format!("ParkHub Server - {}", server_name))
                .with_icon(icon)
                .build()?;

            // Handle tray menu events
            let ui_weak_menu = ui.as_weak();
            let data_dir_for_menu = data_dir;

            // Set up menu event handler using a timer to poll for events
            let ui_weak_click = ui.as_weak();
            let menu_timer = slint::Timer::default();
            menu_timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(100),
                move || {
                    // Check for tray icon click events (double-click to show window)
                    if let Ok(event) = TrayIconEvent::receiver().try_recv() {
                        match event {
                            TrayIconEvent::DoubleClick { .. } | TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } => {
                                // Click/double-click shows window
                                if let Some(ui) = ui_weak_click.upgrade() {
                                    // First show the window, then unminimize
                                    ui.window().show().ok();
                                    ui.window().set_minimized(false);
                                    // Request redraw to fix any rendering issues
                                    ui.window().request_redraw();
                                    info!("Window restored from tray");
                                }
                            }
                            _ => {}
                        }
                    }

                    // Check for menu events
                    if let Ok(event) = MenuEvent::receiver().try_recv() {
                        if event.id == menu_show_id {
                            // Show window
                            if let Some(ui) = ui_weak_menu.upgrade() {
                                ui.window().show().ok();
                                ui.window().set_minimized(false);
                                ui.window().request_redraw();
                                info!("Window restored from menu");
                            }
                        } else if event.id == menu_data_id {
                            // Open data folder
                            let _ = std::process::Command::new("explorer")
                                .arg(&data_dir_for_menu)
                                .spawn();
                        } else if event.id == menu_stop_id {
                            // Stop server and exit
                            let _ = slint::quit_event_loop();
                        } else if event.id == menu_quit_id {
                            // Exit immediately
                            let _ = slint::quit_event_loop();
                        }
                    }
                },
            );

            Ok((tray, menu_timer))
        }

        // Try to create tray icon, but don't fail if it doesn't work
        match create_tray(&config.server_name, &ui, data_dir.clone()) {
            Ok(tray_and_timer) => {
                info!("System tray icon created successfully");
                Some(tray_and_timer)
            }
            Err(e) => {
                warn!("Failed to create system tray icon: {}. Server will run without tray icon.", e);
                None
            }
        }
    };

    // Set up periodic stats update
    let ui_weak = ui.as_weak();
    let state_for_timer = state.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(2),
        move || {
            let ui_weak_clone = ui_weak.clone();
            let state_clone = state_for_timer.clone();
            // Spawn async stats query without blocking
            tokio::spawn(async move {
                if let Ok(state) = state_clone.try_read() {
                    if let Ok(stats) = state.db.stats().await {
                        // Update UI from event loop thread
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_clone.upgrade() {
                                ui.set_user_count(stats.users as i32);
                                ui.set_booking_count(stats.bookings as i32);
                                ui.set_parking_lot_count(stats.parking_lots as i32);
                                ui.set_slot_count(stats.slots as i32);
                                ui.set_session_count(stats.sessions as i32);
                            }
                        });
                    }
                }
            });
        },
    );

    // Handle minimize to tray - minimize window (tray icon allows restore)
    let ui_weak_tray = ui.as_weak();
    ui.on_minimize_to_tray(move || {
        info!("Minimize to tray button clicked");
        if let Some(ui) = ui_weak_tray.upgrade() {
            // Minimize the window - tray icon will still be visible
            // User can click tray icon or use context menu to restore
            ui.window().set_minimized(true);
            info!("Window minimized - click tray icon to restore");
        } else {
            warn!("Failed to upgrade UI weak reference");
        }
    });

    // Handle stop server
    let should_exit = Rc::new(RefCell::new(false));
    let should_exit_clone = should_exit.clone();
    ui.on_stop_server(move || {
        *should_exit_clone.borrow_mut() = true;
        let _ = slint::quit_event_loop();
    });

    // Handle open data folder
    let data_dir_clone = data_dir.clone();
    ui.on_open_data_folder(move || {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(&data_dir_clone)
                .spawn();
        }
        #[cfg(not(windows))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&data_dir_clone)
                .spawn();
        }
    });

    // Handle close requested (when user clicks X button)
    let ui_weak_close = ui.as_weak();
    let config_path_for_close = data_dir.join("config.toml");
    ui.on_close_requested(move || {
        if let Some(ui) = ui_weak_close.upgrade() {
            let behavior = ui.get_close_behavior();
            match behavior.as_str() {
                "minimize" => {
                    // User chose to always minimize
                    ui.invoke_minimize_to_tray();
                }
                "exit" => {
                    // User chose to always exit
                    let _ = slint::quit_event_loop();
                }
                _ => {
                    // "ask" - show the dialog
                    ui.set_show_close_dialog(true);
                }
            }

            // If user checked "remember", save the config
            if ui.get_remember_close_choice() {
                let new_behavior = ui.get_close_behavior();
                if new_behavior != "ask" {
                    // Save the preference to config
                    if let Ok(mut config) = ServerConfig::load(&config_path_for_close) {
                        config.close_behavior = new_behavior.to_string();
                        let _ = config.save(&config_path_for_close);
                        info!("Close behavior saved: {}", new_behavior);
                    }
                }
            }
        }
    });

    // Load saved close behavior from config
    ui.set_close_behavior(config.close_behavior.clone().into());

    // Load accessibility settings from config via ThemeSettings global
    ui.global::<ThemeSettings>().set_mode(config.theme_mode);
    ui.global::<ThemeSettings>().set_font_scale(config.font_scale);
    ui.global::<ThemeSettings>().set_reduce_motion(config.reduce_motion);
    info!(
        "Loaded theme settings: mode={}, font_scale={}, reduce_motion={}",
        config.theme_mode, config.font_scale, config.reduce_motion
    );

    // Handle save accessibility settings
    let ui_weak_a11y = ui.as_weak();
    let config_path_for_a11y = data_dir.join("config.toml");
    ui.on_save_accessibility_settings(move || {
        if let Some(ui) = ui_weak_a11y.upgrade() {
            let theme_mode = ui.global::<ThemeSettings>().get_mode();
            let font_scale = ui.global::<ThemeSettings>().get_font_scale();
            let reduce_motion = ui.global::<ThemeSettings>().get_reduce_motion();

            // Save to config file
            if let Ok(mut config) = ServerConfig::load(&config_path_for_a11y) {
                config.theme_mode = theme_mode;
                config.font_scale = font_scale;
                config.reduce_motion = reduce_motion;
                if let Err(e) = config.save(&config_path_for_a11y) {
                    warn!("Failed to save accessibility settings: {}", e);
                } else {
                    info!(
                        "Saved accessibility settings: mode={}, font_scale={}, reduce_motion={}",
                        theme_mode, font_scale, reduce_motion
                    );
                }
            }
        }
    });

    // Intercept window close button (X)
    let ui_weak_window_close = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = ui_weak_window_close.upgrade() {
            // Trigger our close_requested handler
            ui.invoke_close_requested();
            // Don't actually close - let the handler decide
            slint::CloseRequestResponse::KeepWindowShown
        } else {
            slint::CloseRequestResponse::HideWindow
        }
    });

    // Run the UI event loop
    ui.run().context("Server status window failed")?;

    if *should_exit.borrow() {
        info!("Server stopped by user");
    }

    Ok(())
}

/// Create icon data for the system tray (32x32 RGBA)
/// Creates a professional parking icon with a blue rounded square and white "P"
#[cfg(all(feature = "gui", windows))]
fn create_tray_icon_data() -> Vec<u8> {
    let size: usize = 32;
    let mut data = vec![0u8; size * size * 4];

    // Colors
    let bg_r: u8 = 0x1a;
    let bg_g: u8 = 0x73;
    let bg_b: u8 = 0xe8; // Bright blue for visibility

    let corner_radius = 6.0f32;

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let fx = x as f32;
            let fy = y as f32;
            let fsize = size as f32;

            // Calculate distance to rounded rectangle
            let in_rounded_rect = {
                let margin = 1.0f32;
                let inner_x = fx.clamp(margin + corner_radius, fsize - margin - corner_radius - 1.0);
                let inner_y = fy.clamp(margin + corner_radius, fsize - margin - corner_radius - 1.0);
                let dx = fx - inner_x;
                let dy = fy - inner_y;
                let dist = (dx * dx + dy * dy).sqrt();

                fx >= margin
                    && fx < fsize - margin
                    && fy >= margin
                    && fy < fsize - margin
                    && dist <= corner_radius + 0.5
            };

            if in_rounded_rect {
                // Check if we're drawing the "P" letter
                let is_p = is_letter_p(x as i32, y as i32, size as i32);

                if is_p {
                    // White "P"
                    data[idx] = 255;     // R
                    data[idx + 1] = 255; // G
                    data[idx + 2] = 255; // B
                    data[idx + 3] = 255; // A
                } else {
                    // Blue background with slight gradient
                    let gradient = 1.0 - (fy / fsize) * 0.15;
                    data[idx] = (bg_r as f32 * gradient) as u8;
                    data[idx + 1] = (bg_g as f32 * gradient) as u8;
                    data[idx + 2] = (bg_b as f32 * gradient) as u8;
                    data[idx + 3] = 255;
                }
            } else {
                // Transparent outside
                data[idx] = 0;
                data[idx + 1] = 0;
                data[idx + 2] = 0;
                data[idx + 3] = 0;
            }
        }
    }

    data
}

/// Check if a pixel is part of the "P" letter
#[cfg(all(feature = "gui", windows))]
fn is_letter_p(x: i32, y: i32, size: i32) -> bool {
    // P dimensions relative to 32x32
    let p_left = size / 4;        // 8
    let p_right = size * 3 / 4;   // 24
    let p_top = size / 5;         // 6
    let p_bottom = size * 4 / 5;  // 25
    let p_middle = size / 2 + 1;  // 17 - middle of the P bowl
    let stroke = size / 8;        // 4 - stroke width

    // Vertical bar of P (left side)
    let vertical_bar = x >= p_left && x < p_left + stroke && y >= p_top && y < p_bottom;

    // Top horizontal bar
    let top_bar = x >= p_left && x < p_right - stroke && y >= p_top && y < p_top + stroke;

    // Middle horizontal bar (end of P bowl)
    let middle_bar = x >= p_left && x < p_right - stroke && y >= p_middle - stroke && y < p_middle;

    // Right vertical part of P bowl
    let right_bar = x >= p_right - stroke - stroke && x < p_right - stroke && y >= p_top && y < p_middle;

    // Rounded corner at top-right
    let tr_cx = (p_right - stroke - stroke) as f32;
    let tr_cy = (p_top + stroke) as f32;
    let tr_r = stroke as f32;
    let tr_dist = ((x as f32 - tr_cx).powi(2) + (y as f32 - tr_cy).powi(2)).sqrt();
    let top_right_curve = x >= p_right - stroke - stroke
        && y >= p_top
        && y < p_top + stroke
        && tr_dist <= tr_r + 0.5;

    // Rounded corner at bottom-right of bowl
    let br_cx = (p_right - stroke - stroke) as f32;
    let br_cy = (p_middle - stroke) as f32;
    let br_dist = ((x as f32 - br_cx).powi(2) + (y as f32 - br_cy).powi(2)).sqrt();
    let bottom_right_curve = x >= p_right - stroke - stroke
        && y >= p_middle - stroke
        && y < p_middle
        && br_dist <= tr_r + 0.5;

    vertical_bar || top_bar || middle_bar || right_bar || top_right_curve || bottom_right_curve
}

/// Create the admin user in the database
async fn create_admin_user(db: &Database, config: &ServerConfig) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{User, UserPreferences, UserRole};
    use uuid::Uuid;

    let admin_user = User {
        id: Uuid::new_v4(),
        username: config.admin_username.clone(),
        email: format!("{}@localhost", config.admin_username),
        password_hash: config.admin_password_hash.clone(),
        name: "Administrator".to_string(),
        picture: None,
        phone: None,
        role: UserRole::SuperAdmin,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        preferences: UserPreferences::default(),
        is_active: true,
    };

    db.save_user(&admin_user).await?;
    db.mark_setup_completed().await?;
    info!(
        "Admin user '{}' created successfully",
        config.admin_username
    );
    Ok(())
}

/// Username generation styles for dummy users
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsernameStyle {
    /// First letter + last letter (e.g., "Alex Smith" -> "ah")
    FirstLastLetter,
    /// First name + last name (e.g., "Alex Smith" -> "alex.smith")
    FirstDotLast,
    /// First letter + last name (e.g., "Alex Smith" -> "asmith")
    InitialLast,
    /// First name + last initial (e.g., "Alex Smith" -> "alexs")
    FirstInitial,
}

impl UsernameStyle {
    /// Generate username from first and last name
    fn generate(&self, first: &str, last: &str, index: usize) -> String {
        let base = match self {
            UsernameStyle::FirstLastLetter => {
                let first_char = first.chars().next().unwrap_or('x').to_lowercase().next().unwrap();
                let last_char = last.chars().last().unwrap_or('x').to_lowercase().next().unwrap();
                format!("{}{}", first_char, last_char)
            }
            UsernameStyle::FirstDotLast => {
                format!("{}.{}", first.to_lowercase(), last.to_lowercase())
            }
            UsernameStyle::InitialLast => {
                let first_char = first.chars().next().unwrap_or('x').to_lowercase().next().unwrap();
                format!("{}{}", first_char, last.to_lowercase())
            }
            UsernameStyle::FirstInitial => {
                let last_char = last.chars().next().unwrap_or('x').to_lowercase().next().unwrap();
                format!("{}{}", first.to_lowercase(), last_char)
            }
        };
        // Add index to ensure uniqueness
        format!("{}{}", base, index + 1)
    }
}

/// Generate 50 GDPR-compliant dummy users for testing
/// All users have password "12351235" and can login immediately
async fn generate_dummy_users(db: &Database, username_style: UsernameStyle) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{User, UserPreferences, UserRole};
    use rand::Rng;
    use uuid::Uuid;

    // GDPR-compliant fictional first names (common, not identifying real people)
    let first_names = [
        "Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Quinn", "Avery",
        "Skyler", "Dakota", "Cameron", "Reese", "Parker", "Hayden", "Sage", "River",
        "Phoenix", "Blake", "Drew", "Jamie", "Robin", "Charlie", "Sam", "Pat",
        "Chris", "Lee", "Kim", "Ashley", "Lynn", "Terry", "Jesse", "Dana",
        "Kelly", "Shannon", "Shawn", "Logan", "Peyton", "Kendall", "Reagan", "Finley",
        "Emerson", "Ellis", "Rowan", "Ainsley", "Blair", "Devon", "Eden", "Gray",
        "Harper", "Indigo",
    ];

    // GDPR-compliant fictional last names (common, not identifying real people)
    let last_names = [
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
        "Rodriguez", "Martinez", "Anderson", "Taylor", "Thomas", "Jackson", "White", "Harris",
        "Martin", "Thompson", "Moore", "Young", "Allen", "King", "Wright", "Scott",
        "Green", "Baker", "Adams", "Nelson", "Hill", "Ramirez", "Campbell", "Mitchell",
        "Roberts", "Carter", "Phillips", "Evans", "Turner", "Torres", "Parker", "Collins",
        "Edwards", "Stewart", "Flores", "Morris", "Murphy", "Rivera", "Cook", "Rogers",
        "Morgan", "Peterson",
    ];

    // Default password for all dummy users - they can login with this
    let default_password = "12351235";
    let password_hash = hash_password(default_password)?;

    // Role distribution: mostly Users, some Premium, few Admin
    let roles = [
        UserRole::User, UserRole::User, UserRole::User, UserRole::User,
        UserRole::Premium, UserRole::Admin,
    ];
    let mut rng = rand::thread_rng();

    info!("Generating 50 GDPR-compliant dummy users (password: {})...", default_password);

    for i in 0..50 {
        let first = first_names[rng.gen_range(0..first_names.len())];
        let last = last_names[rng.gen_range(0..last_names.len())];
        let role = roles[rng.gen_range(0..roles.len())].clone();

        // Generate username based on selected style
        let username = username_style.generate(first, last, i);
        let email = format!("{}@example.com", username);

        let user = User {
            id: Uuid::new_v4(),
            username: username.clone(),
            email,
            password_hash: password_hash.clone(),
            name: format!("{} {}", first, last),
            picture: None,
            phone: Some(format!("+1-555-{:04}", rng.gen_range(1000..9999))),
            role,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            preferences: UserPreferences::default(),
            is_active: true,
        };

        db.save_user(&user).await?;
    }

    info!("Created 50 dummy users successfully");
    info!("Default login: any username with password '{}'", default_password);
    Ok(())
}

/// Create a sample parking lot for testing
async fn create_sample_parking_lot(db: &Database) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{
        DayHours, LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo,
        PricingRate, SlotFeature, SlotPosition, SlotStatus, SlotType,
    };
    use uuid::Uuid;

    let lot_id = Uuid::new_v4();
    let floor_id = Uuid::new_v4();

    // Create 10 parking slots
    let mut slots = Vec::new();
    for i in 1..=10 {
        slots.push(ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: i,
            row: (i - 1) / 5,
            column: (i - 1) % 5,
            slot_type: if i == 1 {
                SlotType::Handicap
            } else if i == 10 {
                SlotType::Electric
            } else {
                SlotType::Standard
            },
            status: SlotStatus::Available,
            current_booking: None,
            features: if i <= 2 {
                vec![SlotFeature::NearExit]
            } else {
                vec![]
            },
            position: SlotPosition {
                x: ((i - 1) % 5) as f32 * 80.0,
                y: ((i - 1) / 5) as f32 * 100.0,
                width: 70.0,
                height: 90.0,
                rotation: 0.0,
            },
        });
    }

    let floor = ParkingFloor {
        id: floor_id,
        lot_id,
        name: "Ground Floor".to_string(),
        floor_number: 0,
        total_slots: 10,
        available_slots: 10,
        slots: slots.clone(),
    };

    let lot = ParkingLot {
        id: lot_id,
        name: "Home Parking".to_string(),
        address: "123 Main Street".to_string(),
        latitude: 0.0,
        longitude: 0.0,
        total_slots: 10,
        available_slots: 10,
        floors: vec![floor],
        amenities: vec!["Security".to_string(), "Covered".to_string()],
        pricing: PricingInfo {
            currency: "EUR".to_string(),
            rates: vec![
                PricingRate {
                    duration_minutes: 60,
                    price: 2.0,
                    label: "1 hour".to_string(),
                },
                PricingRate {
                    duration_minutes: 120,
                    price: 3.5,
                    label: "2 hours".to_string(),
                },
                PricingRate {
                    duration_minutes: 240,
                    price: 6.0,
                    label: "4 hours".to_string(),
                },
            ],
            daily_max: Some(15.0),
            monthly_pass: Some(200.0),
        },
        operating_hours: OperatingHours {
            is_24h: true,
            monday: None,
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        },
        images: vec![],
        status: LotStatus::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Save parking lot
    db.save_parking_lot(&lot).await?;

    // Save all slots
    for slot in &slots {
        db.save_parking_slot(slot).await?;
    }

    info!("Sample parking lot created with {} slots", slots.len());
    Ok(())
}
