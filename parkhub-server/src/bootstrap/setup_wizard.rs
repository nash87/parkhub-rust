//! First-run GUI setup wizard and passphrase dialog.
//!
//! Only compiled when the `gui` feature is on. The Slint types used
//! below (`SetupWizard`, `PassphraseDialog`) are generated at the
//! crate root by `slint::include_modules!()` and reachable here via
//! `crate::…`.

#![cfg(feature = "gui")]

use anyhow::{Context, Result};
use slint::ComponentHandle;

use crate::config::ServerConfig;
use crate::{PassphraseDialog, SetupWizard};

use super::paths::{get_local_ip, hash_password};

pub(crate) fn run_setup_wizard() -> Result<ServerConfig> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Get local IP for display
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Create the setup wizard window
    let ui = SetupWizard::new().context("Failed to create setup wizard")?;

    // Set initial values
    ui.set_local_ip(local_ip.into());
    ui.set_port(i32::from(parkhub_common::DEFAULT_PORT));

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
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
                eprintln!("Failed to hash password: {e}");
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let username_style = ui.get_username_style() as u8;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
            session_timeout_minutes: 60, // 1 hour default
            allow_self_registration: false,
            require_email_verification: false,
            max_concurrent_sessions: 0, // Unlimited
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
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    // Handle cancel
    ui.on_cancel_setup(|| {
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    // Run the UI
    ui.run().context("Setup wizard failed")?;

    // Return the result
    let config = result.borrow().clone();
    config.ok_or_else(|| anyhow::anyhow!("Setup was cancelled"))
}

pub(crate) fn prompt_passphrase_gui() -> Result<String> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Create a simple passphrase dialog
    let ui = PassphraseDialog::new().context("Failed to create passphrase dialog")?;

    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let result_clone = result.clone();

    ui.on_submit(move |passphrase| {
        *result_clone.borrow_mut() = Some(passphrase.to_string());
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    ui.on_cancel(|| {
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    ui.run().context("Passphrase dialog failed")?;

    let passphrase = result.borrow().clone();
    passphrase.ok_or_else(|| anyhow::anyhow!("Passphrase entry was cancelled"))
}
