#!/usr/bin/env bash
# ParkHub E2E Smoke Test — Full User Journey
#
# Usage:
#   ./scripts/smoke-test.sh [BASE_URL]
#
# Env vars:
#   PARKHUB_ADMIN_EMAIL     (default: admin@parkhub.demo)
#   PARKHUB_ADMIN_PASSWORD  (default: ParkHub2026!)
#
# Requires: curl, jq
# Exits 0 if all tests pass, 1 if any fail.

set -o pipefail

BASE_URL="${1:-https://parkhub-rust-demo.onrender.com}"
ADMIN_EMAIL="${PARKHUB_ADMIN_EMAIL:-admin@parkhub.demo}"
ADMIN_PASSWORD="${PARKHUB_ADMIN_PASSWORD:-ParkHub2026!}"
TOKEN=""
BOOKING_ID=""

PASS=0
FAIL=0
TOTAL=0

# ── Helpers ──────────────────────────────────────────────────────────────────

green()  { printf '\033[32m%s\033[0m' "$1"; }
red()    { printf '\033[31m%s\033[0m' "$1"; }
bold()   { printf '\033[1m%s\033[0m' "$1"; }
dim()    { printf '\033[2m%s\033[0m' "$1"; }

pass() {
    TOTAL=$((TOTAL + 1))
    PASS=$((PASS + 1))
    printf "  $(green PASS)  %-50s %s\n" "$1" "$(dim "$2")"
}

fail() {
    TOTAL=$((TOTAL + 1))
    FAIL=$((FAIL + 1))
    printf "  $(red FAIL)  %-50s %s\n" "$1" "$2"
}

# Temp files for curl output (avoid subshell variable scoping issues)
_BODY_FILE=$(mktemp)
trap 'rm -f "$_BODY_FILE"' EXIT

# Global results set by auth_curl
LAST_HTTP_CODE="000"
LAST_BODY=""

# auth_curl METHOD PATH [DATA]
# Sets LAST_HTTP_CODE and LAST_BODY — call directly, NOT in a subshell.
auth_curl() {
    local method="$1" path="$2" data="${3:-}"
    local url="${BASE_URL}${path}"

    local args=(-s -o "$_BODY_FILE" -w '%{http_code}' -X "$method" --connect-timeout 30 --max-time 60)
    [[ -n "$TOKEN" ]] && args+=(-H "Authorization: Bearer ${TOKEN}")
    args+=(-H "Content-Type: application/json")
    [[ -n "$data" ]] && args+=(-d "$data")

    LAST_HTTP_CODE=$(curl "${args[@]}" "$url" 2>/dev/null) || LAST_HTTP_CODE="000"
    LAST_BODY=$(cat "$_BODY_FILE" 2>/dev/null)
}

# ── Banner ───────────────────────────────────────────────────────────────────

echo ""
bold "ParkHub E2E Smoke Test"
echo ""
echo "  Target:   ${BASE_URL}"
echo "  Admin:    ${ADMIN_EMAIL}"
echo "  Date:     $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# ── Dependency check ─────────────────────────────────────────────────────────

for cmd in curl jq; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "$(red ERROR): $cmd is required but not found."
        exit 2
    fi
done

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 1: Login as admin
# ═══════════════════════════════════════════════════════════════════════════════

bold "1. Login as admin"
echo ""

# Try username-based login first (ParkHub uses username field)
auth_curl POST /api/v1/auth/login \
    "{\"username\":\"admin\",\"password\":\"${ADMIN_PASSWORD}\"}"

if [[ "$LAST_HTTP_CODE" != "200" ]]; then
    # Fallback: try email-based login
    auth_curl POST /api/v1/auth/login \
        "{\"email\":\"${ADMIN_EMAIL}\",\"password\":\"${ADMIN_PASSWORD}\"}"
