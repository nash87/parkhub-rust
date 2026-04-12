#!/usr/bin/env bash
set -euo pipefail

# ParkHub Rust — Quick Setup
# Usage: curl -fsSL https://raw.githubusercontent.com/nash87/parkhub-rust/main/install.sh | bash

VERSION="${PARKHUB_VERSION:-latest}"
PORT="${PARKHUB_PORT:-8080}"
ADMIN_PASSWORD="${PARKHUB_ADMIN_PASSWORD:-}"

echo "╔══════════════════════════════════════════════╗"
echo "║          ParkHub Rust — Quick Setup          ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# Check Docker
if ! command -v docker &>/dev/null; then
  echo "❌ Docker not found. Install: https://docs.docker.com/get-docker/"
  exit 1
fi

if ! docker compose version &>/dev/null; then
  echo "❌ Docker Compose not found. Install: https://docs.docker.com/compose/install/"
  exit 1
fi

echo "✓ Docker + Compose found"

# Generate password if not set
if [ -z "$ADMIN_PASSWORD" ]; then
  ADMIN_PASSWORD=$(openssl rand -base64 16 | tr -dc 'A-Za-z0-9' | head -c16)
  echo "✓ Generated admin password: $ADMIN_PASSWORD"
  echo "  ⚠ Save this password — it won't be shown again!"
fi

# Download docker-compose.yml
echo "→ Downloading docker-compose.yml..."
curl -fsSL "https://raw.githubusercontent.com/nash87/parkhub-rust/main/docker-compose.yml" -o docker-compose.yml

# Create .env
cat > .env <<EOF
PARKHUB_ADMIN_PASSWORD=$ADMIN_PASSWORD
# PARKHUB_DB_PASSPHRASE=change-me-for-encryption
EOF

# Update port if non-default
if [ "$PORT" != "8080" ]; then
  sed -i "s/8080:8080/$PORT:8080/" docker-compose.yml
fi

# Start
echo "→ Starting ParkHub..."
docker compose up -d

echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║  ✅ ParkHub is running!                      ║"
echo "║                                              ║"
echo "║  URL:      http://localhost:$PORT             "
echo "║  Admin:    admin / $ADMIN_PASSWORD            "
echo "║  API Docs: http://localhost:$PORT/api/v1/docs "
echo "║                                              ║"
echo "║  Stop:  docker compose down                  ║"
echo "║  Logs:  docker compose logs -f               ║"
echo "╚══════════════════════════════════════════════╝"
