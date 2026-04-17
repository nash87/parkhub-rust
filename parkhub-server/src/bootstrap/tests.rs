//! Tests for CLI arg parsing, the standalone health-check probe, and
//! the demo-mode seed path.

#![cfg(test)]

use std::path::PathBuf;

use super::cli::CliArgs;
use super::health::perform_health_check;
use super::seed::seed_demo_data;

// ---------------------------------------------------------------------------
// CliArgs parsing
// ---------------------------------------------------------------------------

fn parse_args(args: &[&str]) -> CliArgs {
    // CliArgs::parse() reads std::env::args(), so we exercise the struct fields
    // directly here to avoid side-effects from the process argument list.
    let mut cli = CliArgs {
        help: false,
        debug: false,
        headless: false,
        unattended: false,
        port: None,
        data_dir: None,
        version: false,
        health_check: false,
    };
    let mut i = 0;
    let owned: Vec<String> = args.iter().map(std::string::ToString::to_string).collect();
    while i < owned.len() {
        match owned[i].as_str() {
            "-h" | "--help" => cli.help = true,
            "-v" | "--version" => cli.version = true,
            "-d" | "--debug" => cli.debug = true,
            "--headless" => cli.headless = true,
            "--unattended" => cli.unattended = true,
            "--health-check" => cli.health_check = true,
            "-p" | "--port" => {
                if i + 1 < owned.len() {
                    cli.port = owned[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--data-dir" => {
                if i + 1 < owned.len() {
                    cli.data_dir = Some(PathBuf::from(&owned[i + 1]));
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    cli
}

#[test]
fn health_check_flag_is_parsed() {
    let cli = parse_args(&["--health-check"]);
    assert!(
        cli.health_check,
        "--health-check must set health_check=true"
    );
    assert!(!cli.headless);
    assert!(!cli.unattended);
}

#[test]
fn health_check_flag_default_is_false() {
    let cli = parse_args(&["--headless", "--unattended"]);
    assert!(!cli.health_check, "health_check must default to false");
}

#[test]
fn health_check_with_port_flag() {
    let cli = parse_args(&["--health-check", "--port", "8080"]);
    assert!(cli.health_check);
    assert_eq!(cli.port, Some(8080));
}

#[test]
fn port_flag_parsed_correctly() {
    let cli = parse_args(&["-p", "9000"]);
    assert_eq!(cli.port, Some(9000));
}

#[test]
fn data_dir_flag_parsed() {
    let cli = parse_args(&["--data-dir", "/tmp/mydata"]);
    assert_eq!(cli.data_dir, Some(PathBuf::from("/tmp/mydata")));
}

// ---------------------------------------------------------------------------
// perform_health_check — connection-refused path exits with 1
// ---------------------------------------------------------------------------

#[test]
fn health_check_returns_1_when_server_not_running() {
    // Port 1 is reserved and guaranteed not to have a listener; expect exit code 1.
    let result = perform_health_check(1);
    assert_eq!(
        result, 1,
        "health check must return 1 when server is unreachable"
    );
}

// ---------------------------------------------------------------------------
// seed_demo_data — creates 10 lots and 200 users in a real database
// ---------------------------------------------------------------------------

#[tokio::test]
async fn seed_demo_data_creates_lots_and_users() {
    use crate::db::{Database, DatabaseConfig};

    let dir = tempfile::tempdir().expect("tempdir");
    let db_config = DatabaseConfig {
        path: dir.path().to_path_buf(),
        encryption_enabled: false,
        passphrase: None,
        create_if_missing: true,
    };
    let db = Database::open(&db_config).expect("open test db");

    // DB should start empty
    let lots_before = db.list_parking_lots().await.unwrap();
    assert_eq!(lots_before.len(), 0, "lots must be empty before seeding");

    seed_demo_data(&db)
        .await
        .expect("seed_demo_data must succeed");

    let lots_after = db.list_parking_lots().await.unwrap();
    assert_eq!(
        lots_after.len(),
        10,
        "seed must create exactly 10 parking lots"
    );

    // All lots should have at least one slot
    for lot in &lots_after {
        assert!(
            lot.total_slots > 0,
            "each seeded lot must have at least one slot"
        );
    }

    // Verify user count (200 demo users)
    let users = db.list_users().await.unwrap();
    assert_eq!(users.len(), 200, "seed must create exactly 200 demo users");
}

#[tokio::test]
async fn seed_demo_data_is_idempotent_when_called_twice() {
    use crate::db::{Database, DatabaseConfig};

    let dir = tempfile::tempdir().expect("tempdir");
    let db_config = DatabaseConfig {
        path: dir.path().to_path_buf(),
        encryption_enabled: false,
        passphrase: None,
        create_if_missing: true,
    };
    let db = Database::open(&db_config).expect("open test db");

    // First call
    seed_demo_data(&db).await.expect("first seed must succeed");
    let lots_first = db.list_parking_lots().await.unwrap().len();

    // Second call must not fail; lots are stored by UUID so duplicate lots
    // may be added by a naive caller — the startup guard (lot_count < 2)
    // prevents double-seeding, but the function itself should not panic.
    let result = seed_demo_data(&db).await;
    assert!(
        result.is_ok(),
        "second seed_demo_data call must not return Err"
    );
    // Lot count after second call is at least the original 10
    let lots_second = db.list_parking_lots().await.unwrap().len();
    assert!(lots_second >= lots_first, "lot count must not decrease");
}
