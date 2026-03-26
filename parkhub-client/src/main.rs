//! ParkHub Client
//!
//! Desktop application for parking lot management.
//! Connects to ParkHub Server via HTTP API with autodiscovery.

#![windows_subsystem = "windows"]
#![allow(unsafe_code)] // Slint uses unsafe FFI for native window management

use anyhow::{Context, Result};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use slint::{ModelRc, SharedString, VecModel};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod discovery;
#[allow(dead_code)]
mod server_connection;

slint::include_modules!();

/// Accessibility settings stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessibilitySettings {
    /// Theme mode: 0=Dark, 1=Light, 2=High Contrast, 3=Deuteranopia, 4=Protanopia, 5=Tritanopia
    #[serde(default)]
    theme_mode: i32,
    /// Font scale: 1.0=Normal, 1.25=Large, 1.5=Extra Large
    #[serde(default = "default_font_scale")]
    font_scale: f32,
    /// Reduce motion animations
    #[serde(default)]
    reduce_motion: bool,
}

const fn default_font_scale() -> f32 {
    1.0
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            theme_mode: 0,
            font_scale: 1.0,
            reduce_motion: false,
        }
    }
}

/// Application state
struct AppState {
    /// Connected server (if any)
    server: Option<server_connection::ServerConnection>,
    /// Discovered servers on the network
    discovered_servers: Vec<parkhub_common::ServerInfo>,
    /// Whether we're currently scanning
    is_scanning: bool,
    /// Cached full user list for search filtering
    admin_users_cache: Vec<parkhub_common::User>,
}

fn role_label(role: &parkhub_common::UserRole) -> &'static str {
    match role {
        parkhub_common::UserRole::User => "User",
        parkhub_common::UserRole::Premium => "Premium",
        parkhub_common::UserRole::Admin => "Admin",
        parkhub_common::UserRole::SuperAdmin => "SuperAdmin",
    }
}

fn build_admin_user_info(user: &parkhub_common::User) -> AdminUserInfo {
    AdminUserInfo {
        id: SharedString::from(user.id.to_string()),
        username: SharedString::from(&user.username),
        email: SharedString::from(&user.email),
        name: SharedString::from(&user.name),
        initial: SharedString::from(
            user.name
                .chars()
                .next()
                .or_else(|| user.username.chars().next())
                .map_or_else(|| "?".to_string(), |c| c.to_uppercase().to_string()),
        ),
        role: SharedString::from(role_label(&user.role)),
        is_active: user.is_active,
        last_login: SharedString::from(user.last_login.map_or_else(
            || "-".to_string(),
            |dt| dt.format("%d.%m.%Y %H:%M").to_string(),
        )),
        created_at: SharedString::from(user.created_at.format("%d.%m.%Y").to_string()),
    }
}

fn render_admin_users(ui: &MainWindow, users: &[parkhub_common::User]) {
    let user_data: Vec<AdminUserInfo> = users.iter().map(build_admin_user_info).collect();
    ui.set_admin_users(ModelRc::new(VecModel::from(user_data)));
}

fn normalize_admin_role(role: &str) -> Result<&'static str> {
    match role.trim().to_ascii_lowercase().as_str() {
        "user" => Ok("user"),
        "premium" => Ok("premium"),
        "admin" => Ok("admin"),
        "superadmin" | "super_admin" => Ok("superadmin"),
        other => Err(anyhow::anyhow!(
            "Unsupported role '{other}'. Use user, premium, admin, or superadmin."
        )),
    }
}

fn show_success_dialog(
    ui_weak: slint::Weak<MainWindow>,
    title: impl Into<String>,
    message: impl Into<String>,
) {
    let title = title.into();
    let message = message.into();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_dialog_title(SharedString::from(title));
            ui.set_dialog_message(SharedString::from(message));
            ui.set_show_error_dialog(false);
            ui.set_show_success_dialog(true);
        }
    });
}

