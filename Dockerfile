# Build stage - Web Frontend
FROM node:22-alpine AS web-builder
WORKDIR /app/web
COPY parkhub-web/package*.json ./
ENV NODE_ENV=production
RUN npm ci --omit=dev
COPY parkhub-web/ ./
RUN npm run build

# Build stage - Rust Server
# Pin minor version to avoid silent toolchain drift; update intentionally
FROM rust:1-alpine AS rust-builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig cmake make perl clang curl
WORKDIR /app
# Copy manifests first for layer caching
# Note: parkhub-client is a desktop GUI app with heavy deps (Slint/GTK/GObject)
# that cannot build in Alpine. We rewrite Cargo.toml to exclude it from the
# workspace so only parkhub-common and parkhub-server are resolved.
COPY Cargo.toml Cargo.lock ./
RUN sed -i '/"parkhub-client"/d' Cargo.toml
COPY parkhub-common/Cargo.toml ./parkhub-common/
COPY parkhub-server/Cargo.toml ./parkhub-server/
# Create dummy sources for dependency caching
RUN mkdir -p parkhub-common/src parkhub-server/src && \
    echo "pub fn dummy() {}" > parkhub-common/src/lib.rs && \
    echo "fn main() {}" > parkhub-server/src/main.rs
# Build dependencies only (headless: no Slint GUI, no tray-icon/GTK deps)
RUN cargo build --release --package parkhub-server --no-default-features --features headless 2>/dev/null || true
# Copy real sources
COPY parkhub-common/ ./parkhub-common/
COPY parkhub-server/ ./parkhub-server/
# Copy web build
COPY --from=web-builder /app/web/dist ./parkhub-web/dist/
# Build the actual binary
RUN touch parkhub-common/src/lib.rs parkhub-server/src/main.rs && \
    cargo build --release --package parkhub-server --no-default-features --features headless

# Runtime stage — minimal Alpine, non-root user
FROM alpine:3.20
RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -S parkhub && adduser -S -G parkhub parkhub
WORKDIR /app

# Copy binary with correct ownership
COPY --chown=parkhub:parkhub --from=rust-builder /app/target/release/parkhub-server /app/parkhub-server

# Create data directory owned by the non-root user
RUN mkdir -p /data && chown parkhub:parkhub /data

# Drop to non-root
USER parkhub

# Environment
ENV PARKHUB_DATA_DIR=/data
ENV PARKHUB_HOST=0.0.0.0
ENV PARKHUB_PORT=10000
ENV RUST_LOG=info

EXPOSE 10000

# Health check — longer start-period for --unattended first-run auto-config
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=5 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:10000/health || exit 1

# Run
ENTRYPOINT ["/app/parkhub-server"]
