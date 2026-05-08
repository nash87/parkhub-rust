# =============================================================================
# ParkHub Rust — Optimized multi-stage Docker build
# Uses cargo-chef for dependency layer caching, fat LTO for smallest binary,
# and a distroless runtime for minimal attack surface (~20 MB image).
# =============================================================================

# Global build-args (declared before the first FROM so every stage can
# reference them). WOLFI_BASE defaults to the homelab LAN mirror to preserve
# the "never pull from Docker Hub" convention for local + gitea-runner
# builds. GitHub Actions cloud runners pass
#   --build-arg WOLFI_BASE=cgr.dev/chainguard/wolfi-base:latest
# to source the same Wolfi base from Chainguard's public registry.
ARG WOLFI_BASE=192.168.178.250:5000/wolfi-base:latest

# ---------------------------------------------------------------------------
# Stage 1: Frontend build (Astro/Vite)
# ---------------------------------------------------------------------------
FROM node:26-alpine@sha256:e71ac5e964b9201072425d59d2e876359efa25dc96bb1768cb73295728d6e4ea AS web-builder
WORKDIR /app
COPY parkhub-web/package*.json ./
RUN npm ci
COPY parkhub-web/ ./
RUN DOCKER=1 npm run build

# ---------------------------------------------------------------------------
# Stage 2: Cargo chef — prepare dependency recipe
# ---------------------------------------------------------------------------
FROM rust:1.95-slim@sha256:81099830a1e1d244607b9a7a30f3ff6ecadc52134a933b4635faba24f52840c9 AS chef
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
# Exclude desktop-only members from workspace (server container only needs
# parkhub-common + parkhub-server). parkhub-client ships Slint GUI + tray;
# parkhub-desktop ships the Tauri 2 shell. Both pull heavy system libs
# (webkit2gtk, skia) that aren't needed in a headless server image.
RUN sed -i -e '/"parkhub-client"/d' -e '/"parkhub-desktop"/d' Cargo.toml
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
RUN sed -i -e '/"parkhub-client"/d' -e '/"parkhub-desktop"/d' Cargo.toml
RUN cargo chef cook --profile release-container --recipe-path recipe.json \
    --package parkhub-server --no-default-features --features headless

# ---------------------------------------------------------------------------
# Stage 5: Build — compile the actual application
# ---------------------------------------------------------------------------
FROM deps AS builder
# Re-copy root manifest so [workspace.lints] is present (cargo-chef strips it)
COPY Cargo.toml Cargo.lock ./
RUN sed -i -e '/"parkhub-client"/d' -e '/"parkhub-desktop"/d' Cargo.toml
# Copy real source (deps are already compiled)
COPY parkhub-common/ ./parkhub-common/
COPY parkhub-server/ ./parkhub-server/
# Copy frontend build output
COPY --from=web-builder /app/dist ./parkhub-web/dist/
# Touch sources to invalidate fingerprints, then build
RUN touch parkhub-common/src/lib.rs parkhub-server/src/main.rs && \
    cargo build --profile release-container --package parkhub-server \
        --no-default-features --features headless && \
    strip /app/target/release-container/parkhub-server

# ---------------------------------------------------------------------------
# Stage 6: Data-directory scaffold
# distroless has no shell, so we create /data with the correct UID here and
# copy the empty directory tree into the final image.
# UID 65532 is the built-in "nonroot" user in gcr.io/distroless/cc-debian12.
# ---------------------------------------------------------------------------
FROM busybox:1.37.0@sha256:1487d0af5f52b4ba31c7e465126ee2123fe3f2305d638e7827681e7cf6c83d5e AS data-setup
RUN mkdir -p /data && chown 65532:65532 /data

# ---------------------------------------------------------------------------
# Stage 7: Runtime — Chainguard Wolfi (replaces gcr.io/distroless/cc-debian13)
#
# Wolfi tracks current upstream and is scanned daily by Chainguard, eliminating
# the libc6 / libssl-debian13 CVE chain that blocks lefthook image-scan on
# Debian-based distroless. Pulled from internal mirror (CLAUDE.md: never pull
# from Docker Hub or external registries during builds — 429 + supply-chain).
#
# Stage adds tini for clean SIGTERM propagation to the tokio runtime + the
# tokio-cron-scheduler subshell that distroless lacked entirely.
#
# Runtime apk set:
#   - ca-certificates + ca-certificates-bundle: TLS roots for reqwest/rustls + lettre/native-tls
#   - tini: PID 1 reaper for tokio runtime + scheduler
#   - openssl: lettre's tokio1-native-tls dynamically links libssl/libcrypto at runtime
#     (openssl-sys is `vendored` at BUILD time, but native-tls still dlopens the system libs)
#   - libgcc: Rust panic unwinding (_Unwind_* symbols)
#
# WOLFI_BASE is a global build-arg declared at the top of the file (see
# header comment). Cloud CI overrides it; local + gitea-runner builds use
# the LAN mirror default.
# ---------------------------------------------------------------------------
FROM ${WOLFI_BASE} AS runtime

# `apk update && apk upgrade --no-cache --available` is mandatory to bump glibc
# past CVE-2026-5450 — without `--available`, glibc 2.43-r6 sticks and grype
# flags the won't-fix-on-Debian advisory chain. See memory recipe pitfall #1.
RUN apk update && apk upgrade --no-cache --available \
    && apk add --no-cache \
        ca-certificates \
        ca-certificates-bundle \
        libgcc \
        openssl \
        tini \
    && rm -rf /var/cache/apk/*

# Wolfi base already ships nonroot:65532 baked in (verified via /etc/passwd:
# `nonroot:x:65532:65532:Account created by apko:/home/nonroot:/bin/sh`).
# distroless had the same UID; helm charts pin securityContext.runAsUser=65532.
# No addgroup/adduser needed — drop-in compat across both bases.

WORKDIR /app

# Copy binary
COPY --from=builder --chown=65532:65532 /app/target/release-container/parkhub-server /app/parkhub-server

# Copy pre-created /data directory (owned by nonroot UID 65532).
# --chown is required: COPY --from defaults to root:root regardless of
# the source stage's ownership, so the busybox chown above would be
# discarded without this flag.
COPY --from=data-setup --chown=65532:65532 /data /data

# Drop to non-root (recreated above to match prior distroless UID).
USER 65532:65532

# Environment
ENV RUST_LOG=info

# Optional: PARKHUB_REDIS_URL — consumed only when the server was compiled
# with `--features redis-revocation` (off by default, see T-1742). Points the
# shared JWT revocation list and refresh-token family map at an external Redis
# so logouts survive pod restarts and propagate across multi-replica deploys.
# When the feature is OFF, this variable is ignored. When the feature is ON
# and this variable is UNSET, the server panics at startup with a clear message.
# Example: PARKHUB_REDIS_URL=redis://redis.parkhub.svc:6379/0

EXPOSE 10000

# Health check — uses the binary's built-in --health-check mode so no shell
# or external tools (wget/curl) are needed.
HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=5 \
    CMD ["/app/parkhub-server", "--health-check", "--port", "10000"]

# tini reaps zombies + handles signals for tokio + scheduler. Distroless had
# no init at all; this is an upgrade for clean pod terminations.
ENTRYPOINT ["/sbin/tini", "--"]

# Direct binary invocation — no shell wrapper required.
# Demo seeding (SEED_DEMO_DATA=true / DEMO_MODE=true) is handled inside the
# binary at startup; no docker-entrypoint.sh is needed.
CMD ["/app/parkhub-server", "--headless", "--unattended", "--data-dir", "/data", "--port", "10000"]
