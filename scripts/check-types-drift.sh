#!/usr/bin/env bash
#
# check-types-drift.sh — local pre-push gate that mirrors the
# `types-drift` job in `.github/workflows/ci.yml`.
#
# Runs the gen-types ts_export integration test which emits TypeScript
# bindings into `parkhub-web/src/generated/types/`, then fails if the
# working tree differs from HEAD (i.e. Rust source drifted from the
# committed `.ts` bindings).
#
# Both diff and untracked-file checks are performed because adding a
# new `#[derive(TS)] #[ts(export)]` type creates a brand-new `.ts`
# file that `git diff` would silently miss.
#
# Local dev override:
#
#   SKIP_TYPES_DRIFT=1 git push

set -euo pipefail

if [[ "${SKIP_TYPES_DRIFT:-0}" == "1" ]]; then
    echo "types-drift: skipped (SKIP_TYPES_DRIFT=1)"
    exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

GENERATED_DIR="parkhub-web/src/generated"
if [[ ! -d "$GENERATED_DIR" ]]; then
    echo "types-drift: missing generated dir $GENERATED_DIR" >&2
    exit 1
fi

# rust_embed needs parkhub-web/dist/ to exist at compile time even for
# the test build, so seed a placeholder if absent.
mkdir -p parkhub-web/dist
[[ -f parkhub-web/dist/index.html ]] || \
    printf '%s' '<!doctype html><html><body></body></html>' \
    > parkhub-web/dist/index.html

echo "types-drift: regenerating ts-rs bindings..." >&2
# This invokes the explicit ts_export integration test (see
# parkhub-server/tests/ts_export.rs) which calls T::export_all_to(...)
# for every DTO and stamps a warning header on each emitted .ts file.
./scripts/fop-wrap.sh cargo test \
    --locked \
    --features gen-types \
    -p parkhub-server \
    --test ts_export \
    -- --nocapture

drift=0

# (1) Modified or deleted tracked files in the generated tree.
if ! git diff --exit-code -- "$GENERATED_DIR/"; then
    echo "::error:: $GENERATED_DIR has modified or deleted files (drift from Rust source)." >&2
    drift=1
fi

# (2) New types that produced untracked files — git diff misses these.
untracked="$(git status --porcelain -- "$GENERATED_DIR/")"
if [[ -n "$untracked" ]]; then
    echo "::error:: $GENERATED_DIR has untracked or modified files not yet committed:" >&2
    printf '%s\n' "$untracked" >&2
    drift=1
fi

if [[ "$drift" -ne 0 ]]; then
    cat >&2 <<EOF

Regenerate locally:

    ./scripts/fop-wrap.sh \\
        cargo test --features gen-types -p parkhub-server --test ts_export -- --nocapture

Then \`git add parkhub-web/src/generated/\` and commit the updated bindings.
EOF
    exit 1
fi

echo "types-drift: $GENERATED_DIR matches the Rust source."
