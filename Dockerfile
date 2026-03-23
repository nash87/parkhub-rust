# =============================================================================
# ParkHub Rust — Optimized multi-stage Docker build
# Uses cargo-chef for dependency layer caching, fat LTO for smallest binary,
# and minimal Debian slim runtime.
# =============================================================================

# ---------------------------------------------------------------------------
# Stage 1: Frontend build (Astro/Vite)
# ---------------------------------------------------------------------------
FROM node:22-alpine AS web-builder
WORKDIR /app
COPY parkhub-web/package*.json ./
RUN npm ci
COPY parkhub-web/ ./
RUN DOCKER=1 npm run build

# ---------------------------------------------------------------------------
# Stage 2: Cargo chef — prepare dependency recipe
# ---------------------------------------------------------------------------
FROM rust:1.94-slim AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev cmake make perl clang curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

# ---------------------------------------------------------------------------
# Stage 3: Plan — generate recipe.json (changes only when deps change)
# ---------------------------------------------------------------------------
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
# Exclude desktop-only parkhub-client from workspace
RUN sed -i '/"parkhub-client"/d' Cargo.toml
COPY parkhub-common/Cargo.toml ./parkhub-common/
COPY parkhub-server/Cargo.toml ./parkhub-server/
COPY parkhub-common/src ./parkhub-common/src
COPY parkhub-server/src ./parkhub-server/src
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------
# Stage 4: Cook — build only dependencies (cached layer)
# ---------------------------------------------------------------------------
FROM chef AS deps
COPY --from=planner /app/recipe.json recipe.json
COPY Cargo.toml Cargo.lock ./
RUN sed -i '/"parkhub-client"/d' Cargo.toml
RUN cargo chef cook --release --recipe-path recipe.json \
    --package parkhub-server --no-default-features --features headless

# ---------------------------------------------------------------------------
# Stage 5: Build — compile the actual application
# ---------------------------------------------------------------------------
FROM deps AS builder
# Copy real source (deps are already compiled)
COPY parkhub-common/ ./parkhub-common/
COPY parkhub-server/ ./parkhub-server/
# Copy frontend build output
COPY --from=web-builder /app/dist ./parkhub-web/dist/
# Touch sources to invalidate fingerprints, then build
RUN touch parkhub-common/src/lib.rs parkhub-server/src/main.rs && \
    cargo build --release --package parkhub-server \
        --no-default-features --features headless && \
    # Strip is in Cargo.toml profile.release, but ensure it
    strip /app/target/release/parkhub-server || true

# ---------------------------------------------------------------------------
# Stage 6: Runtime — minimal Debian slim (python3 needed for seed script)
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates tzdata python3 wget \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r parkhub && useradd -r -g parkhub -s /sbin/nologin parkhub

WORKDIR /app

# Copy binary
COPY --from=builder --chown=parkhub:parkhub /app/target/release/parkhub-server /app/parkhub-server

# Copy seed script and entrypoint
COPY --chown=parkhub:parkhub scripts/seed_demo.py /app/seed_demo.py
COPY --chown=parkhub:parkhub scripts/docker-entrypoint.sh /app/docker-entrypoint.sh

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

# Health check — longer start-period for --unattended first-run + demo seeding
HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=5 \
    CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:10000/health || exit 1

# Entrypoint handles: start server -> wait healthy -> seed demo data -> keep running
CMD ["/app/docker-entrypoint.sh"]
