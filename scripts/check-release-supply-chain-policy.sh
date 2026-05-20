#!/usr/bin/env bash
set -euo pipefail

python3 - <<'PY'
from pathlib import Path
import sys

try:
    import yaml
except ModuleNotFoundError:
    print("PyYAML is required for release supply-chain policy checks", file=sys.stderr)
    raise

repo = Path(".")
errors = []

forbidden = {
    "wolfi-base" + ":latest": "mutable Wolfi base image tag",
    "continue-on-error" + ": true": "non-blocking release or security gate",
    "advisory until " + "first signed": "advisory attestation verification",
}

skip_dirs = {
    ".git",
    ".fop",
    ".claude",
    "node_modules",
    "vendor",
    "target",
    "parkhub-web/node_modules",
}
text_exts = {
    ".dockerfile",
    ".env",
    ".json",
    ".md",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}
top_level_files = {
    "Containerfile",
    "Dockerfile",
    "Dockerfile.debian",
    "docker-compose.yml",
    "docker-compose.yaml",
    "docker-compose.test.yml",
    "fly.toml",
    "koyeb.yaml",
    "render.yaml",
}


def skipped(path: Path) -> bool:
    parts = path.parts
    for item in skip_dirs:
        item_parts = tuple(item.split("/"))
        if any(tuple(parts[index : index + len(item_parts)]) == item_parts for index in range(len(parts))):
            return True
    return False


def is_policy_surface(path: Path) -> bool:
    if skipped(path):
        return False
    if str(path) == "scripts/check-release-supply-chain-policy.sh":
        return False
    if path.name in top_level_files:
        return True
    if path.name.lower().startswith(("dockerfile", "containerfile")):
        return True
    if path.suffix.lower() in text_exts:
        return True
    return False


def read_text(path: Path) -> str:
    try:
        return path.read_text(errors="replace")
    except OSError as exc:
        errors.append(f"{path}: cannot read policy surface: {exc}")
        return ""


for path in sorted(p for p in repo.rglob("*") if p.is_file() and is_policy_surface(p)):
    text = read_text(path)
    for pattern, description in forbidden.items():
        if pattern in text:
            errors.append(f"{path}: contains {description}: {pattern}")

workflow = Path(".github/workflows/docker-publish.yml")
if not workflow.is_file():
    errors.append(".github/workflows/docker-publish.yml is required")
else:
    text = read_text(workflow)
    required_snippets = {
        "id-token: write": "docker publish must grant OIDC id-token for keyless signing",
        "attestations: write": "docker publish must grant attestations write permission",
        "provenance: mode=max": "docker publish must request max provenance",
        "sbom: true": "docker publish must request SBOM generation",
        "attest-build-provenance@": "docker publish must attest build provenance",
        "cosign sign --yes": "docker publish must cosign the immutable image digest",
    }
    for snippet, description in required_snippets.items():
        if snippet not in text:
            errors.append(f"{workflow}: {description}")

verify = Path(".github/workflows/cosign-verify.yml")
if not verify.is_file():
    errors.append(".github/workflows/cosign-verify.yml is required")
else:
    text = read_text(verify)
    for snippet in ("cosign verify", "verify-attestation", "--type spdxjson"):
        if snippet not in text:
            errors.append(f"{verify}: missing {snippet} verification")

try:
    for workflow_path in sorted(Path(".github/workflows").glob("*.yml")) + sorted(Path(".github/workflows").glob("*.yaml")):
        yaml.safe_load(workflow_path.read_text())
    for workflow_path in sorted(Path(".gitea/workflows").glob("*.yml")) + sorted(Path(".gitea/workflows").glob("*.yaml")):
        yaml.safe_load(workflow_path.read_text())
except yaml.YAMLError as exc:
    errors.append(f"workflow YAML parse failed: {exc}")

if errors:
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    sys.exit(1)

print("Release supply-chain policy OK")
PY