fn show_error_dialog(
    ui_weak: slint::Weak<MainWindow>,
    title: impl Into<String>,
    message: impl Into<String>,
) {
    let title = title.into();
    let message = message.into();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_dialog_title(SharedString::from(title));
            ui.set_dialog_message(SharedString::from(message));
            ui.set_show_success_dialog(false);
            ui.set_show_error_dialog(true);
        }
    });
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

    // Try skia renderer first (DirectX on Windows), fallback to software
    std::env::set_var("SLINT_BACKEND", "winit-skia");

    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting ParkHub Client v{}", env!("CARGO_PKG_VERSION"));

    // Create application state
    let state = Arc::new(RwLock::new(AppState {
        server: None,
        discovered_servers: vec![],
        is_scanning: false,
        admin_users_cache: vec![],
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
                // Try to read state without blocking - skip update if locked
                if let Ok(state) = state_for_timer.try_read() {
                    let servers: Vec<DiscoveredServer> = state
                        .discovered_servers
                        .iter()
                        .map(|s| DiscoveredServer {
                            id: SharedString::from(&s.name),
                            name: SharedString::from(&s.name),
                            host: SharedString::from(&s.host),
                            port: i32::from(s.port),
                            tls: s.tls,
                            version: SharedString::from(&s.version),
                        })
                        .collect();
                    ui.set_discovered_servers(ModelRc::new(VecModel::from(servers)));
                    ui.set_is_scanning_servers(state.is_scanning);
                }
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

    // Set up window control callbacks

    // Minimize window
    let ui_weak_min = ui.as_weak();
    ui.on_minimize_window(move || {
        if let Some(ui) = ui_weak_min.upgrade() {
            ui.window().set_minimized(true);
        }
    });

    // Maximize/restore window
    let ui_weak_max = ui.as_weak();
    ui.on_maximize_window(move || {
        if let Some(ui) = ui_weak_max.upgrade() {
            let is_maximized = ui.window().is_maximized();
            ui.window().set_maximized(!is_maximized);
        }
    });

    // Close window
    ui.on_close_window(move || {
        slint::quit_event_loop().unwrap();
    });

    // Start window drag (for custom title bar dragging)
    ui.on_start_window_drag(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::HWND;
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetForegroundWindow, SendMessageW, HTCAPTION, WM_NCLBUTTONDOWN,
            };

            unsafe {
                let hwnd: HWND = GetForegroundWindow();
                if hwnd != std::ptr::null_mut() {
                    // Release mouse capture and send "click on title bar" message
                    ReleaseCapture();
                    SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
                }
            }
        }
    });

    // Screenshot callback (placeholder)
    let ui_weak_screenshot = ui.as_weak();
    ui.on_take_screenshot(move || {
        if let Some(ui) = ui_weak_screenshot.upgrade() {
            // For now just show a notification that screenshot was taken
            ui.set_show_screenshot_notification(true);
            ui.set_screenshot_path(SharedString::from("Screenshot feature not yet implemented"));
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
                    state
                        .discovered_servers
                        .iter()
                        .find(|s| s.name == server_id)
                        .cloned()
                };

                if let Some(info) = server_info {
                    match server_connection::ServerConnection::connect(info.clone()).await {
                        Ok(conn) => {
                            let base_url = conn.base_url().to_string();
                            {
                                let mut state = state.write().await;
                                state.server = Some(conn);
                            }
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_is_connecting_to_server(false);
                                    ui.set_is_connected(true);
                                    ui.set_server_url(SharedString::from(base_url));
                                    ui.set_current_view(AppView::Login);
                                }
                            });
                        }
                        Err(e) => {
                            warn!("Connection failed: {}", e);
                            let error_msg = format!("Connection failed: {e}");
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_is_connecting_to_server(false);
                                    ui.set_connection_error(SharedString::from(error_msg));
                                }
                            });
                        }
                    }
                } else {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_is_connecting_to_server(false);
                            ui.set_connection_error(SharedString::from("Server not found"));
                        }
                    });
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
                    name: format!("{host}:{port}"),
                    version: "unknown".to_string(),
                    protocol_version: parkhub_common::PROTOCOL_VERSION.to_string(),
                    host,
                    port: u16::try_from(port).unwrap_or(8443),
                    tls,
                    fingerprint: None,
                };

                match server_connection::ServerConnection::connect(server_info).await {
                    Ok(conn) => {
                        let base_url = conn.base_url().to_string();
                        {
                            let mut state = state.write().await;
                            state.server = Some(conn);
                        }
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_connecting_to_server(false);
                                ui.set_is_connected(true);
                                ui.set_server_url(SharedString::from(base_url));
                                ui.set_current_view(AppView::Login);
                            }
                        });
                    }
                    Err(e) => {
                        warn!("Connection failed: {}", e);
                        let error_msg = format!("Connection failed: {e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_is_connecting_to_server(false);
                                ui.set_connection_error(SharedString::from(error_msg));
                            }
                        });
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
            ui.set_is_authenticated(false);
            ui.set_current_view(AppView::Connect);
        }
    });

    // Set up login callback
    let ui_weak5 = ui.as_weak();
    let state_for_login = state.clone();
    ui.on_login(move |username, password| {
        let username = username.to_string();
        let password = password.to_string();
        info!("Logging in as: {}", username);

        if let Some(ui) = ui_weak5.upgrade() {
            ui.set_login_loading(true);
            ui.set_login_error(SharedString::from(""));

            let state = state_for_login.clone();
            let ui_weak = ui.as_weak();

            tokio::spawn(async move {
                let result = {
                    let mut state = state.write().await;
                    if let Some(ref mut server) = state.server {
                        Some(server.login(&username, &password).await)
                    } else {
                        None
                    }
                };

                match result {
                    Some(Ok(user)) => {
                        info!("Login successful for user: {}", user.username);
                        let state_for_load = state.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_login_loading(false);
                                ui.set_is_authenticated(true);
                                ui.set_current_user(CurrentUser {
                                    id: SharedString::from(user.id.to_string()),
                                    email: SharedString::from(&user.email),
                                    name: SharedString::from(&user.name),
                                    initial: SharedString::from(
                                        user.name.chars().next().unwrap_or('?').to_string(),
                                    ),
                                    picture: SharedString::from(""),
                                    role: SharedString::from(format!("{:?}", user.role)),
                                });
                                ui.set_current_view(AppView::Parking);

                                // Load parking data
                                let ui_weak_load = ui.as_weak();
                                tokio::spawn(async move {
                                    load_parking_data(state_for_load, ui_weak_load).await;
                                });
                            }
                        });
                    }
                    Some(Err(e)) => {
                        warn!("Login failed: {}", e);
                        let error_msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_login_loading(false);
                                ui.set_login_error(SharedString::from(error_msg));
                            }
                        });
                    }
                    None => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_login_loading(false);
                                ui.set_login_error(SharedString::from("Not connected to server"));
                            }
                        });
                    }
                }
            });
        }
    });

    // Set up register callback
    let ui_weak6 = ui.as_weak();
    let state_for_register = state.clone();
    ui.on_register(move |username, password, email, name| {
        let username = username.to_string();
        let password = password.to_string();
        let email = email.to_string();
        let name = name.to_string();
        info!("Registering new user: {}", username);

        if let Some(ui) = ui_weak6.upgrade() {
            ui.set_login_loading(true);
            ui.set_login_error(SharedString::from(""));

            let state = state_for_register.clone();
            let ui_weak = ui.as_weak();

            tokio::spawn(async move {
                let result = {
                    let mut state = state.write().await;
                    if let Some(ref mut server) = state.server {
                        Some(server.register(&username, &password, &email, &name).await)
                    } else {
                        None
                    }
                };

                match result {
                    Some(Ok(user)) => {
                        info!("Registration successful for user: {}", user.username);
                        let state_for_load = state.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_login_loading(false);
                                ui.set_is_authenticated(true);
                                ui.set_current_user(CurrentUser {
                                    id: SharedString::from(user.id.to_string()),
                                    email: SharedString::from(&user.email),
                                    name: SharedString::from(&user.name),
                                    initial: SharedString::from(
                                        user.name.chars().next().unwrap_or('?').to_string(),
                                    ),
                                    picture: SharedString::from(""),
                                    role: SharedString::from(format!("{:?}", user.role)),
                                });
                                ui.set_current_view(AppView::Parking);

                                // Load parking data
                                let ui_weak_load = ui.as_weak();
                                tokio::spawn(async move {
                                    load_parking_data(state_for_load, ui_weak_load).await;
                                });
                            }
                        });
                    }
                    Some(Err(e)) => {
                        warn!("Registration failed: {}", e);
                        let error_msg = format!("{e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_login_loading(false);
                                ui.set_login_error(SharedString::from(error_msg));
                            }
                        });
                    }
                    None => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_login_loading(false);
                                ui.set_login_error(SharedString::from("Not connected to server"));
                            }
                        });
                    }
                }
            });
        }
    });

    // Set up toggle register callback
    let ui_weak7 = ui.as_weak();
    ui.on_toggle_register(move || {
        if let Some(ui) = ui_weak7.upgrade() {
            let current = ui.get_show_register();
            ui.set_show_register(!current);
            ui.set_login_error(SharedString::from(""));
        }
    });

    // Set up logout callback
    let ui_weak8 = ui.as_weak();
    let state_for_logout = state.clone();
    ui.on_logout(move || {
        info!("Logging out");
        if let Some(ui) = ui_weak8.upgrade() {
            let state = state_for_logout.clone();
            tokio::spawn(async move {
                let mut state = state.write().await;
                state.server = None;
            });
            ui.set_is_authenticated(false);
            ui.set_is_connected(false);
            ui.set_current_view(AppView::Connect);
        }
    });

    // =========================================================================
    // Admin User Management Callbacks
    // =========================================================================

    // Load users callback
    let ui_weak_admin1 = ui.as_weak();
    let state_for_admin_users = state.clone();
    ui.on_admin_load_users(move || {
        info!("Loading admin users list");
        let state = state_for_admin_users.clone();
        let ui_weak = ui_weak_admin1.clone();

        tokio::spawn(async move {
            let users_result = {
                let state = state.read().await;
                if let Some(ref server) = state.server {
                    Some(server.list_users().await)
                } else {
                    None
                }
            };

            if let Some(result) = users_result {
                match result {
                    Ok(users) => {
                        // Save to cache for search filtering
                        {
                            let mut state = state.write().await;
                            state.admin_users_cache.clone_from(&users);
                        }

                        if let Some(ui) = ui_weak.upgrade() {
                            render_admin_users(&ui, &users);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load users: {}", e);
                    }
                }
            }
        });
    });

    // Edit user callback
    let ui_weak_admin2 = ui.as_weak();
    let state_for_edit = state.clone();
    ui.on_admin_edit_user(move |user_id| {
        let user_id = user_id.to_string();
        info!("Edit user: {}", user_id);
        let state = state_for_edit.clone();
        let ui_weak = ui_weak_admin2.clone();

        tokio::spawn(async move {
            let user = {
                let state = state.read().await;
                state
                    .admin_users_cache
                    .iter()
                    .find(|u| u.id.to_string() == user_id)
                    .cloned()
            };

            match user {
                Some(user) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_admin_user_edit_mode(true);
                            ui.set_admin_user_form_id(SharedString::from(user.id.to_string()));
                            ui.set_admin_user_form_title(SharedString::from("Benutzer bearbeiten"));
                            ui.set_admin_user_form_username(SharedString::from(user.username));
                            ui.set_admin_user_form_name(SharedString::from(user.name));
                            ui.set_admin_user_form_email(SharedString::from(user.email));
                            ui.set_admin_user_form_role(SharedString::from(
                                normalize_admin_role(role_label(&user.role)).unwrap_or("user"),
                            ));
                            ui.set_show_admin_user_dialog(true);
                        }
                    });
                }
                None => show_error_dialog(
                    ui_weak,
                    "Benutzer nicht gefunden",
                    "Der ausgewählte Benutzer ist nicht mehr im lokalen Admin-Cache vorhanden.",
                ),
            }
        });
    });

    // Delete user callback
    let ui_weak_admin3 = ui.as_weak();
    let state_for_delete = state.clone();
    ui.on_admin_delete_user(move |user_id| {
        let user_id = user_id.to_string();
        info!("Delete user: {}", user_id);

        let state = state_for_delete.clone();
        let ui_weak = ui_weak_admin3.clone();

        tokio::spawn(async move {
            let refresh_result = {
                let state = state.read().await;
                if let Some(ref server) = state.server {
                    match server.delete_user(&user_id).await {
                        Ok(()) => {
                            info!("User {} deleted successfully", user_id);
                            Some(server.list_users().await)
                        }
                        Err(e) => {
                            warn!("Failed to delete user: {}", e);
                            show_error_dialog(
                                ui_weak.clone(),
                                "Löschen fehlgeschlagen",
                                e.to_string(),
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            };

            if let Some(result) = refresh_result {
                match result {
                    Ok(users) => {
                        {
                            let mut state = state.write().await;
                            state.admin_users_cache.clone_from(&users);
                        }
                        if let Some(ui) = ui_weak.upgrade() {
                            render_admin_users(&ui, &users);
                        }
                    }
                    Err(e) => show_error_dialog(
                        ui_weak,
                        "Benutzerliste konnte nicht aktualisiert werden",
                        e.to_string(),
                    ),
                }
            }
        });
    });

    // Reset user password callback
    let ui_weak_admin4 = ui.as_weak();
    let state_for_reset = state.clone();
    ui.on_admin_reset_user_password(move |user_id| {
        let user_id = user_id.to_string();
        info!("Reset password for user: {}", user_id);

        let state = state_for_reset.clone();
        let ui_weak = ui_weak_admin4.clone();
        let temporary_password = Alphanumeric.sample_string(&mut rand::rng(), 20);

        tokio::spawn(async move {
            let state = state.read().await;
            if let Some(ref server) = state.server {
                match server
                    .reset_user_password(&user_id, &temporary_password)
                    .await
                {
                    Ok(()) => {
                        info!("Password reset for user {} to generated temporary password", user_id);
                        show_success_dialog(
                            ui_weak.clone(),
                            "Password reset",
                            format!(
                                "Temporary password for user {}:\n{}\n\nShare it securely and rotate it after first login.",
                                user_id, temporary_password
                            ),
                        );
                    }
                    Err(e) => {
                        warn!("Failed to reset password: {}", e);
                        show_error_dialog(
                            ui_weak.clone(),
                            "Password reset failed",
                            e.to_string(),
                        );
                    }
                }
            }
        });
    });

    // Toggle user active callback
    let ui_weak_admin5 = ui.as_weak();
    let state_for_toggle = state.clone();
    ui.on_admin_toggle_user_active(move |user_id| {
        let user_id = user_id.to_string();
        info!("Toggle active for user: {}", user_id);

        let state = state_for_toggle.clone();
        let ui_weak = ui_weak_admin5.clone();

        tokio::spawn(async move {
            let current_user = {
                let state = state.read().await;
                state
                    .admin_users_cache
                    .iter()
                    .find(|u| u.id.to_string() == user_id)
                    .cloned()
            };

            let Some(user) = current_user else {
                show_error_dialog(
                    ui_weak.clone(),
                    "Benutzer nicht gefunden",
                    "Der ausgewählte Benutzer ist nicht mehr im lokalen Admin-Cache vorhanden.",
                );
                return;
            };

            let refresh_result = {
                let state = state.read().await;
                if let Some(ref server) = state.server {
                    let new_active = !user.is_active;
                    let updates = serde_json::json!({ "is_active": new_active });
                    match server.update_user(&user_id, updates).await {
                        Ok(_) => {
                            info!("User {} active toggled to {}", user_id, new_active);
                            Some(server.list_users().await)
                        }
                        Err(e) => {
                            warn!("Failed to toggle user active: {}", e);
                            show_error_dialog(
                                ui_weak.clone(),
                                "Statuswechsel fehlgeschlagen",
                                e.to_string(),
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            };

            if let Some(result) = refresh_result {
                match result {
                    Ok(users) => {
                        {
                            let mut state = state.write().await;
                            state.admin_users_cache.clone_from(&users);
                        }
                        if let Some(ui) = ui_weak.upgrade() {
                            render_admin_users(&ui, &users);
                        }
                    }
                    Err(e) => show_error_dialog(
                        ui_weak,
                        "Benutzerliste konnte nicht aktualisiert werden",
                        e.to_string(),
                    ),
                }
            }
        });
    });

    // Add user callback
    let ui_weak_admin6 = ui.as_weak();
    ui.on_admin_add_user(move || {
        info!("Add new user");
        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak_admin6.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_admin_user_edit_mode(false);
                    ui.set_admin_user_form_id(SharedString::from(""));
                    ui.set_admin_user_form_title(SharedString::from("Benutzer hinzufügen"));
                    ui.set_admin_user_form_username(SharedString::from(""));
                    ui.set_admin_user_form_name(SharedString::from(""));
                    ui.set_admin_user_form_email(SharedString::from(""));
                    ui.set_admin_user_form_role(SharedString::from("user"));
                    ui.set_show_admin_user_dialog(true);
                }
            }
        });
    });

    // Admin user form cancel callback
    let ui_weak_admin8 = ui.as_weak();
    ui.on_admin_cancel_user_form(move || {
        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak_admin8.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_show_admin_user_dialog(false);
                }
            }
        });
    });

    // Admin user form submit callback
    let ui_weak_admin9 = ui.as_weak();
    let state_for_submit = state.clone();
    ui.on_admin_submit_user_form(move || {
        let Some(ui) = ui_weak_admin9.upgrade() else {
            return;
        };

        let is_edit = ui.get_admin_user_edit_mode();
        let user_id = ui.get_admin_user_form_id().to_string();
        let username = ui.get_admin_user_form_username().trim().to_string();
        let name = ui.get_admin_user_form_name().trim().to_string();
        let email = ui.get_admin_user_form_email().trim().to_string();
        let role_input = ui.get_admin_user_form_role().trim().to_string();

        if name.is_empty() || email.is_empty() || (!is_edit && username.is_empty()) {
            show_error_dialog(
                ui_weak_admin9.clone(),
                "Pflichtfelder fehlen",
                "Bitte Benutzername, Name und E-Mail ausfüllen.",
            );
            return;
        }

        let role = match normalize_admin_role(&role_input) {
            Ok(role) => role.to_string(),
            Err(e) => {
                show_error_dialog(ui_weak_admin9.clone(), "Ungültige Rolle", e.to_string());
                return;
            }
        };

        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak_admin9.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_show_admin_user_dialog(false);
                }
            }
        });

        let state = state_for_submit.clone();
        let ui_weak = ui_weak_admin9.clone();
        tokio::spawn(async move {
            let temporary_password = Alphanumeric.sample_string(&mut rand::rng(), 20);
            let users_result = {
                let state = state.read().await;
                if let Some(ref server) = state.server {
                    let result = if is_edit {
                        let updates = serde_json::json!({
                            "name": name,
                            "email": email,
                            "role": role,
                        });
                        server.update_user(&user_id, updates).await
                    } else {
                        server
                            .create_user(&username, &email, &name, &role, &temporary_password)
                            .await
                    };

                    match result {
                        Ok(()) => Some(server.list_users().await),
                        Err(e) => {
                            show_error_dialog(
                                ui_weak.clone(),
                                if is_edit {
                                    "Benutzer konnte nicht gespeichert werden"
                                } else {
                                    "Benutzer konnte nicht angelegt werden"
                                },
                                e.to_string(),
                            );
                            None
                        }
                    }
                } else {
                    show_error_dialog(
                        ui_weak.clone(),
                        "Keine Verbindung",
                        "Es ist aktuell kein Server verbunden.",
                    );
                    None
                }
            };

            if let Some(result) = users_result {
                match result {
                    Ok(users) => {
                        {
                            let mut state = state.write().await;
                            state.admin_users_cache.clone_from(&users);
                        }
                        if let Some(ui) = ui_weak.upgrade() {
                            render_admin_users(&ui, &users);
                        }

                        if is_edit {
                            show_success_dialog(
                                ui_weak,
                                "Benutzer gespeichert",
                                format!("Die Änderungen für {} wurden übernommen.", name),
                            );
                        } else {
                            show_success_dialog(
                                ui_weak,
                                "Benutzer angelegt",
                                format!(
                                    "Benutzer {} wurde angelegt.\n\nTemporäres Passwort:\n{}\n\nBitte sicher übermitteln und beim ersten Login rotieren.",
                                    username, temporary_password
                                ),
                            );
                        }
                    }
                    Err(e) => {
                        show_error_dialog(
                            ui_weak,
                            "Benutzerliste konnte nicht aktualisiert werden",
                            e.to_string(),
                        );
                    }
                }
            }
        });
    });

    // Search users callback
    let ui_weak_admin7 = ui.as_weak();
    let state_for_search = state.clone();
    ui.on_admin_search_users(move |query| {
        let query = query.to_lowercase();
        info!("Search users: {}", query);
        let state = state_for_search.clone();
        let ui_weak = ui_weak_admin7.clone();

        tokio::spawn(async move {
            let state = state.read().await;
            let users = &state.admin_users_cache;
            let filtered: Vec<AdminUserInfo> = users
                .iter()
                .filter(|u| {
                    query.is_empty()
                        || u.username.to_lowercase().contains(&query)
                        || u.email.to_lowercase().contains(&query)
                        || u.name.to_lowercase().contains(&query)
                })
                .map(|u| AdminUserInfo {
                    id: SharedString::from(u.id.to_string()),
                    username: SharedString::from(&u.username),
                    email: SharedString::from(&u.email),
                    name: SharedString::from(&u.name),
                    initial: SharedString::from(
                        u.name
                            .chars()
                            .next()
                            .or_else(|| u.username.chars().next())
                            .map_or_else(|| "?".to_string(), |c| c.to_uppercase().to_string()),
                    ),
                    role: SharedString::from(format!("{:?}", u.role)),
                    is_active: u.is_active,
                    last_login: SharedString::from(u.last_login.map_or_else(
                        || "-".to_string(),
                        |dt| dt.format("%d.%m.%Y %H:%M").to_string(),
                    )),
                    created_at: SharedString::from(u.created_at.format("%d.%m.%Y").to_string()),
                })
                .collect();

            if let Some(ui) = ui_weak.upgrade() {
                ui.set_admin_users(ModelRc::new(VecModel::from(filtered)));
            }
        });
    });

    // =========================================================================
    // Admin Server Config Callbacks
    // =========================================================================

    // Load server config callback
    let ui_weak_config1 = ui.as_weak();
    let state_for_config = state.clone();
    ui.on_admin_load_server_config(move || {
        info!("Loading server configuration");
        let state = state_for_config.clone();
        let ui_weak = ui_weak_config1.clone();

        tokio::spawn(async move {
            let state = state.read().await;
            if let Some(ref server) = state.server {
                match server.get_server_config().await {
                    Ok(config) => {
                        if let Some(ui) = ui_weak.upgrade() {
                            let config_data = ServerConfigData {
                                server_name: SharedString::from(
                                    config["server_name"].as_str().unwrap_or(""),
                                ),
                                port: i32::try_from(config["port"].as_i64().unwrap_or(8443))
                                    .unwrap_or(8443),
                                enable_tls: config["enable_tls"].as_bool().unwrap_or(true),
                                enable_mdns: config["enable_mdns"].as_bool().unwrap_or(true),
                                encryption_enabled: config["encryption_enabled"]
                                    .as_bool()
                                    .unwrap_or(true),
                                session_timeout_minutes: i32::try_from(
                                    config["session_timeout_minutes"].as_i64().unwrap_or(60),
                                )
                                .unwrap_or(60),
                                allow_self_registration: config["allow_self_registration"]
                                    .as_bool()
                                    .unwrap_or(true),
                                max_concurrent_sessions: i32::try_from(
                                    config["max_concurrent_sessions"].as_i64().unwrap_or(5),
                                )
                                .unwrap_or(5),
                                auto_backup_enabled: config["auto_backup_enabled"]
                                    .as_bool()
                                    .unwrap_or(true),
                                backup_retention_count: i32::try_from(
                                    config["backup_retention_count"].as_i64().unwrap_or(7),
                                )
                                .unwrap_or(7),
                                audit_logging_enabled: config["audit_logging_enabled"]
                                    .as_bool()
                                    .unwrap_or(true),
                                license_plate_display: i32::try_from(
                                    config["license_plate_display"].as_i64().unwrap_or(0),
                                )
                                .unwrap_or(0),
                                organization_name: SharedString::from(
                                    config["organization_name"].as_str().unwrap_or(""),
                                ),
                            };
                            ui.set_admin_server_config(config_data);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load server config: {}", e);
                    }
                }
            }
        });
    });

    // Save server config callback
    let ui_weak_config2 = ui.as_weak();
    let state_for_save = state;
    ui.on_admin_save_server_config(move |config| {
        info!("Saving server configuration");
        let state = state_for_save.clone();
        let _ui_weak = ui_weak_config2.clone();

        let updates = serde_json::json!({
            "server_name": config.server_name.to_string(),
            "port": config.port,
            "enable_tls": config.enable_tls,
            "enable_mdns": config.enable_mdns,
            "encryption_enabled": config.encryption_enabled,
            "session_timeout_minutes": config.session_timeout_minutes,
            "allow_self_registration": config.allow_self_registration,
            "max_concurrent_sessions": config.max_concurrent_sessions,
            "auto_backup_enabled": config.auto_backup_enabled,
            "backup_retention_count": config.backup_retention_count,
            "audit_logging_enabled": config.audit_logging_enabled,
            "license_plate_display": config.license_plate_display,
            "organization_name": config.organization_name.to_string(),
        });

        tokio::spawn(async move {
            let state = state.read().await;
            if let Some(ref server) = state.server {
                match server.update_server_config(updates).await {
                    Ok(()) => {
                        info!("Server config saved successfully");
                    }
                    Err(e) => {
                        warn!("Failed to save server config: {}", e);
                    }
                }
            }
        });
    });

    // Load accessibility settings from local config
    let config_dir = directories::ProjectDirs::from("com", "parkhub", "ParkHub Client")
        .map_or_else(
            || std::path::PathBuf::from(".").join("config"),
            |p| p.config_dir().to_path_buf(),
        );
    let config_path = config_dir.join("accessibility.toml");

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(settings) = toml::from_str::<AccessibilitySettings>(&content) {
                info!("Loaded accessibility settings from {:?}", config_path);
                ui.global::<ThemeSettings>().set_mode(settings.theme_mode);
                ui.global::<ThemeSettings>()
                    .set_font_scale(settings.font_scale);
                ui.global::<ThemeSettings>()
                    .set_reduce_motion(settings.reduce_motion);
            }
        }
    }

    // Save accessibility settings when changed
    let ui_weak_a11y = ui.as_weak();
    ui.on_setting_changed(move |key, value| {
        let key = key.to_string();
        let value = value.to_string();

        if let Some(ui) = ui_weak_a11y.upgrade() {
            // Only handle accessibility-related settings
            if key == "theme_mode" || key == "font_scale" || key == "reduce_motion" {
                let settings = AccessibilitySettings {
                    theme_mode: ui.global::<ThemeSettings>().get_mode(),
                    font_scale: ui.global::<ThemeSettings>().get_font_scale(),
                    reduce_motion: ui.global::<ThemeSettings>().get_reduce_motion(),
                };

                // Save to file
                let config_dir = directories::ProjectDirs::from("com", "parkhub", "ParkHub Client")
                    .map_or_else(
                        || std::path::PathBuf::from(".").join("config"),
                        |p| p.config_dir().to_path_buf(),
                    );

                if let Err(e) = std::fs::create_dir_all(&config_dir) {
                    warn!("Failed to create config dir: {}", e);
                    return;
                }

                let config_path = config_dir.join("accessibility.toml");
                if let Ok(content) = toml::to_string_pretty(&settings) {
                    if let Err(e) = std::fs::write(&config_path, content) {
                        warn!("Failed to save accessibility settings: {}", e);
                    } else {
                        info!("Saved accessibility settings: {} = {}", key, value);
                    }
                }
            }
        }
    });

    // Run UI event loop
    ui.run().context("UI event loop error")?;

    Ok(())
}

