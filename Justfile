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
    @echo "Cargo version: $(awk -F'\"' '/^version[[:space:]]*=/ { print $2; exit }' Cargo.toml)"
    @echo "package.json: $(jq -r .version package.json)"
    @echo "parkhub-web: $(jq -r .version parkhub-web/package.json)"
    @echo "Last tag: $(git describe --tags --abbrev=0)"
    @echo ""
    @echo "Commits since last tag:"
    @git log --oneline "$(git describe --tags --abbrev=0)..HEAD" | head -20
    @echo ""
    @echo "Generated CHANGELOG section (dry-run, what cliff would write):"
    @git cliff --unreleased --strip header 2>/dev/null || echo "(install git-cliff to preview)"

[doc("regenerate CHANGELOG.md from git history (cliff.toml + conventional commits)")]
changelog:
    git cliff --output CHANGELOG.md
    @echo "✓ CHANGELOG.md regenerated. Review the diff before committing."

[doc("cut a new tag — bumps Cargo.toml + package.json + tags + pushes")]
release-tag VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! "{{VERSION}}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      echo "VERSION must be semver, e.g. 5.0.9 (got: {{VERSION}})" >&2
      exit 1
    fi
    if [[ -n "$(git status --porcelain)" ]]; then
      echo "working tree not clean — commit or stash first" >&2
      exit 1
    fi
    echo "▶ bumping versions to {{VERSION}}"
    sed -i 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml
    jq '.version = "{{VERSION}}"' package.json > package.json.tmp && mv package.json.tmp package.json
    jq '.version = "{{VERSION}}"' parkhub-web/package.json > parkhub-web/package.json.tmp && mv parkhub-web/package.json.tmp parkhub-web/package.json
    cargo update --workspace
    echo "▶ regenerating CHANGELOG"
    git cliff -t v{{VERSION}} --output CHANGELOG.md
    git add Cargo.toml Cargo.lock package.json parkhub-web/package.json CHANGELOG.md
    git commit -m "chore(release): v{{VERSION}}"
    git tag -a v{{VERSION}} -m "Release v{{VERSION}}"
    @echo "✓ tagged v{{VERSION}}. Review: git show v{{VERSION}}"
    @echo "  Push when ready: git push github main && git push github v{{VERSION}}"

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
[doc("remove cargo target + npm node_modules + parkhub-web/dist")]
clean:
    cargo clean
    rm -rf parkhub-web/node_modules parkhub-web/dist node_modules
