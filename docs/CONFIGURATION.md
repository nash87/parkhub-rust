# Configuration Reference — ParkHub Rust

ParkHub is configured through a `config.toml` file and environment variables.
Environment variables take precedence over the config file where applicable.

---

## Configuration File Location

| Mode | Path |
|------|------|
| Portable (default) | `./data/config.toml` next to the binary |
| Docker | `/data/config.toml` (inside the named volume) |
| Custom `--data-dir` | `<data-dir>/config.toml` |
| Windows system | `%APPDATA%\parkhub\ParkHub Server\config.toml` |
| Linux system | `~/.local/share/ParkHub Server/config.toml` |
| macOS system | `~/Library/Application Support/com.parkhub.ParkHub-Server/config.toml` |

On first run with no `config.toml`, ParkHub either runs the GUI setup wizard (GUI builds)
or auto-configures with secure defaults (headless / `--unattended` mode).

---

## Environment Variables

These variables override or supplement `config.toml`. Use them in Docker and
systemd environments where editing a file inside a volume is inconvenient.

| Variable | Default | Required | Description |
|----------|---------|----------|-------------|
| `PARKHUB_DB_PASSPHRASE` | — | When encryption enabled | AES-256-GCM database encryption passphrase. Never written to disk. Supply via environment variable or GUI prompt. |
| `PARKHUB_HOST` | `0.0.0.0` | No | Bind address for the HTTP server |
| `PARKHUB_PORT` | `8080` | No | Listen port (overrides `port` in config.toml) |
| `PARKHUB_TLS_ENABLED` | `false` | No | Enable TLS. Set `true` to use TLS on the port |
| `PARKHUB_TLS_CERT` | `data/cert.pem` | No | Path to TLS certificate file |
| `PARKHUB_TLS_KEY` | `data/key.pem` | No | Path to TLS private key file |
| `RUST_LOG` | `info` | No | Log level and filter. Examples: `info`, `debug`, `warn`, `parkhub_server=trace` |

---

## config.toml — All Fields

### Core Server

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `server_name` | string | `"ParkHub Server"` | Display name shown in the UI header and mDNS advertisements |
| `port` | integer | `7878` | TCP port. Overridden by `PARKHUB_PORT` environment variable |
| `enable_tls` | bool | `true` | Enable TLS 1.3. Auto-generates a self-signed cert via `rcgen` if no cert file exists |
| `enable_mdns` | bool | `true` | Broadcast presence via mDNS for LAN autodiscovery (Bonjour / Zeroconf) |
| `portable_mode` | bool | `true` | Store all data next to the binary instead of system directories |

### Authentication

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `admin_username` | string | `"admin"` | Username for the initial admin account created on first run |
| `admin_password_hash` | string | — | Argon2id hash of the admin password. Set by setup wizard or auto-generated. Never store a plain-text password here |
| `allow_self_registration` | bool | `false` | Allow users to self-register. Disabled by default — new users are created by administrators |
| `require_email_verification` | bool | `false` | Require email verification on registration (requires SMTP — not yet implemented) |
| `session_timeout_minutes` | integer | `60` | Session token expiry in minutes. Set `0` for sessions that never expire |
| `max_concurrent_sessions` | integer | `0` | Maximum simultaneous sessions per user. Set `0` for unlimited |

### Database Encryption

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `encryption_enabled` | bool | `true` | Enable AES-256-GCM at-rest encryption for the redb database |
| `encryption_passphrase` | string | — | The passphrase. Uses `#[serde(skip)]` — never persisted to disk. Supply via `PARKHUB_DB_PASSPHRASE` environment variable or via the GUI passphrase prompt |

**Key derivation**: PBKDF2-SHA256 is used to derive a 256-bit AES key from the passphrase.
The passphrase is held only in memory and zeroed on drop via the `zeroize` crate.

> If you lose the passphrase, the database cannot be decrypted. Store it in a password
> manager (KeePass, 1Password, Bitwarden) or a secret vault (HashiCorp Vault, AWS Secrets Manager).

