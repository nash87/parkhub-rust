#!/usr/bin/env bash
set -euo pipefail

python3 - <<'PY'
import pathlib
import re
import sys

try:
    import yaml
except ModuleNotFoundError:
    print("PyYAML is required for CI workflow policy checks", file=sys.stderr)
    raise

workflow_dir = pathlib.Path(".github/workflows")
ci_path = workflow_dir / "ci.yml"
ci = yaml.safe_load(ci_path.read_text())

errors = []

on = ci.get("on") or ci.get(True) or {}
merge_group = on.get("merge_group") if isinstance(on, dict) else None
merge_group_types = []
if isinstance(merge_group, dict):
    raw_types = merge_group.get("types") or []
    merge_group_types = raw_types if isinstance(raw_types, list) else [raw_types]
if "checks_requested" not in merge_group_types:
    errors.append("ci.yml must trigger on merge_group.types: [checks_requested]")

jobs = ci.get("jobs") or {}
required = jobs.get("required") or {}
if required.get("name") != "Required checks":
    errors.append("jobs.required.name must be 'Required checks'")
if required.get("if") != "always()":
    errors.append("jobs.required.if must be always()")

needs = required.get("needs") or []
if isinstance(needs, str):
    needs = [needs]
actual_needs = sorted(needs)

expected_needs = sorted(
    job_id
    for job_id, job in jobs.items()
    if job_id != "required" and not bool((job or {}).get("continue-on-error"))
)
if actual_needs != expected_needs:
    errors.append(f"jobs.required.needs drift expected={expected_needs} actual={actual_needs}")

required_steps = required.get("steps") or []
env_values = []
for step in required_steps:
    env = (step or {}).get("env") or {}
    env_values.extend(str(value) for value in env.values())
covered_needs = sorted(set(re.findall(r"needs\.([A-Za-z0-9_-]+)\.result", "\n".join(env_values))))
missing_env = [job_id for job_id in actual_needs if job_id not in covered_needs]
if missing_env:
    errors.append(f"jobs.required env missing needs result coverage for {missing_env}")

for job_id, job in sorted(jobs.items()):
    condition = str((job or {}).get("if") or "")
    if (
        "github.event.pull_request.head.repo.full_name != github.repository" in condition
        and "contains(github.event.pull_request.labels.*.name, 'github-ci-full')" in condition
        and "github.event.pull_request.user.type == 'Bot'" not in condition
    ):
        errors.append(
            f"jobs.{job_id}.if must run full GitHub CI for same-repo bot PRs "
            "instead of skipping both local attestation and heavy jobs"
        )

def walk_uses(node, path):
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "uses" and isinstance(value, str):
                yield value, path + ["uses"]
            else:
                yield from walk_uses(value, path + [str(key)])
    elif isinstance(node, list):
        for index, value in enumerate(node):
            yield from walk_uses(value, path + [str(index)])

for workflow in sorted(workflow_dir.glob("*.yml")) + sorted(workflow_dir.glob("*.yaml")):
    data = yaml.safe_load(workflow.read_text())
    for uses, path in walk_uses(data, [str(workflow)]):
        if uses.startswith("./") or uses.startswith("docker://"):
            continue
        if "@" not in uses:
            errors.append(f"{workflow}: unpinned action {uses}")
            continue
        ref = uses.rsplit("@", 1)[1]
        if not re.fullmatch(r"[0-9a-fA-F]{40}", ref):
            errors.append(f"{workflow}: action must be pinned to full commit SHA: {uses}")

if errors:
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    sys.exit(1)

print("CI workflow policy OK")
PY

scripts/tests/test-rustsec-ignore-policy.sh
