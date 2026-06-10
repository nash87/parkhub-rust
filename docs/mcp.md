# ParkHub MCP Server

ParkHub exposes a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP)
server so AI agents (Claude Code, Claude Desktop, any MCP client) can query
availability, inspect occupancy, and create bookings on behalf of a user.

## Requirements

- Build with the `mod-mcp` feature enabled:
  ```
  cargo build --features headless,mod-mcp
  ```
- The `mod-mcp` feature is independent of `full`/`headless`; default builds are
  unaffected.

## Starting the server

```
PARKHUB_API_KEY=<your-api-key> parkhub-server --mcp
```

The process speaks MCP over **stdio** (JSON-RPC newline-delimited, compatible
with all MCP clients). It exits when stdin closes.

`PARKHUB_API_KEY` must be a valid ParkHub API key (create one under
**Settings → API keys** in the web UI or via `POST /api/v1/auth/api-keys`).
The server resolves the key to its owner user and all tool calls operate as
that user.

If the key is missing or invalid the server exits with an error before the
MCP handshake begins.

## Wiring into Claude Code

Add to `.mcp.json` (project) or `~/.claude.json` (global):

```json
{
  "mcpServers": {
    "parkhub": {
      "command": "/path/to/parkhub-server",
      "args": ["--mcp"],
      "env": {
        "PARKHUB_API_KEY": "<your-api-key>",
        "PARKHUB_DATA_DIR": "/path/to/parkhub-data"
      }
    }
  }
}
```

Claude Code will launch `parkhub-server --mcp` as a child process and connect
over stdio automatically.

## Wiring into Claude Desktop

In `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "parkhub": {
      "command": "/path/to/parkhub-server",
      "args": ["--mcp"],
      "env": {
        "PARKHUB_API_KEY": "<your-api-key>"
      }
    }
  }
}
```

## Tools

### `check_availability`

Check parking lot availability for a time window.

**Parameters**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `lot_id` | string (UUID) | No | Filter to a single lot. Omit for all lots. |
| `from` | string (RFC 3339) | Yes | Window start, e.g. `"2025-06-15T09:00:00Z"` |
| `to` | string (RFC 3339) | Yes | Window end |

**Returns** JSON array of `LotAvailability`:

```json
[
  {
    "lot_id": "…",
    "lot_name": "Main Lot",
    "free_slots": 4,
    "total_slots": 10
  }
]
```

A slot is counted as free when it has no confirmed/pending booking overlapping
the requested window and its status is not `maintenance` or `disabled`.

---

### `get_occupancy`

Current occupancy snapshot for parking lots (based on lot-level counters).

**Parameters**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `lot_id` | string (UUID) | No | Filter to a single lot. Omit for all lots. |

**Returns** JSON array of `LotOccupancy`:

```json
[
  {
    "lot_id": "…",
    "lot_name": "Main Lot",
    "total_slots": 10,
    "occupied_slots": 3,
    "available_slots": 7,
    "status": "open"
  }
]
```

`status` values: `open`, `closed`, `full`, `maintenance`.

---

### `list_my_bookings`

List all parking bookings for the authenticated user (identified by
`PARKHUB_API_KEY`).

**Parameters** — none required.

**Returns** JSON array of full `Booking` objects (same shape as
`GET /api/v1/bookings`), including past and upcoming bookings.

---

### `create_booking`

Create a parking booking for the authenticated user.

**Parameters**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `lot_id` | string (UUID) | Yes | Target lot |
| `slot_id` | string (UUID) | No | Specific slot. Omit to auto-select the first available slot in the lot. |
| `from` | string (RFC 3339) | Yes | Booking start (must be in the future) |
| `to` | string (RFC 3339) | Yes | Booking end |

**Returns** `BookingCreated` on success:

```json
{
  "booking_id": "…",
  "lot_id": "…",
  "slot_id": "…",
  "slot_number": 3,
  "start_time": "2025-06-15T09:00:00Z",
  "end_time": "2025-06-15T11:00:00Z",
  "status": "confirmed"
}
```

**Validation** mirrors the REST `POST /api/v1/bookings` path:

- `from` must be in the future.
- `to` must be after `from`.
- Lot must exist and have status `open`.
- Slot (explicit or auto-selected) must have status `available`.

On validation failure the tool returns a JSON object with an `"error"` field
describing the rejection reason (e.g. `"SLOT_UNAVAILABLE"`,
`"LOT_UNAVAILABLE"`, `"INVALID_BOOKING_TIME"`).

## Authentication note

The MCP server does **not** expose admin-only data. All tool calls operate
within the permissions of the resolved API-key user. Admin endpoints (lot
management, user administration, etc.) remain accessible only via the REST API
with admin credentials.
