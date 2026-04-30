# parkhub-rust task runner — `just` (MIT, casey/just).
#
# `just` lists all available recipes. Run any with `just <name>`.
#
# This file is the canonical entry point for local dev workflows. CI uses
# the same recipes where it makes sense (one-source-of-truth).

set shell := ["bash", "-uc"]
set dotenv-load := false

# ---------------------------------------------------------------------------
# Default — interactive list of recipes
# ---------------------------------------------------------------------------
default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# One-command bootstrap for fresh clones
# ---------------------------------------------------------------------------
[doc("install all dev tools (mise + lefthook + npm) — fresh-clone setup")]
bootstrap:
    @echo "▶ installing mise tools (rust, node, sccache, mold, gates...)"
    mise install
    @echo "▶ wiring lefthook git hooks"
    lefthook install --force
    @echo "▶ installing parkhub-web dependencies"
    cd parkhub-web && npm ci
    @echo "✓ ready — try: just dev"

# ---------------------------------------------------------------------------
# Day-to-day dev loop
# ---------------------------------------------------------------------------
[doc("live cargo check + clippy + test (TUI via bacon)")]
dev:
    bacon

[doc("astro dev server for parkhub-web")]
web-dev:
    cd parkhub-web && npm run dev

[doc("parkhub-server (headless) + parkhub-web together")]
serve:
    #!/usr/bin/env bash
    set -euo pipefail
    cd parkhub-web && npm run build &
    cargo run --release -p parkhub-server --no-default-features --features headless

# ---------------------------------------------------------------------------
# Local CI gates — mirror lefthook pre-push so you can replay before pushing
# ---------------------------------------------------------------------------
[doc("run all pre-push gates locally (the same lefthook fires on `git push`)")]
ci:
    lefthook run pre-push

[doc("just the fast subset: fmt + clippy + lib tests")]
check:
    cargo fmt --all -- --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --lib

[doc("run cargo check on the headless surfaces (matches CI)")]
cargo-check:
    mkdir -p parkhub-web/dist
    [ -f parkhub-web/dist/index.html ] || printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html
    cargo check --locked -p parkhub-common --all-targets
    cargo check --locked -p parkhub-server --no-default-features --features headless --all-targets
    cargo check --locked -p parkhub-client --all-targets

[doc("vitest (parkhub-web) — only changed files since main")]
test-web:
    cd parkhub-web && npx vitest --run --changed

# ---------------------------------------------------------------------------
# Formatting + linting — quick fixers
# ---------------------------------------------------------------------------
[doc("apply cargo fmt + dprint (md/json/yml/toml) + biome (parkhub-web)")]
fmt:
    cargo fmt --all
    dprint fmt
    cd parkhub-web && [ -d node_modules/@biomejs/biome ] && npx biome check --write src/ || true

[doc("run typos repo-wide (in-place auto-fix)")]
typos-fix:
    typos --write-changes

# ---------------------------------------------------------------------------
# Security — local-first scanners (mirrors scripts/local-security-audit.sh)
# ---------------------------------------------------------------------------
[doc("full security suite locally — same as CI Security workflow")]
security:
    bash scripts/local-security-audit.sh

[doc("just cargo-audit (RustSec advisories)")]
audit:
    cargo audit --quiet

[doc("just cargo-deny (advisories + bans + licenses + sources)")]
deny:
    cargo deny check

[doc("trivy filesystem scan, CRITICAL+HIGH only (matches CI)")]
trivy:
    trivy fs --quiet --exit-code 1 \
      --scanners=vuln,misconfig \
      --severity=CRITICAL,HIGH \
      --ignorefile .trivyignore \
      --skip-dirs=node_modules,target,parkhub-web/node_modules,.claude/worktrees \
      .

# ---------------------------------------------------------------------------
# Drift checkers
# ---------------------------------------------------------------------------
[doc("verify OpenAPI snapshot has not drifted (regen prints command on fail)")]
openapi-drift:
    bash scripts/check-openapi-drift.sh

[doc("verify ts-rs TypeScript bindings have not drifted")]
types-drift:
    bash scripts/check-types-drift.sh

# ---------------------------------------------------------------------------
# Release plumbing
# ---------------------------------------------------------------------------
[doc("preview the next release — version surfaces + changelog dry-run")]
release-preview:
    @echo "Cargo version: $(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"
    @echo "package.json: $(jq -r .version package.json)"
    @echo "parkhub-web: $(jq -r .version parkhub-web/package.json)"
    @echo "Last tag: $(git describe --tags --abbrev=0)"
    @echo "Commits since last tag:"
    @git log --oneline "$(git describe --tags --abbrev=0)..HEAD" | head -20

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
[doc("remove cargo target + npm node_modules + parkhub-web/dist")]
clean:
    cargo clean
    rm -rf parkhub-web/node_modules parkhub-web/dist node_modules