### Privacy and Display

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `license_plate_display` | integer | `0` | How to show licence plates in the UI: `0`=show full, `1`=blur, `2`=redact (show `***`), `3`=hide entirely |
| `organization_name` | string | `""` | Organization name used in the UI and legal documents |
| `default_language` | string | `"en"` | Default UI language (`en`, `de`) |

### Automatic Backup

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_backup_enabled` | bool | `true` | Enable automatic daily database backups to `data/backups/` |
| `backup_retention_count` | integer | `7` | Number of backup files to keep. Older files are deleted on rotation |

### Audit Logging

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `audit_logging_enabled` | bool | `true` | Write audit log entries for security-relevant events: logins, booking creation/cancellation, account deletion |

### UI / Accessibility

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `theme_mode` | integer | `0` | UI theme: `0`=Dark, `1`=Light, `2`=High Contrast, `3`=Deuteranopia, `4`=Protanopia, `5`=Tritanopia |
| `font_scale` | float | `1.0` | Font size multiplier: `1.0`=Normal, `1.25`=Large, `1.5`=Extra Large |
| `reduce_motion` | bool | `false` | Reduce UI animation (accessibility preference) |
| `close_behavior` | string | `"ask"` | Windows tray: what to do on window close. `"ask"`, `"minimize"` (minimize to tray), `"exit"` |

---

## CLI Flags

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Show help and exit |
| `-v`, `--version` | Print version and protocol version, then exit |
| `-d`, `--debug` | Enable verbose debug logging (equivalent to `RUST_LOG=debug`) |
| `--headless` | Run without GUI — console-only mode for servers |
| `--unattended` | Auto-configure with defaults: admin/admin, encryption off, TLS off. Suitable for CI and Docker |
| `-p PORT`, `--port PORT` | Override the listening port from config |
| `--data-dir PATH` | Override the data directory path |

---

## Example: Minimal Production `config.toml`

```toml
server_name         = "Firmenparkplatz"
port                = 8080
enable_tls          = false        # TLS handled by reverse proxy
enable_mdns         = false        # disable in cloud / Docker
portable_mode       = true

admin_username      = "admin"
# admin_password_hash is set by the first-run wizard or API

allow_self_registration     = false
require_email_verification  = false
session_timeout_minutes     = 60
max_concurrent_sessions     = 0

encryption_enabled  = true
# encryption_passphrase is NOT stored here — supply via PARKHUB_DB_PASSPHRASE

license_plate_display       = 0
organization_name           = "Muster GmbH"
default_language            = "de"

auto_backup_enabled         = true
backup_retention_count      = 14

audit_logging_enabled       = true

theme_mode          = 0
font_scale          = 1.0
reduce_motion       = false
```

---

## Encryption Passphrase Handling

When `encryption_enabled = true`, ParkHub requires the passphrase at startup.

**Priority order:**

1. `PARKHUB_DB_PASSPHRASE` environment variable (recommended for Docker and servers)
2. GUI passphrase prompt (GUI builds only)
3. Fatal startup error if neither is provided in headless mode

```bash
# Docker — pass via environment variable
docker run -e PARKHUB_DB_PASSPHRASE="my-strong-passphrase" ghcr.io/nash87/parkhub:latest

# Docker Compose — use .env file (never commit this file)
docker compose --env-file .env up -d

# Bare metal — export before running
export PARKHUB_DB_PASSPHRASE="my-strong-passphrase"
./parkhub-server --headless
```

---

## Logging

ParkHub uses the `tracing` crate. Control verbosity via `RUST_LOG`:

```bash
# Production — info only
RUST_LOG=info

# Debug a specific module
RUST_LOG=parkhub_server::db=debug

# All debug output
RUST_LOG=debug
```

Log output goes to stdout in JSON-structured format when `RUST_LOG` is set,
plain text when running interactively.
