# =============================================================================
# ParkHub Rust — Optimized multi-stage Docker build
# Uses cargo-chef for dependency layer caching, fat LTO for smallest binary,
# and a distroless runtime for minimal attack surface (~20 MB image).
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
# Re-copy root manifest so [workspace.lints] is present (cargo-chef strips it)
COPY Cargo.toml Cargo.lock ./
RUN sed -i '/"parkhub-client"/d' Cargo.toml
# Copy real source (deps are already compiled)
COPY parkhub-common/ ./parkhub-common/
COPY parkhub-server/ ./parkhub-server/
# Copy frontend build output
COPY --from=web-builder /app/dist ./parkhub-web/dist/
# Touch sources to invalidate fingerprints, then build
RUN touch parkhub-common/src/lib.rs parkhub-server/src/main.rs && \
    cargo build --release --package parkhub-server \
        --no-default-features --features headless && \
    strip /app/target/release/parkhub-server

# ---------------------------------------------------------------------------
# Stage 6: Data-directory scaffold
# distroless has no shell, so we create /data with the correct UID here and
# copy the empty directory tree into the final image.
# UID 65532 is the built-in "nonroot" user in gcr.io/distroless/cc-debian12.
# ---------------------------------------------------------------------------
FROM busybox:1.37.0 AS data-setup
RUN mkdir -p /data && chown 65532:65532 /data

# ---------------------------------------------------------------------------
# Stage 7: Runtime — distroless/cc for minimal attack surface
# No shell, no package manager, no wget — just glibc + libstdc++ + ca-certs.
# Demo seeding and health checks are handled by the binary itself.
#
# debian13 (trixie) matches rust:1.94-slim's base image, so OpenSSL
# symbol versions align between build and runtime. Using debian12
# caused `libssl.so.3: version OPENSSL_3.2.0 not found` at startup.
# Digest-pinned for reproducibility and to satisfy CodeQL DS-0001
# (':latest' tag used).
# ---------------------------------------------------------------------------
FROM gcr.io/distroless/cc-debian13@sha256:9d412062635760e6aaf0f73809c3c6ea60f389d4986713fe153667e95c6b7b72 AS runtime

WORKDIR /app

# Copy binary
COPY --from=builder --chown=65532:65532 /app/target/release/parkhub-server /app/parkhub-server

# Copy pre-created /data directory (owned by nonroot UID 65532).
# --chown is required: COPY --from defaults to root:root regardless of
# the source stage's ownership, so the busybox chown above would be
# discarded without this flag.
COPY --from=data-setup --chown=65532:65532 /data /data

# Drop to non-root (distroless built-in nonroot user)
USER 65532:65532

# Environment
ENV RUST_LOG=info

EXPOSE 10000

# Health check — uses the binary's built-in --health-check mode so no shell
# or external tools (wget/curl) are needed in the distroless image.
HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=5 \
    CMD ["/app/parkhub-server", "--health-check", "--port", "10000"]

# Direct binary invocation — no shell wrapper required.
# Demo seeding (SEED_DEMO_DATA=true / DEMO_MODE=true) is handled inside the
# binary at startup; no docker-entrypoint.sh is needed.
CMD ["/app/parkhub-server", "--headless", "--unattended", "--data-dir", "/data", "--port", "10000"]