/// Load parking data from server
async fn load_parking_data(state: Arc<RwLock<AppState>>, ui_weak: slint::Weak<MainWindow>) {
    let state = state.read().await;
    if let Some(ref server) = state.server {
        // Load parking lots
        match server.list_lots().await {
            Ok(lots) => {
                if let Some(lot) = lots.first() {
                    let lot_name = lot.name.clone();
                    let total_slots = lot.total_slots;
                    let available_slots = lot.available_slots;
                    let ui_weak_lot = ui_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_lot.upgrade() {
                            ui.set_lot_name(SharedString::from(&lot_name));
                            ui.set_total_slots(total_slots);
                            ui.set_available_slots(available_slots);
                        }
                    });

                    // Load slots for the first lot
                    match server.get_lot_slots(&lot.id.to_string()).await {
                        Ok(mut slots) => {
                            // Sort slots by slot_number to ensure proper display order
                            slots.sort_by_key(|s| s.slot_number);
                            info!("Loaded {} slots from server", slots.len());
                            let slot_data: Vec<ParkingSlotData> = slots
                                .iter()
                                .map(|s| {
                                    let (license_plate, end_time, booked_by) = s
                                        .current_booking
                                        .as_ref()
                                        .map(|b| {
                                            (
                                                b.license_plate.clone(),
                                                b.end_time.format("%H:%M").to_string(),
                                                if b.is_own_booking {
                                                    "You".to_string()
                                                } else {
                                                    "Other".to_string()
                                                },
                                            )
                                        })
                                        .unwrap_or_default();

                                    info!(
                                        "Slot {}: row={}, col={}, status={:?}",
                                        s.slot_number, s.row, s.column, s.status
                                    );
                                    ParkingSlotData {
                                        id: SharedString::from(s.id.to_string()),
                                        slot_number: s.slot_number,
                                        row: s.row,
                                        col: s.column,
                                        status: match s.status {
                                            parkhub_common::SlotStatus::Available => {
                                                SlotStatus::Available
                                            }
                                            parkhub_common::SlotStatus::Occupied
                                            | parkhub_common::SlotStatus::Reserved => {
                                                SlotStatus::Occupied
                                            }
                                            parkhub_common::SlotStatus::Maintenance
                                            | parkhub_common::SlotStatus::Disabled => {
                                                SlotStatus::Disabled
                                            }
                                        },
                                        license_plate: SharedString::from(license_plate),
                                        end_time: SharedString::from(end_time),
                                        booked_by: SharedString::from(booked_by),
                                    }
                                })
                                .collect();
                            let ui_weak_slots = ui_weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak_slots.upgrade() {
                                    info!("Setting {} slots in UI", slot_data.len());
                                    ui.set_slots(ModelRc::new(VecModel::from(slot_data)));
                                }
                            });
                        }
                        Err(e) => {
                            warn!("Failed to load slots: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to load lots: {}", e);
            }
        }

        // Load user's bookings
        match server.list_bookings().await {
            Ok(bookings) => {
                let booking_data: Vec<BookingData> = bookings
                    .iter()
                    .map(|b| BookingData {
                        id: SharedString::from(b.id.to_string()),
                        slot_number: b.slot_number,
                        start_time: SharedString::from(b.start_time.format("%H:%M").to_string()),
                        end_time: SharedString::from(b.end_time.format("%H:%M").to_string()),
                        license_plate: SharedString::from(&b.vehicle.license_plate),
                        status: SharedString::from(format!("{:?}", b.status)),
                    })
                    .collect();
                let ui_weak_bookings = ui_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_bookings.upgrade() {
                        ui.set_my_bookings(ModelRc::new(VecModel::from(booking_data)));
                    }
                });
            }
            Err(e) => {
                warn!("Failed to load bookings: {}", e);
            }
        }
    }
}
