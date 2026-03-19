#!/bin/bash
# Run Maestro E2E tests against ParkHub
# Usage: ./e2e/run.sh [base_url]
#   ./e2e/run.sh                              → http://localhost:8080
#   ./e2e/run.sh https://parkhub-rust-demo.onrender.com

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export BASE_URL="${1:-http://localhost:8080}"
MAESTRO="${HOME}/.maestro/bin/maestro"

echo "Running Maestro E2E tests against: $BASE_URL"
echo "================================================"

$MAESTRO test "$SCRIPT_DIR" --format junit --output "$SCRIPT_DIR/results.xml" 2>&1

echo ""
echo "Results saved to: $SCRIPT_DIR/results.xml"
