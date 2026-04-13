# PaaS Deployment Guide

ParkHub compiles to a single static binary that runs anywhere Linux runs. This guide
covers deployment to Render, Railway, and Fly.io.

> **GDPR note**: For production deployments subject to German/EU data protection
> requirements, self-hosted or private-cloud deployment provides full data sovereignty.
> PaaS platforms are ideal for demos, staging, and evaluation.

---

## Prerequisites

- A GitHub fork of the ParkHub repository (for automatic builds)
- An account on your chosen platform
- A persistent volume/disk (ParkHub uses an embedded SQLite database)

---

## Render

Render builds from your Dockerfile and provides persistent disks.

### Step 1 — Create a Web Service

1. Go to [dashboard.render.com](https://dashboard.render.com) and click **New Web Service**
2. Connect your GitHub fork of `parkhub-rust`
3. Configure:
   - **Environment**: Docker
   - **Region**: Frankfurt (eu-central) for EU data residency
   - **Instance Type**: Starter or Standard
   - **Docker Command Override**:
     ```
     /app/parkhub-server --headless --unattended --data-dir /data --port 10000
     ```

### Step 2 — Attach Persistent Disk

In the service settings, add a **Disk**:
- **Mount Path**: `/data`
- **Size**: 1 GB (sufficient for most installations)

### Step 3 — Set Environment Variables

In the **Environment** tab:

| Variable | Value |
|----------|-------|
| `PARKHUB_DB_PASSPHRASE` | A strong random passphrase (`openssl rand -base64 32`) |
| `PARKHUB_ADMIN_PASSWORD` | Your admin password |
| `RUST_LOG` | `info` |
| `PORT` | `10000` (Render assigns this; match with `--port`) |

Optional SMTP variables for email notifications:

| Variable | Value |
|----------|-------|
| `SMTP_HOST` | `smtp.example.com` |
| `SMTP_PORT` | `587` |
| `SMTP_USER` | `parkhub@example.com` |
| `SMTP_PASS` | Your SMTP password |
| `SMTP_FROM` | `parkhub@example.com` |

### Step 4 — Deploy

Click **Manual Deploy** or push to your fork's main branch. Render builds the Docker
image and starts the service. First build takes 10-15 minutes (dependency caching
speeds up subsequent builds).

### Step 5 — Verify

```bash
curl https://your-service.onrender.com/health/ready
# {"ready":true}
```

### Render Health Check

In service settings, configure the health check:
- **Path**: `/health/live`
- **Period**: 30 seconds

### Custom Domain

1. Add your domain in Render's **Settings > Custom Domains**
2. Create a CNAME record pointing to your Render URL
3. Render provisions TLS automatically

---

## Railway

Railway deploys from your Dockerfile with automatic builds on push.

### Step 1 — Create a Project

1. Go to [railway.app/new](https://railway.app/new) and click **Deploy from GitHub repo**
2. Select your fork of `parkhub-rust`
3. Railway detects the Dockerfile and starts building

### Step 2 — Configure Service

In the service settings:

- **Start Command** (override):
  ```
  /app/parkhub-server --headless --unattended --data-dir /data --port $PORT
  ```
  Railway injects `$PORT` at runtime. The `--port $PORT` flag binds to it.

### Step 3 — Add a Volume

1. Click **+ New** in your project and select **Volume**
2. Attach it to the ParkHub service
3. Set **Mount Path** to `/data`

### Step 4 — Set Variables

In the **Variables** tab:

```
PARKHUB_DB_PASSPHRASE=your-strong-passphrase
PARKHUB_ADMIN_PASSWORD=your-admin-password
RUST_LOG=info
```

### Step 5 — Deploy and Verify

Railway deploys automatically. Check the deployment logs for the admin password
if you did not set `PARKHUB_ADMIN_PASSWORD`.

```bash
curl https://your-project.up.railway.app/health/ready
```

### Railway Health Check

Railway uses the Dockerfile `HEALTHCHECK` instruction automatically.

### Custom Domain

1. Go to **Settings > Networking > Custom Domain**
2. Add your domain and configure the DNS CNAME

---

## Fly.io

Fly.io deploys Docker images to edge locations with persistent volumes.

### Step 1 — Install flyctl

```bash
curl -L https://fly.io/install.sh | sh
fly auth login
```

### Step 2 — Launch

```bash
cd parkhub-rust
fly launch --no-deploy
```

Select a region (e.g., `fra` for Frankfurt). This creates `fly.toml`.

### Step 3 — Configure fly.toml

```toml
app = "parkhub"
primary_region = "fra"

[build]
  dockerfile = "Dockerfile"

[env]
  RUST_LOG = "info"

[http_service]
  internal_port = 10000
  force_https = true
  auto_stop_machines = "suspend"
  auto_start_machines = true

[[http_service.checks]]
  grace_period = "60s"
  interval = "30s"
  method = "GET"
  path = "/health/live"
  timeout = "5s"

[mounts]
  source = "parkhub_data"
  destination = "/data"

[processes]
  app = "/app/parkhub-server --headless --unattended --data-dir /data --port 10000"
```

### Step 4 — Create Volume and Set Secrets

```bash
fly volumes create parkhub_data --region fra --size 1

fly secrets set \
  PARKHUB_DB_PASSPHRASE="$(openssl rand -base64 32)" \
  PARKHUB_ADMIN_PASSWORD="your-admin-password"
```

### Step 5 — Deploy

```bash
fly deploy
```

First deploy builds the image remotely (10-15 minutes). Subsequent deploys use
layer caching.

### Step 6 — Verify

```bash
fly status
curl https://parkhub.fly.dev/health/ready
```

### Custom Domain

```bash
fly certs add parking.yourdomain.com
```

Then create a CNAME record pointing to `parkhub.fly.dev`.

### Scaling

Fly.io suspends idle machines by default (`auto_stop_machines = "suspend"`). For
always-on production:

```toml
auto_stop_machines = "off"
```

---

## Platform Comparison

| Feature | Render | Railway | Fly.io |
|---------|--------|---------|--------|
| Build from Dockerfile | Yes | Yes | Yes |
| Persistent volume | Disk (1-100 GB) | Volume | Volume (1-500 GB) |
| EU region | Frankfurt | Limited | Frankfurt, Amsterdam, London |
| Free tier | Yes (spins down) | $5 credit/mo | $5 credit/mo |
| Custom domain + TLS | Automatic | Automatic | Automatic |
| Health checks | HTTP path | Dockerfile HEALTHCHECK | HTTP path |
| Auto-deploy on push | Yes | Yes | Yes |

---

## Common Configuration

Regardless of platform, these environment variables apply:

| Variable | Required | Description |
|----------|----------|-------------|
| `PARKHUB_DB_PASSPHRASE` | Recommended | AES-256-GCM encryption passphrase |
| `PARKHUB_ADMIN_PASSWORD` | Optional | Admin password (auto-generated if unset) |
| `RUST_LOG` | Optional | Log level (`info`, `debug`, `warn`) |
| `SMTP_HOST` | Optional | SMTP server for email notifications |
| `SMTP_PORT` | Optional | SMTP port (default: 587) |
| `SMTP_USER` | Optional | SMTP auth username |
| `SMTP_PASS` | Optional | SMTP auth password |
| `SMTP_FROM` | Optional | Sender email address |
| `APP_URL` | Optional | Public URL for email links |

Port is always configured via the `--port` CLI flag, not an environment variable.
Match it with whatever port the platform expects (Render uses 10000, Railway injects
`$PORT`, Fly.io uses `internal_port`).

---

## Backup on PaaS

ParkHub creates automatic daily backups in `/data/backups/`. To download a backup:

### Render

Use the Render Shell feature or SSH into the service:
```bash
render ssh --service your-service-id
tar czf /tmp/backup.tar.gz -C /data .
# Download via Render dashboard
```

### Fly.io

```bash
fly ssh console
tar czf /tmp/backup.tar.gz -C /data .
exit
fly ssh sftp get /tmp/backup.tar.gz ./backup.tar.gz
```

### Railway

Railway does not provide direct shell access. Use the Railway CLI:
```bash
railway run tar czf /tmp/backup.tar.gz -C /data .
railway run cat /tmp/backup.tar.gz > backup.tar.gz
```

---

## Upgrading on PaaS

Push to your fork's main branch. All three platforms detect the change and rebuild
automatically. Database migrations apply on startup — no manual intervention needed.

For major version upgrades, download a backup first (see above).
