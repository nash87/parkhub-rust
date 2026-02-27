# Installation Guide

Detailed installation instructions for ParkHub Rust.

## Prerequisites

**Docker Compose (recommended)**
- Docker Engine 24+ with Compose v2 (`docker compose` — not `docker-compose`)

**Bare metal / build from source**
- Rust 1.83 or later (`rustup update stable`)
- Node.js 22+ and npm (to build the web frontend)
- For the Windows GUI build: CMake, a C++ compiler

## Docker Compose (Recommended)

This is the fastest path to a running instance.

### 1. Clone the repository

```bash
git clone https://github.com/nash87/parkhub
cd parkhub
```

### 2. (Optional) Configure environment

The default `docker-compose.yml` works out of the box. To customize, create a `.env` file:

```env
PARKHUB_HOST=0.0.0.0
PARKHUB_PORT=8080
RUST_LOG=info

# Enable AES-256-GCM database encryption (recommended for production)
# PARKHUB_DB_PASSPHRASE=change-this-to-a-strong-passphrase

# Enable TLS (bring your own cert, or let the server auto-generate one)
# PARKHUB_TLS_ENABLED=true
# PARKHUB_TLS_CERT=/data/cert.pem
# PARKHUB_TLS_KEY=/data/key.pem
```

### 3. Start the service

```bash
docker compose up -d
```

### 4. Verify it is running

```bash
docker compose ps
curl http://localhost:8080/health
```

Open `http://localhost:8080` in your browser.

### 5. First-run setup

On first start, ParkHub automatically:
- Creates the admin account with username `admin` and password `admin`
- Creates a sample parking lot so you can explore the UI immediately

**Change the admin password immediately** via the UI or by recreating the admin user.

### Data persistence

The named volume `parkhub-data` stores the database and configuration:

```bash
# Backup
docker run --rm -v parkhub_parkhub-data:/data -v $(pwd):/backup alpine \
  tar czf /backup/parkhub-backup-$(date +%Y%m%d).tar.gz -C /data .

# Restore
docker run --rm -v parkhub_parkhub-data:/data -v $(pwd):/backup alpine \
  sh -c "cd /data && tar xzf /backup/parkhub-backup-YYYYMMDD.tar.gz"
```

### Updating

```bash
docker compose pull
docker compose up -d
```

---

## Kubernetes (Flux GitOps)

The project is designed for deployment in a Kubernetes cluster managed by Flux CD v2.

### Namespace and basic deployment

```yaml
# apps/parkhub/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: parkhub
```

```yaml
# apps/parkhub/deployment.yaml
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
      containers:
        - name: parkhub
          image: ghcr.io/nash87/parkhub:latest
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
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8080
            initialDelaySeconds: 5
          resources:
            requests:
              memory: "128Mi"
              cpu: "50m"
            limits:
              memory: "512Mi"
              cpu: "500m"
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: parkhub-data
```

```yaml
# apps/parkhub/pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: parkhub-data
  namespace: parkhub
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 5Gi
```

```yaml
# apps/parkhub/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: parkhub
  namespace: parkhub
spec:
  selector:
    app: parkhub
  ports:
    - port: 8080
      targetPort: 8080
```

Store the encryption passphrase in a Secret (use External Secrets Operator or kubectl):

```bash
kubectl create secret generic parkhub-secrets \
  --namespace parkhub \
  --from-literal=db-passphrase='your-strong-passphrase'
```

### Flux Kustomization

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

## Bare Metal (Build from Source)

### Build the web frontend

```bash
cd parkhub-web
npm ci
npm run build
cd ..
```

### Build the server binary

```bash
# Headless server (no GUI, recommended for Linux servers)
cargo build --release --package parkhub-server --no-default-features

# Server with Windows GUI (system tray + setup wizard)
cargo build --release --package parkhub-server --features gui
```

The resulting binary is at `target/release/parkhub-server`.

### Run

```bash
# Headless, auto-configure with defaults (admin/admin)
./target/release/parkhub-server --headless --unattended

# Headless with custom port and data directory
./target/release/parkhub-server --headless --port 8443 --data-dir /var/lib/parkhub

# With debug logging
./target/release/parkhub-server --headless --debug
```

### Systemd service

```ini
# /etc/systemd/system/parkhub.service
[Unit]
Description=ParkHub Parking Management Server
After=network.target

[Service]
Type=simple
User=parkhub
Group=parkhub
WorkingDirectory=/opt/parkhub
ExecStart=/opt/parkhub/parkhub-server --headless
Environment=RUST_LOG=info
Environment=PARKHUB_DB_PASSPHRASE=your-passphrase-here
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
sudo useradd --system --no-create-home parkhub
sudo mkdir -p /opt/parkhub /var/lib/parkhub
sudo cp target/release/parkhub-server /opt/parkhub/
sudo chown -R parkhub:parkhub /opt/parkhub /var/lib/parkhub
sudo systemctl daemon-reload
sudo systemctl enable --now parkhub
```

---

## TLS Configuration

### Auto-generated self-signed certificate (default)

Set `enable_tls = true` in `config.toml` (or `PARKHUB_TLS_ENABLED=true` in Docker).
On first start, the server generates a self-signed certificate using `rcgen` and saves it to
`data/cert.pem` and `data/key.pem`.

Browsers will show a warning for self-signed certs. Accept it once, or add the cert to your
system's trust store.

### Bring your own certificate

Place your certificate and key in the data directory and set in `config.toml`:

```toml
enable_tls = true
```

Then set environment variables or mount them in Docker:

```bash
PARKHUB_TLS_CERT=/data/cert.pem
PARKHUB_TLS_KEY=/data/key.pem
```

### Let's Encrypt via Caddy (recommended for public servers)

Run Caddy as a reverse proxy — it handles certificate renewal automatically:

```caddy
parkhub.example.com {
    reverse_proxy localhost:8080
}
```

When using a reverse proxy with TLS termination, disable TLS in ParkHub itself
(`enable_tls = false`) so it only handles plain HTTP internally.

---

## Reverse Proxy Examples

When ParkHub runs behind a reverse proxy, set `enable_tls = false` in ParkHub
and let the proxy handle TLS.

### nginx

```nginx
server {
    listen 443 ssl http2;
    server_name parkhub.example.com;

    ssl_certificate     /etc/letsencrypt/live/parkhub.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/parkhub.example.com/privkey.pem;

    location / {
        proxy_pass         http://127.0.0.1:8080;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
    }
}
```

### Caddy

```caddy
parkhub.example.com {
    reverse_proxy localhost:8080
}
```

### Traefik (Docker labels)

The default `docker-compose.yml` already includes Traefik labels:

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.parkhub.rule=Host(`parkhub.local`)"
  - "traefik.http.services.parkhub.loadbalancer.server.port=8080"
```

Add TLS and a cert resolver for HTTPS:

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.parkhub.rule=Host(`parkhub.example.com`)"
  - "traefik.http.routers.parkhub.tls.certresolver=letsencrypt"
```
