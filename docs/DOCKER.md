# Docker Deployment Guide

ParkHub ships a production-ready multi-stage Dockerfile that builds a ~20 MB distroless
image. This guide covers building, running, customizing, and operating ParkHub with Docker.

For quick-start Docker Compose instructions, see [INSTALLATION.md](INSTALLATION.md#docker-compose-recommended).

---

## Image Architecture

The Dockerfile uses six stages:

| Stage | Base | Purpose |
|-------|------|---------|
| `web-builder` | `node:22-alpine` | Builds the Astro/Vite frontend |
| `chef` / `planner` | `rust:1.94-slim` | Generates a cargo-chef dependency recipe |
| `deps` | `rust:1.94-slim` | Cooks (compiles) only dependencies — cached layer |
| `builder` | (inherits deps) | Compiles application source against pre-built deps |
| `data-setup` | `busybox` | Creates `/data` with correct ownership for distroless |
| `runtime` | `gcr.io/distroless/cc-debian12` | Final image — no shell, no package manager |

Rebuilds that only change application code skip the dependency cook stage entirely,
yielding sub-minute incremental builds.

---

## Quick Start

```bash
git clone https://github.com/nash87/parkhub-rust && cd parkhub-rust
docker compose up -d
```

ParkHub is available at `http://localhost:8080`. Check the logs for the auto-generated
admin password:

```bash
docker compose logs parkhub | grep -i password
```

---

## Building the Image

### With Docker Compose

```bash
docker compose build
```

### Standalone

```bash
docker build -t parkhub:latest .
```

### Build Arguments

The Dockerfile does not expose build-args. To change the Rust toolchain version, edit
the `FROM rust:1.94-slim` line in the Dockerfile.

---

## Running

### Minimal

```bash
docker run -d \
  --name parkhub \
  -p 8080:8080 \
  -v parkhub-data:/data \
  parkhub:latest \
  /app/parkhub-server --headless --unattended --data-dir /data --port 8080
```

### With Encryption and SMTP

```bash
docker run -d \
  --name parkhub \
  -p 8080:8080 \
  -v parkhub-data:/data \
  -e PARKHUB_DB_PASSPHRASE="$(openssl rand -base64 32)" \
  -e PARKHUB_ADMIN_PASSWORD="StrongAdminPass!42" \
  -e SMTP_HOST=smtp.example.com \
  -e SMTP_PORT=587 \
  -e SMTP_USER=parkhub@example.com \
  -e SMTP_PASS=smtp-secret \
  -e SMTP_FROM=parkhub@example.com \
  -e RUST_LOG=info \
  parkhub:latest \
  /app/parkhub-server --headless --unattended --data-dir /data --port 8080
```

### Port Configuration

The port is set via the `--port` CLI flag, not an environment variable. When changing
the port, update both the flag and the `-p` mapping:

```bash
docker run -d -p 3000:3000 -v parkhub-data:/data parkhub:latest \
  /app/parkhub-server --headless --unattended --data-dir /data --port 3000
```

---

## Docker Compose Reference

The repository includes three Compose files:

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Production deployment |
| `docker-compose.override.yml.example` | Development overrides (debug logging) |
| `docker-compose.test.yml` | CI pipeline: app + E2E tests + k6 load tests |

### Production

```bash
docker compose up -d
docker compose ps
curl http://localhost:8080/health/ready
```

### Development

```bash
cp docker-compose.override.yml.example docker-compose.override.yml
docker compose up -d
# Runs with RUST_LOG=debug and admin password "admin"
```

### Test Pipeline

```bash
docker compose -f docker-compose.test.yml up --build --abort-on-container-exit
```

This builds the app, waits for the health check, then runs Playwright E2E tests.
Add the `load-test` profile for k6:

```bash
docker compose -f docker-compose.test.yml --profile load-test up --build --abort-on-container-exit
```

---

## Health Checks

The distroless image has no shell or curl. Health checks use the binary's built-in
`--health-check` mode:

```yaml
healthcheck:
  test: ["CMD", "/app/parkhub-server", "--health-check", "--port", "8080"]
  interval: 30s
  timeout: 5s
  retries: 3
  start_period: 60s
```

HTTP endpoints for external monitoring:

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | Liveness — returns `OK` |
| `GET /health/live` | Kubernetes-style liveness probe |
| `GET /health/ready` | Readiness — checks database accessibility |

---

## Data and Backups

All persistent state lives in the `/data` volume: the SQLite database, config.toml,
TLS certificates, and automatic daily backups.

### Backup

```bash
VOLUME=$(docker volume ls --format '{{.Name}}' | grep parkhub-data)
docker run --rm -v "$VOLUME":/data -v "$(pwd)/backups":/backup alpine \
  tar czf "/backup/parkhub-$(date +%Y%m%d-%H%M%S).tar.gz" -C /data .
```

### Restore

```bash
docker compose down
docker run --rm -v "$VOLUME":/data -v "$(pwd)/backups":/backup alpine \
  sh -c "rm -rf /data/* && tar xzf /backup/parkhub-YYYYMMDD-HHMMSS.tar.gz -C /data"
docker compose up -d
```

---

## Reverse Proxy

### Traefik (labels in docker-compose.yml)

The default `docker-compose.yml` includes Traefik labels. To use them:

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.parkhub.rule=Host(`parking.yourdomain.com`)"
  - "traefik.http.routers.parkhub.tls.certresolver=letsencrypt"
  - "traefik.http.services.parkhub.loadbalancer.server.port=8080"
```

### Caddy

```caddy
parking.yourdomain.com {
    reverse_proxy parkhub:8080
}
```

When using a reverse proxy with TLS termination, set `enable_tls = false` in
`/data/config.toml` inside the container to avoid double encryption.

---

## Resource Limits

The default Compose file sets:

```yaml
deploy:
  resources:
    limits:
      memory: 256M
    reservations:
      memory: 64M
```

ParkHub idles at ~15 MB RSS. The 256 MB limit handles large imports and concurrent
load comfortably. Adjust if your instance manages thousands of concurrent users.

---

## Security Notes

- The runtime image is distroless: no shell, no package manager, no attack surface
- The process runs as UID 65532 (distroless nonroot user)
- Enable `PARKHUB_DB_PASSPHRASE` for AES-256-GCM database encryption at rest
- The image does not contain build tools, source code, or Node.js
- Scan with `docker scout cves parkhub:latest` or Trivy for vulnerability reports

---

## Upgrading

```bash
git pull
docker compose up -d --build
```

The server applies database migrations automatically on startup. Always back up
`/data` before upgrading a production instance.
