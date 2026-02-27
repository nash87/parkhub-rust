# Configuration Reference

All configuration options for ParkHub Rust.

## Configuration File

ParkHub reads its configuration from `config.toml` in the data directory.

**Data directory locations:**

| Mode | Path |
|---|---|
| Portable (default) | `./data/` next to the binary |
| Docker | `/data/` (volume mount) |
| Custom | `--data-dir /path/to/dir` CLI flag |
| Windows system | `%APPDATA%\parkhub\ParkHub Server\` |
| Linux system | `~/.local/share/ParkHub Server/` |
| macOS system | `~/Library/Application Support/com.parkhub.ParkHub-Server/` |

On first run with no existing `config.toml`, the server runs the setup wizard (GUI mode)
or auto-configures with secure defaults (headless/unattended mode).

---

## All Configuration Fields

### Core Server Settings

| Field | Type | Default | Description |
|---|---|---|---|
| `server_name` | string | `"ParkHub Server"` | Display name shown in the UI and mDNS advertisements |
| `port` | integer | `7878` | TCP port the HTTP server binds to |
| `enable_tls` | bool | `true` | Enable TLS 1.3. Auto-generates a self-signed cert if none exists |
| `enable_mdns` | bool | `true` | Broadcast server presence via mDNS for LAN autodiscovery |
| `portable_mode` | bool | `true` | Store data next to the binary instead of system directories |

### Authentication

| Field | Type | Default | Description |
|---|---|---|---|
| `admin_username` | string | `"admin"` | Username for the initial admin account |
| `admin_password_hash` | string | — | Argon2id hash of the admin password. Set by setup wizard or auto-gen |
| `allow_self_registration` | bool | `false` | Allow users to register without an invite. Off by default |
| `require_email_verification` | bool | `false` | Require email verification for new accounts (requires SMTP config) |
| `session_timeout_minutes` | integer | `60` | Session expiry in minutes. `0` means sessions never expire |
| `max_concurrent_sessions` | integer | `0` | Max active sessions per user. `0` means unlimited |

### Database Encryption

| Field | Type | Default | Description |
|---|---|---|---|
| `encryption_enabled` | bool | `true` | Enable AES-256-GCM at-rest encryption for the redb database |
| `encryption_passphrase` | string | — | The passphrase. **Not saved to disk** (`#[serde(skip)]`). Supply via `PARKHUB_DB_PASSPHRASE` env var or GUI prompt |

### Privacy and Display

| Field | Type | Default | Description |
|---|---|---|---|
| `license_plate_display` | integer | `0` | How to display license plates: `0`=show, `1`=blur, `2`=redact (show `***`), `3`=hide |
| `organization_name` | string | `""` | Organization name used for branding in the UI |
| `default_language` | string | `"en"` | Default UI language (`en`, `de`, etc.) |

### Backup

| Field | Type | Default | Description |
|---|---|---|---|
| `auto_backup_enabled` | bool | `true` | Enable automatic daily database backups |
| `backup_retention_count` | integer | `7` | Number of backup files to keep before rotating |

### Audit

| Field | Type | Default | Description |
|---|---|---|---|
| `audit_logging_enabled` | bool | `true` | Log security-relevant events (login, booking creation, account deletion) |

### UI / Accessibility

| Field | Type | Default | Description |
|---|---|---|---|
| `theme_mode` | integer | `0` | UI theme: `0`=Dark, `1`=Light, `2`=High Contrast, `3`=Deuteranopia, `4`=Protanopia, `5`=Tritanopia |
| `font_scale` | float | `1.0` | Font size multiplier: `1.0`=Normal, `1.25`=Large, `1.5`=Extra Large |
| `reduce_motion` | bool | `false` | Reduce animation motion for accessibility |
| `close_behavior` | string | `"ask"` | Windows tray behavior on window close: `"ask"`, `"minimize"`, `"exit"` |

### Developer / Setup (not saved to disk)

These fields use `#[serde(skip)]` and are only set programmatically during setup:

| Field | Type | Description |
|---|---|---|
| `generate_dummy_users` | bool | Create sample users on first run (unattended mode only) |
| `username_style` | integer | Style for generated usernames: `0`=FirstLastLetter, `1`=FirstDotLast, `2`=InitialLast, `3`=FirstInitial |

---

## Example config.toml

```toml
server_name = "Mein Parkplatz"
port = 8080
enable_tls = false
enable_mdns = true
encryption_enabled = true
admin_username = "admin"
admin_password_hash = "$argon2id$v=19$m=19456,t=2,p=1$..."
portable_mode = true
license_plate_display = 0
session_timeout_minutes = 60
allow_self_registration = false
require_email_verification = false
max_concurrent_sessions = 0
auto_backup_enabled = true
backup_retention_count = 7
audit_logging_enabled = true
default_language = "de"
organization_name = "Meine GmbH"
close_behavior = "minimize"
theme_mode = 0
font_scale = 1.0
reduce_motion = false
```

---

## Environment Variables

ParkHub respects these environment variables, which override `config.toml` where applicable:

| Variable | Purpose |
|---|---|
| `PARKHUB_DB_PASSPHRASE` | AES-256-GCM database encryption passphrase. Required when `encryption_enabled = true` and no GUI is available to prompt for it |
| `RUST_LOG` | Tracing filter, e.g. `info`, `debug`, `parkhub_server=trace` |

---

## CLI Flags

| Flag | Description |
|---|---|
| `-h`, `--help` | Show help and exit |
| `-v`, `--version` | Show version and protocol version, then exit |
| `-d`, `--debug` | Enable verbose debug logging |
| `--headless` | Run without GUI (console mode) |
| `--unattended` | Auto-configure with defaults: admin/admin, encryption off, TLS off. Suitable for CI and containers |
| `-p PORT`, `--port PORT` | Override the port from config |
| `--data-dir PATH` | Override the data directory |

---

## Encryption Passphrase Handling

When `encryption_enabled = true`, ParkHub needs the passphrase at startup to open the database.

**Priority order:**
1. `PARKHUB_DB_PASSPHRASE` environment variable (recommended for servers and Docker)
2. GUI passphrase prompt (GUI builds only)
3. Fatal error in headless mode if neither is provided

The passphrase is **never written to disk** — it is held only in memory for the lifetime of
the process and used to derive a key via PBKDF2-SHA256 for AES-256-GCM.

```bash
# Docker — pass via environment
docker run -e PARKHUB_DB_PASSPHRASE="my-strong-passphrase" parkhub

# Docker Compose — use a secret or .env file (never commit the passphrase to git)
docker compose --env-file .env up -d
```

```bash
# Bare metal — export before running
export PARKHUB_DB_PASSPHRASE="my-strong-passphrase"
./parkhub-server --headless
```

> If you lose the passphrase, the database cannot be decrypted. Store it in a password
> manager or secret vault (e.g., HashiCorp Vault).
