# Build stage - Web Frontend
FROM node:22-alpine AS web-builder
WORKDIR /app/web
COPY parkhub-web/package*.json ./
RUN npm ci
COPY parkhub-web/ ./
RUN npm run build

# Build stage - Rust Server
FROM rust:1.83-alpine AS rust-builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig cmake make perl clang
WORKDIR /app
# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY parkhub-common/Cargo.toml ./parkhub-common/
COPY parkhub-server/Cargo.toml ./parkhub-server/
# Create dummy sources for dependency caching
RUN mkdir -p parkhub-common/src parkhub-server/src && \
    echo "pub fn dummy() {}" > parkhub-common/src/lib.rs && \
    echo "fn main() {}" > parkhub-server/src/main.rs
# Build dependencies only
RUN cargo build --release --package parkhub-server 2>/dev/null || true
# Copy real sources
COPY parkhub-common/ ./parkhub-common/
COPY parkhub-server/ ./parkhub-server/
# Copy web build
COPY --from=web-builder /app/web/dist ./parkhub-web/dist/
# Build the actual binary
RUN touch parkhub-common/src/lib.rs parkhub-server/src/main.rs && \
    cargo build --release --package parkhub-server

# Runtime stage
FROM alpine:3.20
RUN apk add --no-cache ca-certificates tzdata
WORKDIR /app

# Copy binary
COPY --from=rust-builder /app/target/release/parkhub-server /app/parkhub-server

# Create data directory
RUN mkdir -p /data

# Environment
ENV PARKHUB_DATA_DIR=/data
ENV PARKHUB_HOST=0.0.0.0
ENV PARKHUB_PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Run
ENTRYPOINT ["/app/parkhub-server"]
