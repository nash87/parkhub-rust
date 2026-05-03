#!/usr/bin/env bash
# Local release rehearsal — mirrors .github/workflows/release-rehearsal.yml.
# Exercises the release path (validate-tag, headless+client builds,
# Syft SBOM, optional cosign sign-blob) WITHOUT pushing tags or creating
# a GitHub release. Use before pushing a `v*` tag so Tauri/cosign/Syft
# bugs surface locally instead of after a 30-min GHA round-trip.
#
# Usage:
#   scripts/local-release-rehearsal.sh [--tag v5.0.9-rc1]
#
# Skips Tauri builds locally (require macOS/Windows runners). Server +
# client headless builds run on this Linux box.
#
# Requires: cargo, syft. Optional: cosign + FOP_LOCAL_REL_SIGN_KEY for
# the signing rehearsal (otherwise SBOM gen runs unsigned).

set -euo pipefail

tag="v0.0.0-rc.local"
for arg in "$@"; do
  case "$arg" in
    --tag) shift; tag="$1" ;;
    --tag=*) tag="${arg#--tag=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

# ── Stage 1: validate-tag ──────────────────────────────────────────────
# Mirrors the synthetic-ref check in release.yml — verifies the tag name
# parses, doesn't already exist as a real release, and is on the expected
# format pattern.
echo "▶ validate-tag: $tag"
if ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-(alpha|beta|rc)\.[0-9]+|-rc\.local|-rc[0-9]+)?$ ]]; then
  echo "✗ tag '$tag' does not match v<MAJOR>.<MINOR>.<PATCH>[-prerelease]" >&2
  exit 1
fi
if git tag -l | grep -qx "$tag"; then
  echo "⚠ tag '$tag' already exists locally (rehearsal continues; real release.yml would FAIL)"
fi

# ── Stage 2: server headless build ─────────────────────────────────────
echo "▶ cargo build --release -p parkhub-server --no-default-features --features headless"
cargo build --locked --release -p parkhub-server --no-default-features --features headless

# ── Stage 3: client build ──────────────────────────────────────────────
if [[ -d parkhub-client ]]; then
  echo "▶ cargo build --release -p parkhub-client"
  cargo build --locked --release -p parkhub-client
else
  echo "⊘ parkhub-client/ missing — skipping client build"
fi

target_dir=$(cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory)

# ── Stage 4: SBOM via syft ────────────────────────────────────────────
out_dir=".fop/reports/release-rehearsal-$tag"
mkdir -p "$out_dir"
if command -v syft >/dev/null 2>&1; then
  echo "▶ syft scan release artifacts → $out_dir/"
  for bin in "$target_dir/release/parkhub-server" "$target_dir/release/parkhub-client"; do
    [[ -f "$bin" ]] || continue
    name=$(basename "$bin")
    syft scan "file:$bin" -o "spdx-json=$out_dir/sbom-$name.json" --quiet
    echo "  ✓ $out_dir/sbom-$name.json"
  done
else
  echo "⊘ syft not on PATH — skipping SBOM"
fi

# ── Stage 5: optional cosign sign-blob ────────────────────────────────
if [[ -n "${FOP_LOCAL_REL_SIGN_KEY:-}" ]] && command -v cosign >/dev/null 2>&1; then
  echo "▶ cosign sign-blob (key: $FOP_LOCAL_REL_SIGN_KEY)"
  for bin in "$target_dir/release/parkhub-server" "$target_dir/release/parkhub-client"; do
    [[ -f "$bin" ]] || continue
    cosign sign-blob \
      --key "$FOP_LOCAL_REL_SIGN_KEY" \
      --output-signature "$bin.sig" \
      --output-certificate "$bin.cert" \
      --yes \
      "$bin"
    echo "  ✓ $bin.{sig,cert}"
  done
else
  echo "⊘ cosign sign-blob skipped (set FOP_LOCAL_REL_SIGN_KEY + install cosign)"
fi

echo "✓ release rehearsal complete for $tag"
echo "  artifacts: $out_dir/"
echo "  binaries:  $target_dir/release/{parkhub-server,parkhub-client}"
echo "  next:      git tag -a $tag && git push github $tag"
