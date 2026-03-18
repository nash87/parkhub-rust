#!/usr/bin/env bash
# ParkHub E2E Smoke Test
#
# Usage:
#   ./scripts/smoke-test.sh [BASE_URL]
#
# Env vars:
#   PARKHUB_ADMIN_PASSWORD  (default: ParkHub2026!)
#
# Exits 0 if all tests pass, 1 if any fail.

set -euo pipefail

BASE_URL="${1:-http://localhost:10000}"
ADMIN_PASSWORD="${PARKHUB_ADMIN_PASSWORD:-ParkHub2026!}"
TOKEN=""

PASS=0
FAIL=0
TOTAL=0

# ── Helpers ──────────────────────────────────────────────────────────────────

green()  { printf '\033[32m%s\033[0m' "$1"; }
red()    { printf '\033[31m%s\033[0m' "$1"; }
bold()   { printf '\033[1m%s\033[0m' "$1"; }

# check_endpoint METHOD PATH EXPECTED_CODES...
# EXPECTED_CODES is a space-separated list of acceptable HTTP status codes.
check_endpoint() {
    local method="$1" path="$2"
    shift 2
    local expected=("$@")

    local url="${BASE_URL}${path}"
    local code

    TOTAL=$((TOTAL + 1))

    if [[ "$method" == "GET" ]]; then
        code=$(curl -s -o /dev/null -w "%{http_code}" \
            -H "Authorization: Bearer ${TOKEN}" \
            "$url" 2>/dev/null || echo "000")
    elif [[ "$method" == "POST" ]]; then
        code=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            "$url" 2>/dev/null || echo "000")
    else
        code=$(curl -s -o /dev/null -w "%{http_code}" \
            -X "$method" \
            -H "Authorization: Bearer ${TOKEN}" \
            -H "Content-Type: application/json" \
            "$url" 2>/dev/null || echo "000")
    fi

    local match=false
    for exp in "${expected[@]}"; do
        if [[ "$code" == "$exp" ]]; then
            match=true
            break
        fi
    done

    if $match; then
        PASS=$((PASS + 1))
        printf "  $(green PASS)  %-6s %-45s %s\n" "$method" "$path" "$code"
    else
        FAIL=$((FAIL + 1))
        printf "  $(red FAIL)  %-6s %-45s %s (expected: %s)\n" "$method" "$path" "$code" "${expected[*]}"
    fi
}

# ── Banner ───────────────────────────────────────────────────────────────────

echo ""
bold "ParkHub Smoke Test"
echo ""
echo "  Target:   ${BASE_URL}"
echo "  Admin:    admin / ****"
echo ""

# ── 1. Public endpoints ─────────────────────────────────────────────────────

bold "Public endpoints"
echo ""

check_endpoint GET /health                    200
check_endpoint GET /api/v1/setup/status       200
check_endpoint GET /api/v1/public/occupancy   200
check_endpoint GET /api/v1/public/display     200
check_endpoint GET /api/v1/demo/config        200
check_endpoint GET /api/v1/push/vapid-key     200 404

echo ""

# ── 2. Auth (login) ─────────────────────────────────────────────────────────

bold "Authentication"
echo ""

TOTAL=$((TOTAL + 1))
LOGIN_RESPONSE=$(curl -s -w "\n%{http_code}" \
    -X POST "${BASE_URL}/api/v1/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"admin\",\"password\":\"${ADMIN_PASSWORD}\"}" 2>/dev/null || echo -e "\n000")

LOGIN_CODE=$(echo "$LOGIN_RESPONSE" | tail -1)
LOGIN_BODY=$(echo "$LOGIN_RESPONSE" | sed '$d')

if [[ "$LOGIN_CODE" == "200" ]]; then
    PASS=$((PASS + 1))
    printf "  $(green PASS)  %-6s %-45s %s\n" "POST" "/api/v1/auth/login" "$LOGIN_CODE"

    # Extract token — try .data.tokens.access_token then .data.token
    TOKEN=$(echo "$LOGIN_BODY" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    t = d.get('data',{}).get('tokens',{}).get('access_token','')
    if not t:
        t = d.get('data',{}).get('token','')
    print(t)
except:
    print('')
" 2>/dev/null || echo "")

    if [[ -z "$TOKEN" ]]; then
        echo "  $(red WARN)  Could not extract token from login response"
        echo "  Skipping authenticated endpoints."
        echo ""
        bold "Results"
        echo ""
        echo "  Total: ${TOTAL}  $(green "Pass: ${PASS}")  $(red "Fail: ${FAIL}")"
        exit 1
    fi
else
    FAIL=$((FAIL + 1))
    printf "  $(red FAIL)  %-6s %-45s %s (expected: 200)\n" "POST" "/api/v1/auth/login" "$LOGIN_CODE"
    echo ""
    echo "  $(red "Cannot continue without auth token. Aborting.")"
    echo ""
    bold "Results"
    echo ""
    echo "  Total: ${TOTAL}  $(green "Pass: ${PASS}")  $(red "Fail: ${FAIL}")"
    exit 1
fi

echo ""

# ── 3. Authenticated user endpoints ─────────────────────────────────────────

bold "Authenticated (user) endpoints"
echo ""

check_endpoint GET /api/v1/lots                   200
check_endpoint GET /api/v1/bookings               200
check_endpoint GET /api/v1/vehicles               200
check_endpoint GET /api/v1/vehicles/city-codes     200
check_endpoint GET /api/v1/user/credits            200
check_endpoint GET /api/v1/user/stats              200
check_endpoint GET /api/v1/user/preferences        200
check_endpoint GET /api/v1/user/favorites          200
check_endpoint GET /api/v1/notifications           200
check_endpoint GET /api/v1/absences                200
check_endpoint GET /api/v1/waitlist                200
check_endpoint GET /api/v1/team                    200
check_endpoint GET /api/v1/calendar/events         200
check_endpoint GET /api/v1/announcements/active    200
check_endpoint GET /api/v1/webhooks                200

echo ""

# ── 4. Admin endpoints ──────────────────────────────────────────────────────

bold "Admin endpoints"
echo ""

check_endpoint GET /api/v1/admin/stats                     200
check_endpoint GET /api/v1/admin/reports                    200
check_endpoint GET /api/v1/admin/heatmap                    200
check_endpoint GET /api/v1/admin/audit-log                  200
check_endpoint GET /api/v1/admin/settings                   200
check_endpoint GET /api/v1/admin/privacy                    200
check_endpoint GET /api/v1/admin/settings/auto-release      200
check_endpoint GET /api/v1/admin/settings/email             200
check_endpoint GET /api/v1/admin/dashboard/charts           200
check_endpoint GET /api/v1/admin/users/export-csv           200
check_endpoint GET /api/v1/admin/bookings/export-csv        200

echo ""

# ── Summary ──────────────────────────────────────────────────────────────────

bold "Results"
echo ""
echo "  Total: ${TOTAL}  $(green "Pass: ${PASS}")  $(red "Fail: ${FAIL}")"
echo ""

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
exit 0
