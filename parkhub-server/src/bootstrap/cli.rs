//! Command-line argument parsing for the `parkhub-server` binary.
//!
//! Exposes [`CliArgs`] with its hand-rolled `parse()` / `print_help()` /
//! `print_version()` methods. Intentionally dependency-free — the binary
//! refuses to pull in `clap` for four boolean flags and two options.

use std::path::PathBuf;

/// CLI arguments for the server
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
#[derive(Debug, Clone)]
pub(crate) struct CliArgs {
    /// Show help message
    pub(crate) help: bool,
    /// Run in debug mode with verbose logging
    pub(crate) debug: bool,
    /// Run without GUI (headless mode)
    pub(crate) headless: bool,
    /// Run in unattended mode (auto-configure with defaults)
    pub(crate) unattended: bool,
    /// Custom port to listen on
    pub(crate) port: Option<u16>,
    /// Custom data directory
    pub(crate) data_dir: Option<PathBuf>,
    /// Show version
    pub(crate) version: bool,
    /// Perform a health check against the running server and exit 0/1.
    /// Used as the Docker HEALTHCHECK command (works in distroless images).
    pub(crate) health_check: bool,
}

impl CliArgs {
    pub(crate) fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut cli = Self {
            help: false,
            debug: false,
            headless: false,
            unattended: false,
            port: None,
            data_dir: None,
            version: false,
            health_check: false,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--help" => cli.help = true,
                "-v" | "--version" => cli.version = true,
                "-d" | "--debug" => cli.debug = true,
                "--headless" => cli.headless = true,
                "--unattended" => cli.unattended = true,
                "--health-check" => cli.health_check = true,
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

    pub(crate) fn print_help() {
        println!("ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("USAGE:");
        println!("    parkhub-server [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help         Show this help message");
        println!("    -v, --version      Show version information");
        println!("    -d, --debug        Enable debug logging");
        println!("    --headless         Run without GUI (console only)");
        println!("    --unattended       Auto-configure with defaults (no setup wizard)");
        println!("    -p, --port PORT    Set the server port (default: 7878)");
        println!("    --data-dir PATH    Set custom data directory");
        println!("    --health-check     Check if a running server is healthy (exits 0/1)");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    PARKHUB_DB_PASSPHRASE    Database encryption passphrase");
        println!("    PORT                     Server port (overridden by --port flag)");
        println!("    SEED_DEMO_DATA           Seed demo lots/users on first start (true/1)");
        println!("    DEMO_MODE                Enable demo UI and seed data on first start");
        println!("    RUST_LOG                 Logging filter (e.g., debug,info)");
        println!();
        println!("EXAMPLES:");
        println!("    parkhub-server                    # Start with GUI");
        println!("    parkhub-server --headless         # Start in console mode");
        println!("    parkhub-server --debug            # Start with debug logging");
        println!("    parkhub-server --unattended       # Auto-configure and start");
        println!("    parkhub-server -p 8080            # Use port 8080");
        println!("    parkhub-server --health-check     # Docker HEALTHCHECK probe");
    }

    pub(crate) fn print_version() {
        println!("ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
        println!("Protocol Version: {}", parkhub_common::PROTOCOL_VERSION);
        #[cfg(feature = "gui")]
        println!("GUI: enabled");
        #[cfg(not(feature = "gui"))]
        println!("GUI: disabled");
    }
}
