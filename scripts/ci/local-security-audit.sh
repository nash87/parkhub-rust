#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/ci/local-security-audit.sh [--profile pr|cd] [--strict-tools] [--fail-advisory]

Runs the local OSS filesystem-and-source security mirror for security.yml.
Default mode mirrors GitHub PR behavior for installed tools: enforced gates
fail, advisory gates report findings without failing the run, and missing
local OSS tools are reported and skipped. Use --strict-tools before a release
to require the full local toolchain to be installed.

Coverage parity with security.yml: this script runs filesystem-and-source
gates only (cargo deny/audit/geiger, npm audit, gitleaks, zizmor, typos,
osv-scanner, actionlint, helm lint, docker compose config, trivy fs). It does
NOT run security.yml's image-scan jobs (trivy-image, grype-image) because
they require a built container image — those run in CI after the build job
publishes a tagged image to ghcr. Run \`docker build\` + \`trivy image\` /
\`grype\` manually before pushing release tags if you want pre-push parity.

All gates use commercial-license-safe OSS (MIT/Apache-2.0/BSD/ISC). No CodeQL,
no Semgrep (LGPL), no SaaS scanners. Mirrors parkhub-php's UX exactly so a
single Florian-level mental model spans both repos.

Profiles:
  pr  cargo deny (advisories+licenses+bans+sources), cargo audit, npm audits
      (root + parkhub-web), gitleaks, zizmor, typos, and workflow/manifest
      hygiene when local tools are present.
  cd  pr + Trivy filesystem scan for HIGH/CRITICAL vuln/misconfig findings.

Environment:
  FOP_SECURITY_BASE_REF  Base ref for PR-style gitleaks range (default:
                         github/main, falling back to origin/main).
  ZIZMOR_ARGS            Extra zizmor args (default: --persona=auditor).
EOF
}

profile="pr"
strict_tools=0
fail_advisory=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      profile="${2:?missing profile}"
      shift 2
      ;;
    --strict-tools)
      strict_tools=1
      shift
      ;;
    --fail-advisory)
      fail_advisory=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$profile" in
  pr|cd) ;;
  *)
    echo "invalid profile: $profile" >&2
    exit 2
    ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

missing_tools=()
advisory_failures=()

tool_path() {
  command -v "$1" 2>/dev/null || true
}

require_core_tool() {
  local tool="$1"
  if [[ -z "$(tool_path "$tool")" ]]; then
    echo "required tool missing: $tool" >&2
    exit 1
  fi
}

optional_tool() {
  local tool="$1"
  if [[ -n "$(tool_path "$tool")" ]]; then
    return 0
  fi
  missing_tools+=("$tool")
  return 1
}

section() {
  printf '\n==> %s\n' "$1"
}

run_required() {
  local name="$1"
  shift
  section "$name"
  "$@"
}

run_advisory() {
  local name="$1"
  shift
  section "$name (advisory)"
  if "$@"; then
    return 0
  fi
  advisory_failures+=("$name")
  echo "$name returned non-zero (advisory in GitHub mode)"
  if [[ "$fail_advisory" -eq 1 ]]; then
    return 1
  fi
  return 0
}

run_if_available() {
  local tool="$1"
  local name="$2"
  shift 2
  if optional_tool "$tool"; then
    run_required "$name" "$@"
  else
    section "$name"
    echo "$tool not installed; skipping"
  fi
}

run_advisory_if_available() {
  local tool="$1"
  local name="$2"
  shift 2
  if optional_tool "$tool"; then
    run_advisory "$name" "$@"
  else
    section "$name (advisory)"
    echo "$tool not installed; skipping"
  fi
}

require_core_tool git
require_core_tool cargo
require_core_tool npm

section "local security profile"
echo "profile=$profile strict_tools=$strict_tools fail_advisory=$fail_advisory"

# ─── Required gates: cargo deny mirrors security.yml cargo-deny job ──────────
# cargo-deny is dual MIT/Apache-2.0 (github.com/EmbarkStudios/cargo-deny).
run_if_available cargo-deny "cargo deny (advisories+bans+licenses+sources)" \
  cargo deny check advisories bans licenses sources

# ─── cargo audit mirrors security.yml cargo-audit job ────────────────────────
# cargo-audit is dual MIT/Apache-2.0 (github.com/RustSec/rustsec).
# Ignore list MUST stay in sync with .github/workflows/security.yml; if you
# update one, update the other in the same commit.
if optional_tool cargo-audit; then
  section "cargo audit"
  cargo audit \
    --ignore RUSTSEC-2024-0412 \
    --ignore RUSTSEC-2024-0413 \
    --ignore RUSTSEC-2024-0415 \
    --ignore RUSTSEC-2024-0416 \
    --ignore RUSTSEC-2024-0418 \
    --ignore RUSTSEC-2024-0419 \
    --ignore RUSTSEC-2024-0420 \
    --ignore RUSTSEC-2024-0436 \
    --ignore RUSTSEC-2024-0370 \
    --ignore RUSTSEC-2023-0071 \
    --ignore RUSTSEC-2025-0057 \
    --ignore RUSTSEC-2023-0019 \
    --ignore RUSTSEC-2024-0384 \
    --ignore RUSTSEC-2026-0097
else
  section "cargo audit"
  echo "cargo-audit not installed; skipping (install: cargo install cargo-audit)"
fi

# ─── npm audits ──────────────────────────────────────────────────────────────
# npm CLI is Artistic-2.0 but `npm audit` is bundled and a sub-command of the
# stock npm distribution; no separate license obligation. Mirrors parkhub-php
# (root + workspace audits) — root audit is skipped if no root package-lock.
if [[ -f package-lock.json ]]; then
  run_advisory "npm audit root (prod high)" npm audit --package-lock-only --omit=dev --audit-level=high
else
  section "npm audit root"
  echo "no root package-lock.json; skipping"
fi
run_advisory "npm audit parkhub-web (prod high)" npm audit --prefix parkhub-web --package-lock-only --omit=dev --audit-level=high

# ─── secret scan (gitleaks; MIT) ─────────────────────────────────────────────
# Mirrors security.yml secret-scan job — direct binary, not the proprietary
# gitleaks-action wrapper. PR-scoped diff against base ref when available.
if optional_tool gitleaks; then
  section "secret scan (gitleaks)"
  base_ref="${FOP_SECURITY_BASE_REF:-github/main}"
  if ! git rev-parse --verify --quiet "$base_ref" >/dev/null; then
    base_ref="origin/main"
  fi
  if git rev-parse --verify --quiet "$base_ref" >/dev/null; then
    base_sha="$(git merge-base HEAD "$base_ref")"
    gitleaks detect --source=. --redact --verbose --no-banner --log-opts="--no-merges ${base_sha}..HEAD"
  else
    echo "no base ref available; scanning all reachable history"
    gitleaks detect --source=. --redact --verbose --no-banner
  fi
else
  section "secret scan (gitleaks)"
  echo "gitleaks not installed; skipping"
fi

# ─── workflow/manifest hygiene (advisory-style local helpers) ────────────────
# actionlint (MIT) is the primary workflow linter. We deliberately skip
# yamllint (GPL-3.0) to keep the local gate strictly within Florian's
# commercial-license-safe set (MIT/Apache-2.0/BSD/ISC).
run_if_available actionlint "workflow lint (actionlint)" actionlint .github/workflows
run_if_available helm "helm chart render" bash -euo pipefail -c "if [[ -d helm ]]; then for chart in helm/*/Chart.yaml; do [[ -f \"\$chart\" ]] || continue; chartdir=\"\$(dirname \"\$chart\")\"; helm lint \"\$chartdir\" && helm template parkhub \"\$chartdir\" >/dev/null; done; else echo 'no helm/ directory; skipping'; fi"
run_if_available docker "docker compose config" bash -euo pipefail -c "if [[ -f docker-compose.yml ]]; then docker compose -f docker-compose.yml config -q; else echo 'no docker-compose.yml; skipping'; fi"

# ─── zizmor (MIT, GHA SAST in audit-mode) ───────────────────────────────────
# Mirrors security.yml zizmor job. Advisory-only until the open-finding
# inventory hits zero; then promote to required.
zizmor_args=()
if [[ -n "${ZIZMOR_ARGS:-}" ]]; then
  # shellcheck disable=SC2206
  zizmor_args=(${ZIZMOR_ARGS})
else
  zizmor_args=(--persona=auditor)
fi
run_advisory_if_available zizmor "zizmor (gha sast audit-mode)" zizmor "${zizmor_args[@]}" .github/workflows

# ─── typos (MIT, advisory) ───────────────────────────────────────────────────
run_advisory_if_available typos "typos" typos .

# ─── cargo-geiger (Apache-2.0 OR MIT, unsafe-block SAST) ────────────────────
# cargo-geiger counts unsafe blocks in the entire dep graph, surfacing
# supply-chain unsafe-usage growth before it lands in production. Advisory:
# unsafe is unavoidable in low-level crates (memmap2, axum's deps, etc.) and
# we don't want to fail on legitimate usage — we want to track the trend and
# review additions during dep bumps.
# Source: github.com/geiger-rs/cargo-geiger (license: "Apache-2.0 OR MIT").
run_advisory_if_available cargo-geiger "cargo-geiger (unsafe-block SAST)" \
  cargo geiger --output-format Ascii --frozen

# ─── osv-scanner (Apache-2.0, multi-ecosystem SCA) ──────────────────────────
# Google OSV-Scanner cross-references Cargo.lock + parkhub-web/package-lock.json
# against the OSV.dev database. Defense-in-depth complement to cargo-audit
# (which only checks RustSec advisories) and npm audit (which only checks the
# npm advisory feed) — OSV.dev aggregates GHSA, RustSec, npm, GHSA, Go,
# debian, alpine, etc. into a single feed.
#
# Why osv-scanner instead of Bearer/Semgrep for "SAST coverage": Bearer is
# Elastic License 2.0 (source-available, banned per platform commercial-safe
# doctrine alongside BSL/SSPL/FSL). Semgrep is LGPL-2.1 (also banned).
# Rudra (Apache-2.0 OR MIT) is archived 2026-04-02, requires a frozen
# nightly-2021-10-21 toolchain, and only analyses crates that compile with
# that pinned compiler — not viable on a current Rust 1.94 codebase.
# Kani (Apache-2.0 OR MIT) is a bounded model checker and requires hand-written
# `#[kani::proof]` harnesses, so it is not a drop-in SAST gate.
# OSV-Scanner is the FOSS equivalent that actually runs against today's tree.
run_advisory_if_available osv-scanner "osv-scanner (multi-ecosystem SCA)" \
  osv-scanner scan source \
    -L Cargo.lock \
    -L parkhub-web/package-lock.json

# ─── cd profile: trivy filesystem scan (Apache-2.0) ─────────────────────────
# Mirrors security.yml trivy-fs job. Skips node_modules/target/vendor for
# parity with the GitHub job's effective scope.
if [[ "$profile" == "cd" ]]; then
  run_if_available trivy "trivy filesystem scan" trivy fs \
    --severity HIGH,CRITICAL --exit-code 1 \
    --skip-dirs target,node_modules,parkhub-web/node_modules,vendor \
    .
fi

if [[ "$strict_tools" -eq 1 && "${#missing_tools[@]}" -gt 0 ]]; then
  section "missing tools"
  printf '%s\n' "${missing_tools[@]}" | sort -u
  echo "install missing tools or rerun without --strict-tools" >&2
  exit 1
fi

if [[ "${#advisory_failures[@]}" -gt 0 ]]; then
  section "advisory failures"
  printf '%s\n' "${advisory_failures[@]}"
fi

section "local security audit passed"
