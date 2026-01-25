# ParkHub

Open source parking lot management system with client-server architecture.

## Features

- **Server Application** - Database server with HTTP API and LAN autodiscovery
  - Setup wizard for easy configuration
  - TLS encryption with self-signed certificates
  - mDNS/DNS-SD for automatic network discovery
  - Embedded database (redb) - no external database required
  - Headless and GUI modes

- **Client Application** - Desktop application for parking management
  - Automatic server discovery on local network
  - Manual server connection option
  - Modern Slint UI

## Installation

### Portable Mode

Both server and client support portable mode - just extract and run:

1. Download the release for your platform
2. Extract to any folder
3. Create a `parkhub-data` folder next to the executable (for server)
4. Run the application

Data will be stored in the `parkhub-data` folder, making it easy to move or backup.

### Standard Installation

If no `parkhub-data` folder exists, the application uses system directories:
- Windows: `%APPDATA%\parkhub\ParkHub Server`
- Linux: `~/.local/share/ParkHub Server`
- macOS: `~/Library/Application Support/com.parkhub.ParkHub-Server`

## Building from Source

### Prerequisites

- Rust 1.75 or later
- For GUI builds: CMake and system dependencies for Slint

### Build Commands

```bash
# Build everything
cargo build --release --workspace

# Build server only (headless mode)
cargo build --release --package parkhub-server --no-default-features --features headless

# Build server with GUI
cargo build --release --package parkhub-server --features gui

# Build client
cargo build --release --package parkhub-client
```

## Configuration

### Server Configuration

On first run, the server will:
1. Show setup wizard (GUI mode) or use defaults (headless mode)
2. Create configuration in `config.toml`
3. Generate TLS certificates

Configuration options in `config.toml`:
```toml
server_name = "ParkHub Server"
port = 7878
enable_tls = true
enable_mdns = true
admin_username = "admin"
admin_password_hash = "..."
```

### Client Configuration

The client automatically discovers servers on the local network via mDNS.
For remote servers, use the manual connection option with the server's IP address.

## Architecture

```
parkhub/
  parkhub-common/     # Shared types and protocol definitions
  parkhub-server/     # Server application
  parkhub-client/     # Client application
```

## API

The server provides a REST API at `http(s)://host:port/api/v1/`:

- `POST /handshake` - Protocol handshake
- `POST /api/v1/auth/login` - User authentication
- `GET /api/v1/users/me` - Current user info
- `GET /api/v1/lots` - List parking lots
- `GET /api/v1/lots/{id}/slots` - List slots in a lot
- `GET /api/v1/bookings` - List bookings
- `POST /api/v1/bookings` - Create booking
- `DELETE /api/v1/bookings/{id}` - Cancel booking

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