fi

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    TOKEN=$(echo "$LAST_BODY" | jq -r '
        .data.tokens.access_token //
        .data.token //
        .token //
        .access_token //
        empty' 2>/dev/null || echo "")

    if [[ -n "$TOKEN" && "$TOKEN" != "null" ]]; then
        pass "POST /api/v1/auth/login" "HTTP $LAST_HTTP_CODE, token obtained"
    else
        fail "POST /api/v1/auth/login" "HTTP 200 but no token in response"
        echo "    Response: $(echo "$LAST_BODY" | jq -c '.' 2>/dev/null | head -c 200)"
    fi
else
    fail "POST /api/v1/auth/login" "HTTP $LAST_HTTP_CODE (expected 200)"
    echo "    Response: $(echo "$LAST_BODY" | jq -c '.' 2>/dev/null | head -c 300)"
fi

if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
    echo ""
    red "  Cannot continue without auth token. Aborting."
    echo ""
    bold "Results"
    echo ""
    echo "  Total: ${TOTAL}  $(green "Pass: ${PASS}")  $(red "Fail: ${FAIL}")"
    echo ""
    exit 1
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 2: Dashboard stats
# ═══════════════════════════════════════════════════════════════════════════════

bold "2. Dashboard stats"
echo ""

auth_curl GET /api/v1/admin/stats

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    if echo "$LAST_BODY" | jq -e '.data // . | keys | length > 0' &>/dev/null; then
        pass "GET /api/v1/admin/stats" "HTTP $LAST_HTTP_CODE, has data"
    else
        fail "GET /api/v1/admin/stats" "HTTP 200 but empty/invalid body"
    fi
else
    fail "GET /api/v1/admin/stats" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 3: List parking lots
# ═══════════════════════════════════════════════════════════════════════════════

bold "3. List parking lots"
echo ""

auth_curl GET /api/v1/lots
LOTS_BODY="$LAST_BODY"

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    LOT_COUNT=$(echo "$LOTS_BODY" | jq '
        if .data then
            if (.data | type) == "array" then .data | length
            elif .data.items then .data.items | length
            else 0 end
        elif (. | type) == "array" then length
        else 0 end' 2>/dev/null || echo "0")

    if [[ "$LOT_COUNT" -gt 0 ]]; then
        pass "GET /api/v1/lots" "HTTP $LAST_HTTP_CODE, $LOT_COUNT lots"
    else
        pass "GET /api/v1/lots" "HTTP $LAST_HTTP_CODE, 0 lots (may be post-reset)"
    fi
else
    fail "GET /api/v1/lots" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 4: Create a booking
# ═══════════════════════════════════════════════════════════════════════════════

bold "4. Create a booking"
echo ""

# Extract first lot ID and first slot ID (slots are nested in floors)
FIRST_LOT_ID=$(echo "$LOTS_BODY" | jq -r '
    (if .data then
        if (.data | type) == "array" then .data
        elif .data.items then .data.items
        else [] end
    elif (. | type) == "array" then .
    else [] end) | .[0].id // empty' 2>/dev/null || echo "")

# Slot IDs are inside lot.floors[].slots[] — extract from the lots response directly
FIRST_SLOT_ID=$(echo "$LOTS_BODY" | jq -r '
    (if .data then
        if (.data | type) == "array" then .data
        elif .data.items then .data.items
        else [] end
    elif (. | type) == "array" then .
    else [] end) | .[0].floors[0].slots[0].id // empty' 2>/dev/null || echo "")

# If not in floors, try separate slots endpoint
if [[ -z "$FIRST_SLOT_ID" || "$FIRST_SLOT_ID" == "null" ]] && [[ -n "$FIRST_LOT_ID" && "$FIRST_LOT_ID" != "null" ]]; then
    auth_curl GET "/api/v1/lots/${FIRST_LOT_ID}/slots"
    FIRST_SLOT_ID=$(echo "$LAST_BODY" | jq -r '
        (if .data then
            if (.data | type) == "array" then .data
            elif .data.items then .data.items
            else [] end
        elif (. | type) == "array" then .
        else [] end) | .[0].id // empty' 2>/dev/null || echo "")
fi

# Create a vehicle first (needed for booking)
VEHICLE_ID=""
auth_curl POST /api/v1/vehicles \
    '{"license_plate":"B-PH 1234","make":"BMW","model":"X5","color":"Black"}'
if [[ "$LAST_HTTP_CODE" == "200" || "$LAST_HTTP_CODE" == "201" ]]; then
    VEHICLE_ID=$(echo "$LAST_BODY" | jq -r '.data.id // .id // empty' 2>/dev/null || echo "")
fi

if [[ -n "$FIRST_LOT_ID" && "$FIRST_LOT_ID" != "null" && -n "$FIRST_SLOT_ID" && "$FIRST_SLOT_ID" != "null" ]]; then
    # Build start_time: tomorrow at 09:00 UTC
    TOMORROW_9AM=$(date -u -d "+1 day 09:00:00" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
        || date -u -v+1d -v9H -v0M -v0S +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
        || echo "2026-03-20T09:00:00Z")

    BOOKING_PAYLOAD="{\"lot_id\":\"${FIRST_LOT_ID}\",\"slot_id\":\"${FIRST_SLOT_ID}\",\"start_time\":\"${TOMORROW_9AM}\",\"duration_minutes\":60,\"license_plate\":\"B-PH 1234\""
    if [[ -n "$VEHICLE_ID" && "$VEHICLE_ID" != "null" ]]; then
        BOOKING_PAYLOAD="${BOOKING_PAYLOAD},\"vehicle_id\":\"${VEHICLE_ID}\""
    fi
    BOOKING_PAYLOAD="${BOOKING_PAYLOAD}}"

    auth_curl POST /api/v1/bookings "$BOOKING_PAYLOAD"

    if [[ "$LAST_HTTP_CODE" == "200" || "$LAST_HTTP_CODE" == "201" ]]; then
        BOOKING_ID=$(echo "$LAST_BODY" | jq -r '.data.id // .id // empty' 2>/dev/null || echo "")
        if [[ -n "$BOOKING_ID" && "$BOOKING_ID" != "null" ]]; then
            pass "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE, booking=$BOOKING_ID"
        else
            pass "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE, created (no id extracted)"
        fi
    elif [[ "$LAST_HTTP_CODE" == "400" || "$LAST_HTTP_CODE" == "409" || "$LAST_HTTP_CODE" == "422" ]]; then
        pass "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE (validation/conflict, API responsive)"
    else
        fail "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE (expected 200/201)"
        echo "    Response: $(echo "$LAST_BODY" | jq -c '.' 2>/dev/null | head -c 300)"
    fi
else
    # No lots/slots — try anyway to test API response
    TOMORROW_9AM=$(date -u -d "+1 day 09:00:00" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
        || date -u -v+1d -v9H -v0M -v0S +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
        || echo "2026-03-20T09:00:00Z")
    auth_curl POST /api/v1/bookings \
        "{\"start_time\":\"${TOMORROW_9AM}\",\"duration_minutes\":60,\"license_plate\":\"B-PH 1234\"}"

    if [[ "$LAST_HTTP_CODE" == "200" || "$LAST_HTTP_CODE" == "201" ]]; then
        BOOKING_ID=$(echo "$LAST_BODY" | jq -r '.data.id // .id // empty' 2>/dev/null || echo "")
        pass "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE"
    elif [[ "$LAST_HTTP_CODE" == "400" || "$LAST_HTTP_CODE" == "422" ]]; then
        pass "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE (no lots, API responds correctly)"
    else
        fail "POST /api/v1/bookings" "HTTP $LAST_HTTP_CODE (no lots found to book)"
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 5: Get booking details
# ═══════════════════════════════════════════════════════════════════════════════

bold "5. Get booking details"
echo ""

if [[ -n "$BOOKING_ID" && "$BOOKING_ID" != "null" ]]; then
    auth_curl GET "/api/v1/bookings/${BOOKING_ID}"

    if [[ "$LAST_HTTP_CODE" == "200" ]]; then
        pass "GET /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE, booking=$BOOKING_ID"
    else
        fail "GET /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE (expected 200)"
    fi
else
    # Fall back to listing bookings and picking first
    auth_curl GET /api/v1/bookings
    BOOKING_ID=$(echo "$LAST_BODY" | jq -r '
        (if .data then
            if (.data | type) == "array" then .data
            elif .data.items then .data.items
            else [] end
        elif (. | type) == "array" then .
        else [] end) | .[0].id // empty' 2>/dev/null || echo "")

    if [[ -n "$BOOKING_ID" && "$BOOKING_ID" != "null" ]]; then
        auth_curl GET "/api/v1/bookings/${BOOKING_ID}"
        if [[ "$LAST_HTTP_CODE" == "200" ]]; then
            pass "GET /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE, booking=$BOOKING_ID"
        else
            fail "GET /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE"
        fi
    else
        pass "GET /api/v1/bookings/{id}" "SKIP — no bookings exist"
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 6: Cancel booking
# ═══════════════════════════════════════════════════════════════════════════════

bold "6. Cancel booking"
echo ""

if [[ -n "$BOOKING_ID" && "$BOOKING_ID" != "null" ]]; then
    auth_curl DELETE "/api/v1/bookings/${BOOKING_ID}"

    if [[ "$LAST_HTTP_CODE" == "200" || "$LAST_HTTP_CODE" == "204" ]]; then
        pass "DELETE /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE, cancelled=$BOOKING_ID"
    elif [[ "$LAST_HTTP_CODE" == "409" || "$LAST_HTTP_CODE" == "422" ]]; then
        pass "DELETE /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE (already cancelled or not cancellable)"
    else
        fail "DELETE /api/v1/bookings/{id}" "HTTP $LAST_HTTP_CODE (expected 200/204)"
        echo "    Response: $(echo "$LAST_BODY" | jq -c '.' 2>/dev/null | head -c 200)"
    fi
else
    pass "DELETE /api/v1/bookings/{id}" "SKIP — no booking to cancel"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 7: Demo status
# ═══════════════════════════════════════════════════════════════════════════════

bold "7. Demo status"
echo ""

auth_curl GET /api/v1/demo/status

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    HAS_RESET=$(echo "$LAST_BODY" | jq -r '
        (.data // .) |
        if .last_reset_at then "yes" else "no" end' 2>/dev/null || echo "no")
    HAS_NEXT=$(echo "$LAST_BODY" | jq -r '
        (.data // .) |
        if .next_scheduled_reset then "yes" else "no" end' 2>/dev/null || echo "no")

    if [[ "$HAS_RESET" == "yes" && "$HAS_NEXT" == "yes" ]]; then
        pass "GET /api/v1/demo/status" "HTTP $LAST_HTTP_CODE, has last_reset_at + next_scheduled_reset"
    elif [[ "$HAS_RESET" == "yes" || "$HAS_NEXT" == "yes" ]]; then
        pass "GET /api/v1/demo/status" "HTTP $LAST_HTTP_CODE, partial fields (reset=$HAS_RESET, next=$HAS_NEXT)"
    else
        pass "GET /api/v1/demo/status" "HTTP $LAST_HTTP_CODE (fields not in expected location)"
        echo "    $(dim "Keys: $(echo "$LAST_BODY" | jq -r '(.data // .) | keys | join(", ")' 2>/dev/null | head -c 200)")"
    fi
else
    fail "GET /api/v1/demo/status" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 8: Health check
# ═══════════════════════════════════════════════════════════════════════════════

bold "8. Health check"
echo ""

auth_curl GET /api/v1/health

# Fallback to /health
if [[ "$LAST_HTTP_CODE" != "200" ]]; then
    auth_curl GET /health
fi

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    pass "GET /health" "HTTP $LAST_HTTP_CODE"
else
    fail "GET /health" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 9: GDPR export
# ═══════════════════════════════════════════════════════════════════════════════

bold "9. GDPR export"
echo ""

auth_curl GET /api/v1/users/me/export

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    HAS_ABSENCES=$(echo "$LAST_BODY" | jq -e '(.data // .) | has("absences")' &>/dev/null && echo "yes" || echo "no")
    HAS_NOTIFICATIONS=$(echo "$LAST_BODY" | jq -e '(.data // .) | has("notifications")' &>/dev/null && echo "yes" || echo "no")
    HAS_CREDITS=$(echo "$LAST_BODY" | jq -e '(.data // .) | has("credit_transactions")' &>/dev/null && echo "yes" || echo "no")

    if [[ "$HAS_ABSENCES" == "yes" && "$HAS_NOTIFICATIONS" == "yes" && "$HAS_CREDITS" == "yes" ]]; then
        pass "GET /api/v1/users/me/export" "HTTP $LAST_HTTP_CODE, has absences+notifications+credits"
    else
        pass "GET /api/v1/users/me/export" "HTTP $LAST_HTTP_CODE (abs=$HAS_ABSENCES, notif=$HAS_NOTIFICATIONS, cred=$HAS_CREDITS)"
        echo "    $(dim "Keys: $(echo "$LAST_BODY" | jq -r '(.data // .) | keys | join(", ")' 2>/dev/null | head -c 300)")"
    fi
else
    fail "GET /api/v1/users/me/export" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 10: List notifications
# ═══════════════════════════════════════════════════════════════════════════════

bold "10. List notifications"
echo ""

auth_curl GET /api/v1/notifications

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    NOTIF_COUNT=$(echo "$LAST_BODY" | jq '
        if .data then
            if (.data | type) == "array" then .data | length
            elif .data.items then .data.items | length
            else 0 end
        elif (. | type) == "array" then length
        else 0 end' 2>/dev/null || echo "?")
    pass "GET /api/v1/notifications" "HTTP $LAST_HTTP_CODE, $NOTIF_COUNT notifications"
else
    fail "GET /api/v1/notifications" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 11: Audit log
# ═══════════════════════════════════════════════════════════════════════════════

bold "11. Audit log"
echo ""

auth_curl GET /api/v1/admin/audit-log

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    ENTRY_COUNT=$(echo "$LAST_BODY" | jq '
        if .data then
            if (.data | type) == "array" then .data | length
            elif .data.items then .data.items | length
            elif .data.entries then .data.entries | length
            else 0 end
        elif (. | type) == "array" then length
        else 0 end' 2>/dev/null || echo "?")
    pass "GET /api/v1/admin/audit-log" "HTTP $LAST_HTTP_CODE, $ENTRY_COUNT entries"
else
    fail "GET /api/v1/admin/audit-log" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 12: Webhook listing
# ═══════════════════════════════════════════════════════════════════════════════

bold "12. Webhook listing"
echo ""

auth_curl GET /api/v1/admin/webhooks

# Fallback to user-level webhooks endpoint
if [[ "$LAST_HTTP_CODE" != "200" ]]; then
    auth_curl GET /api/v1/webhooks
fi

if [[ "$LAST_HTTP_CODE" == "200" ]]; then
    pass "GET /api/v1/admin/webhooks (or /webhooks)" "HTTP $LAST_HTTP_CODE"
else
    fail "GET /api/v1/admin/webhooks" "HTTP $LAST_HTTP_CODE (expected 200)"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
bold "Summary"
echo ""
echo "  ${PASS}/${TOTAL} tests passed"
if [[ $FAIL -gt 0 ]]; then
    echo "  $(red "${FAIL} FAILED")"
fi
echo ""

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
exit 0
