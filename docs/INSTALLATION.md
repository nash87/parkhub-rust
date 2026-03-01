# Installation Guide — ParkHub Rust

> **Supported platforms**: Linux, macOS, Windows, Docker, Kubernetes, bare metal.
> Docker Compose is the recommended path for new deployments.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Compose (Recommended)](#docker-compose-recommended)
- [VPS / Ubuntu 24.04 (Bare Metal)](#vps--ubuntu-2404-bare-metal)
- [Kubernetes](#kubernetes)
- [PaaS (Railway, Render, Fly.io)](#paas-railway-render-flyio)
- [Windows Desktop (GUI)](#windows-desktop-gui)
- [TLS Configuration](#tls-configuration)
- [Reverse Proxy Examples](#reverse-proxy-examples)
- [Backup Strategy](#backup-strategy)
- [Upgrade Guide](#upgrade-guide)

---

## Prerequisites

| Deployment | Requirements |
|-----------|--------------|
| Docker Compose | Docker Engine 24+, Compose v2 (`docker compose`) |
| Bare metal / source | Rust 1.84+, Node.js 22+, npm |
| Windows GUI build | Rust 1.84+, Node.js 22+, CMake, C++ compiler |
| Kubernetes | kubectl, a running cluster, a PVC storage class |

---

## Docker Compose (Recommended)

Three commands from clone to running instance.

### Step 1 — Clone

```bash
git clone https://github.com/nash87/parkhub-rust
cd parkhub-rust
```

### Step 2 — (Optional) Enable database encryption

For production, set the `PARKHUB_DB_PASSPHRASE` environment variable to enable AES-256-GCM at-rest encryption.
Edit `docker-compose.yml` and uncomment the `PARKHUB_DB_PASSPHRASE` line, then set a strong random value:

```bash
# Generate a strong random passphrase
openssl rand -base64 32
```

Then in `docker-compose.yml`:
```yaml
    environment:
      - RUST_LOG=info
      - PARKHUB_DB_PASSPHRASE=your-generated-passphrase-here
```

> Never commit the passphrase to version control. Store it in a password manager or secret vault.

> **Note on port configuration**: The port is set via the `--port` CLI flag in the `command` section of
> `docker-compose.yml` (not an environment variable). To change the port, edit both the `--port` value
> in `command` and the `ports` mapping.

### Step 3 — Start

```bash
docker compose up -d
```

### Step 4 — Verify

```bash
docker compose ps
curl http://localhost:8080/health/live
# HTTP 200 OK
```

Open `http://localhost:8080` in your browser.

### First Boot

On first start, ParkHub automatically:

- Creates an admin account: username `admin`, password `admin`
- Creates a sample parking lot for immediate exploration

**Change the admin password immediately** after first login via the UI (Profile → Change Password) or via the API.

### Data Persistence

The named Docker volume `parkhub-data` stores the database and all configuration.

> **Volume naming**: Docker Compose prefixes the volume name with the project name (the directory you
> cloned into). If you cloned into `parkhub-rust/`, the volume is named `parkhub-rust_parkhub-data`.
> Check with: `docker volume ls | grep parkhub`

```bash
# Find the actual volume name
VOLUME=$(docker volume ls --format '{{.Name}}' | grep parkhub-data)

# Backup
docker run --rm \
  -v "$VOLUME":/data \
  -v $(pwd):/backup alpine \
  tar czf /backup/parkhub-backup-$(date +%Y%m%d).tar.gz -C /data .

# Restore
docker run --rm \
  -v "$VOLUME":/data \
  -v $(pwd):/backup alpine \
  sh -c "cd /data && tar xzf /backup/parkhub-backup-YYYYMMDD.tar.gz"
```

---

## VPS / Ubuntu 24.04 (Bare Metal)

This guide targets Ubuntu 24.04 LTS. Adapt for Debian or Rocky Linux as needed.

### Step 1 — Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
```

### Step 2 — Install Node.js 22

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt-get install -y nodejs
```

### Step 3 — Build ParkHub

```bash
git clone https://github.com/nash87/parkhub-rust
cd parkhub-rust

# Build the React frontend
cd parkhub-web
npm ci
npm run build
cd ..

# Build the server (headless, no GUI)
cargo build --release --package parkhub-server --no-default-features --features headless
```

The compiled binary is at `target/release/parkhub-server`.

### Step 4 — Create a system user and directories

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin parkhub
sudo mkdir -p /opt/parkhub /var/lib/parkhub
sudo cp target/release/parkhub-server /opt/parkhub/
sudo chown -R parkhub:parkhub /opt/parkhub /var/lib/parkhub
```

### Step 5 — Create a systemd service

```ini
# /etc/systemd/system/parkhub.service
[Unit]
Description=ParkHub Parking Management Server
After=network.target

[Service]
Type=simple
User=parkhub
Group=parkhub
WorkingDirectory=/var/lib/parkhub
ExecStart=/opt/parkhub/parkhub-server --headless --data-dir /var/lib/parkhub
Environment=RUST_LOG=info
Environment=PARKHUB_DB_PASSPHRASE=your-strong-passphrase-here
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/parkhub

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now parkhub
sudo systemctl status parkhub
```

Verify:

```bash
curl http://localhost:8080/health/ready
# {"ready":true}
```

### Step 6 — Configure nginx reverse proxy with HTTPS

```bash
sudo apt install -y nginx certbot python3-certbot-nginx
```

```nginx
# /etc/nginx/sites-available/parkhub
server {
    listen 80;
    server_name parking.yourdomain.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name parking.yourdomain.com;

    ssl_certificate     /etc/letsencrypt/live/parking.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/parking.yourdomain.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;

    # HSTS
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location / {
        proxy_pass         http://127.0.0.1:8080;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
        proxy_read_timeout 60s;
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/parkhub /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
sudo certbot --nginx -d parking.yourdomain.com
```

When using a reverse proxy with TLS termination, disable TLS in ParkHub:

```toml
# /var/lib/parkhub/config.toml
enable_tls = false
```

---

## Kubernetes

### Namespace

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: parkhub
```

### PersistentVolumeClaim

```yaml
# k8s/pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: parkhub-data
  namespace: parkhub
spec:
  accessModes: [ReadWriteOnce]
  storageClassName: standard   # change to your storage class
  resources:
    requests:
      storage: 5Gi
```

### Secret

```bash
kubectl create secret generic parkhub-secrets \
  --namespace parkhub \
  --from-literal=db-passphrase='your-strong-passphrase'
```

Or use External Secrets Operator / Sealed Secrets for GitOps-managed secrets.

### Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: parkhub
  namespace: parkhub
spec:
  replicas: 1
  selector:
    matchLabels:
      app: parkhub
  template:
    metadata:
      labels:
        app: parkhub
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
        - name: parkhub
          image: ghcr.io/nash87/parkhub-rust:v1.2.0  # Pin to specific version; check releases for latest
          ports:
            - containerPort: 8080
          env:
            - name: PARKHUB_HOST
              value: "0.0.0.0"
            - name: PARKHUB_PORT
              value: "8080"
            - name: RUST_LOG
              value: "info"
            - name: PARKHUB_DB_PASSPHRASE
              valueFrom:
                secretKeyRef:
                  name: parkhub-secrets
                  key: db-passphrase
          volumeMounts:
            - name: data
              mountPath: /data
          livenessProbe:
            httpGet:
              path: /health/live
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 30
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            requests:
              memory: "128Mi"
              cpu: "50m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: false
            capabilities:
              drop: [ALL]
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: parkhub-data
```

### Service

```yaml
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: parkhub
  namespace: parkhub
spec:
  selector:
    app: parkhub
  ports:
    - port: 80
      targetPort: 8080
  type: ClusterIP
```

### Ingress (nginx-ingress example)

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: parkhub
  namespace: parkhub
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
    - hosts: [parking.yourdomain.com]
      secretName: parkhub-tls
  rules:
    - host: parking.yourdomain.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: parkhub
                port:
                  number: 80
```

### Flux GitOps Kustomization

```yaml
# clusters/production/apps-parkhub.yaml
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: apps-parkhub
  namespace: flux-system
spec:
  interval: 10m
  path: ./apps/parkhub
  prune: true
  sourceRef:
    kind: GitRepository
    name: flux-infra
```

---

## PaaS (Railway, Render, Fly.io)

> Note: PaaS deployments suit development and evaluation. For production with German/EU GDPR
> requirements, an on-premise or private cloud deployment gives you full data sovereignty.

### Railway

1. Fork the repository on GitHub
2. Create a new project at [railway.app](https://railway.app)
3. Connect your GitHub fork
4. Set environment variables in the Railway dashboard:
   - `PARKHUB_DB_PASSPHRASE` — required for encryption
   - `PARKHUB_PORT` — set to the port Railway assigns (use `$PORT` variable)
5. Attach a persistent volume for `/data`

### Render

1. Fork the repository
2. New Web Service → connect your fork
3. Environment: set `PARKHUB_DB_PASSPHRASE` and any other variables
4. Mount a Persistent Disk at `/data`

### Fly.io

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Deploy
fly launch
fly secrets set PARKHUB_DB_PASSPHRASE="your-passphrase"
fly volumes create parkhub_data --size 5
```

Update `fly.toml`:

```toml
[mounts]
  source = "parkhub_data"
  destination = "/data"
```

```bash
fly deploy
```

---

## Windows Desktop (GUI)

ParkHub includes a Windows desktop application with a system tray icon and setup wizard.

Download the pre-built installer from the GitHub Releases page or build from source:

```powershell
cargo build --release --package parkhub-server --features gui
```

The `parkhub-server.exe` binary:
- Shows a setup wizard on first launch
- Runs as a system tray application
- Stores data in `%APPDATA%\parkhub\ParkHub Server\`

---

## TLS Configuration

### Auto-generated self-signed certificate

Set `enable_tls = true` in `config.toml`. On first start, ParkHub generates a self-signed
certificate using `rcgen` and saves it to `data/cert.pem` and `data/key.pem`.

Browsers display a warning for self-signed certificates. Accept once, or add the certificate
to your system trust store.

### Bring your own certificate

Place your certificate and private key in the data directory, then:

```toml
enable_tls = true
```

Point the paths via environment variables:

```env
PARKHUB_TLS_CERT=/data/cert.pem
PARKHUB_TLS_KEY=/data/key.pem
```

### Let's Encrypt via Caddy (recommended for public servers)

```caddy
# Caddyfile
parking.yourdomain.com {
    reverse_proxy localhost:8080
}
```

Caddy handles certificate issuance and renewal automatically. Disable TLS in ParkHub when
using Caddy (`enable_tls = false`).

---

## Reverse Proxy Examples

When ParkHub runs behind a reverse proxy, disable TLS in ParkHub (`enable_tls = false`)
and let the proxy handle HTTPS.

### nginx

See the [VPS section](#step-6--configure-nginx-reverse-proxy-with-https) above.

### Caddy

```caddy
parking.yourdomain.com {
    reverse_proxy localhost:8080
}
```

### Traefik (Docker labels)

```yaml
# In docker-compose.yml services.parkhub:
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.parkhub.rule=Host(`parking.yourdomain.com`)"
  - "traefik.http.routers.parkhub.tls.certresolver=letsencrypt"
  - "traefik.http.services.parkhub.loadbalancer.server.port=8080"
```

---

## Backup Strategy

### Automatic backups

ParkHub creates daily backups of the database automatically. Configuration:

```toml
auto_backup_enabled = true
backup_retention_count = 7     # keep 7 daily backups
```

Backup files are stored in `data/backups/` next to the main database.

### Manual backup (Docker)

```bash
# Find the volume name (project prefix varies based on clone directory name)
VOLUME=$(docker volume ls --format '{{.Name}}' | grep parkhub-data)

# Create a timestamped backup tarball
mkdir -p backups
docker run --rm \
  -v "$VOLUME":/data \
  -v $(pwd)/backups:/backup alpine \
  tar czf /backup/parkhub-$(date +%Y%m%d-%H%M%S).tar.gz -C /data .
```

### Off-site backup (recommended for production)

Copy the backup tarball to an off-site location:

```bash
# Example: copy to a remote server via rsync
rsync -avz ./backups/ user@backup-server:/parkhub-backups/
```

For maximum data safety, store at least one backup copy off-site or in a different physical
location. The encrypted database file is safe to store on untrusted storage — it requires
the passphrase to decrypt.

---

## Upgrade Guide

### Docker Compose

```bash
git pull
docker compose pull
docker compose up -d --build
```

Database migrations run automatically. The server starts, applies any schema changes,
and serves traffic.

### Bare metal

```bash
git pull

# Rebuild frontend
cd parkhub-web && npm ci && npm run build && cd ..

# Rebuild and restart
cargo build --release --package parkhub-server --no-default-features --features headless
sudo systemctl stop parkhub
sudo cp target/release/parkhub-server /opt/parkhub/
sudo systemctl start parkhub
```

### Backup before upgrading

Always create a manual backup before upgrading a production instance:

```bash
VOLUME=$(docker volume ls --format '{{.Name}}' | grep parkhub-data)
docker run --rm -v "$VOLUME":/data -v $(pwd):/backup alpine \
  tar czf /backup/pre-upgrade-$(date +%Y%m%d).tar.gz -C /data .
```

---

## Health Check Endpoints

| Endpoint | Purpose | Auth |
|----------|---------|------|
| `GET /health` | Simple liveness — returns `OK` (plain text) | None |
| `GET /health/live` | Kubernetes liveness probe — HTTP 200 if process is alive | None |
| `GET /health/ready` | Kubernetes readiness probe — HTTP 200 if database is accessible | None |

```bash
curl http://localhost:8080/health/ready
# {"ready":true}
```

Returns HTTP 503 with `{"ready":false}` if the database is not operational.

---

## First-Run Checklist

After installation, verify the following before putting ParkHub into production:

- [ ] Login at `http(s)://your-host:port` succeeds with `admin` / `admin`
- [ ] Admin password changed to a strong unique password
- [ ] `PARKHUB_DB_PASSPHRASE` set (database encryption enabled)
- [ ] TLS active (self-signed, own cert, or reverse proxy)
- [ ] `allow_self_registration = false` in `config.toml` (unless open registration is desired)
- [ ] Impressum filled in: Admin panel → Impressum (`/impressum` is publicly reachable)
- [ ] First parking lot and slots created
- [ ] `GET /health/ready` returns `{"ready":true}`
- [ ] Backup strategy configured and tested
- [ ] Audit logging enabled (`audit_logging_enabled = true`)
