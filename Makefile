# parkhub-rust — Local CI/CD mirror
#
# These targets mirror the reproducible local subset of .github/workflows/*.yml.
# NEVER let them drift from the workflow jobs they claim to mirror: if a
# workflow job changes, update the matching make target in the same commit.
# The GitHub workflows remain the source of truth; use `make ci` for the fast
# local core gate and `make act` when you need to execute the actual YAML.
#
# Usage:
#   make ci         # core local gate: fmt + clippy + check + client-check + test + frontend + drift
#   make lint       # fmt --check + clippy (fmt + clippy jobs)
#   make test       # cargo test headless (test job)
#   make drift      # regenerate openapi + fail on diff — mirrors openapi-drift.yml
#   make act        # run the actual .github/workflows locally via nektos/act
#   make pre-push   # alias for ci; run before `git push origin/github`
#
# Requires: rust 1.94.1 (rust-toolchain.toml), node 22, npm. `act` is optional.

SHELL := bash
.SHELLFLAGS := -euo pipefail -c
MAKEFLAGS += --no-print-directory

EMBED_PLACEHOLDER := parkhub-web/dist/index.html
SERVER_FEATURES   := --no-default-features --features headless

.PHONY: help ci fmt clippy check client-check test lint drift frontend integration embed act pre-push clean

help:
	@echo "parkhub-rust local CI mirror (see .github/workflows/*.yml)"
	@echo ""
	@echo "  make ci          — fmt + clippy + check + client-check + test + frontend + drift"
	@echo "  make lint        — fmt --check + clippy (mirrors fmt + clippy jobs)"
	@echo "  make check       — cargo check headless"
	@echo "  make client-check — cargo check parkhub-client (requires cmake + fontconfig dev libs)"
	@echo "  make test        — cargo test headless"
	@echo "  make integration — integration tests (-- integration --test-threads=1)"
	@echo "  make frontend    — parkhub-web vitest + build"
	@echo "  make drift       — openapi snapshot drift check"
	@echo "  make act         — run workflows via nektos/act (if installed)"
	@echo "  make pre-push    — alias for ci; run before git push"

## rust_embed needs parkhub-web/dist/ to exist at compile time
embed:
	@mkdir -p parkhub-web/dist
	@[ -f $(EMBED_PLACEHOLDER) ] || printf '%s' '<!doctype html><html><body></body></html>' > $(EMBED_PLACEHOLDER)

## Mirrors: fmt job
fmt:
	cargo fmt --all -- --check

## Mirrors: clippy job
clippy: embed
	cargo clippy --locked --package parkhub-common --all-targets -- -D warnings
	cargo clippy --locked --package parkhub-server $(SERVER_FEATURES) --all-targets -- \
		-D warnings -A clippy::cognitive_complexity -A clippy::assigning_clones

lint: fmt clippy

## Mirrors: check job
check: embed
	cargo check --locked --package parkhub-common --all-targets
	cargo check --locked --package parkhub-server $(SERVER_FEATURES) --all-targets

## Mirrors: client-check job
client-check:
	cargo check --locked --package parkhub-client --all-targets

## Mirrors: test job
test: embed
	cargo test --locked --package parkhub-common --all-targets
	cargo test --locked --package parkhub-server $(SERVER_FEATURES) --all-targets

## Mirrors: integration job
integration: embed
	cargo build --locked --package parkhub-server $(SERVER_FEATURES)
	RUST_LOG=warn cargo test --locked --package parkhub-server $(SERVER_FEATURES) -- integration --test-threads=1

## Mirrors: frontend job
frontend:
	cd parkhub-web && npm ci && npm test && npm run build

## Mirrors: openapi-drift.yml (starts headless server on :18181, dumps, diffs)
drift: embed
	cargo build --locked --release --package parkhub-server --no-default-features --features 'full,headless'
	@mkdir -p /tmp/parkhub-drift-db
	@./target/release/parkhub-server --headless --unattended --port 18181 --data-dir /tmp/parkhub-drift-db > /tmp/parkhub-drift.log 2>&1 & \
		SERVER_PID=$$!; \
		for _ in $$(seq 1 45); do curl -sf http://localhost:18181/health >/dev/null 2>&1 && break; sleep 1; done; \
		./scripts/dump-openapi.sh 18181; RC=$$?; \
		kill $$SERVER_PID 2>/dev/null || true; \
		exit $$RC
	@if ! git diff --exit-code docs/openapi/rust.json; then \
		echo "ERROR: docs/openapi/rust.json drifted — commit the regenerated snapshot."; \
		exit 1; \
	fi
	@echo "OpenAPI snapshot in sync."

## Core local CI — fast, reproducible subset of the blocking GitHub checks
ci: fmt clippy check client-check test frontend drift
	@echo ""
	@echo "Core local gate passed. Run 'make act' for the full workflow YAML."

pre-push: ci

## Run the real workflows locally with nektos/act
act:
	@if ! command -v act >/dev/null 2>&1; then \
		echo "act not installed. Install:"; \
		echo "  brew install act                                 # macOS/Linuxbrew"; \
		echo "  curl -fsSL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash"; \
		echo "See DEVELOPMENT.md for .actrc conventions."; \
		exit 1; \
	fi
	act -W .github/workflows/ci.yml

clean:
	cargo clean
	rm -rf node_modules parkhub-web/node_modules parkhub-web/dist
