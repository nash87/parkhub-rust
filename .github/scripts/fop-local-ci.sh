#!/usr/bin/env bash
# Deprecation shim — delegates to nido-local-ci.sh.
#
# fop-local-ci.sh is the legacy name for the local-CI orchestrator.
# It has been renamed to nido-local-ci.sh as part of the nido-first
# tooling migration (T-nido-ci-migration). This shim preserves backward
# compatibility for any scripts, documentation, or muscle memory that
# still references the old name.
#
# All FOP_LOCAL_CI_* environment variables are accepted by nido-local-ci.sh
# via its compat alias layer. No behaviour change.
#
# TODO(T-7009): remove this shim once all callers have been updated to
# reference nido-local-ci.sh directly.
exec "$(dirname "$0")/nido-local-ci.sh" "$@"
