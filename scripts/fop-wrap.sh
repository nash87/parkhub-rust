#!/usr/bin/env bash
# fop-wrap.sh — wrap a command with `fop build --backend local ...` if `fop` is
# on PATH; otherwise run the command directly.
#
# Used by `lefthook.yml` and `Makefile` so local CI gates work on machines
# without `fop` installed (fresh clones, contributors who haven't installed
# the homelab toolchain). When `fop` is present, the wrapper enforces the
# memory cap + sccache locality + queue accounting we get in the Bazzite host
# environment. When it isn't, the bare command runs — we lose the cap but
# the gate still produces the correct pass/fail signal.
#
# Usage:
#   ./scripts/fop-wrap.sh <cmd> [args...]
#
# Examples:
#   ./scripts/fop-wrap.sh cargo fmt --all -- --check
#   ./scripts/fop-wrap.sh bash -lc 'cargo check && cargo clippy'
#
# Override the resource profile via FOP_RESOURCE_PROFILE (default
# `interactive-small`) when a heavier preset is needed for one-off runs.

set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <cmd> [args...]" >&2
  exit 2
fi

profile="${FOP_RESOURCE_PROFILE:-interactive-small}"

if command -v fop >/dev/null 2>&1; then
  exec fop build --backend local --resource-profile "$profile" . --preset custom -- "$@"
else
  exec "$@"
fi
