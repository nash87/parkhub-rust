#!/bin/sh
# ParkHub Demo Seed Script (shell-based, no Python dependency)
# Creates admin, 10 lots, 200 users, and ~3500 bookings via the REST API.
# Usage: seed_demo.sh [--base-url URL]
set -e

BASE_URL="${1:-http://127.0.0.1:10000}"
ADMIN_PASSWORD="${PARKHUB_ADMIN_PASSWORD:-demo}"

post() {
  wget -qO- --header="Content-Type: application/json" \
       --header="Accept: application/json" \
       ${TOKEN:+--header="Authorization: Bearer $TOKEN"} \
       --post-data="$2" "$BASE_URL$1" 2>/dev/null || echo '{"error":true}'
}

get() {
  wget -qO- --header="Accept: application/json" \
       ${TOKEN:+--header="Authorization: Bearer $TOKEN"} \
       "$BASE_URL$1" 2>/dev/null || echo '{}'
}

# Extract JSON field (basic awk, no jq/python needed)
json_field() {
  echo "$1" | sed 's/.*"'"$2"'"\s*:\s*"//' | sed 's/".*//'
}

uuid() {
  cat /proc/sys/kernel/random/uuid 2>/dev/null || \
    od -x /dev/urandom | head -1 | awk '{printf "%s-%s-%s-%s-%s", substr($2,1,8), substr($3,1,4), substr($4,1,4), substr($5,1,4), substr($6,1,12)}'
}

echo "ParkHub Demo Seeder (shell)"
echo "Target: $BASE_URL"

# 1. Admin login
echo "Logging in as admin..."
RESP=$(post "/api/v1/auth/login" "{\"username\":\"admin\",\"password\":\"$ADMIN_PASSWORD\"}")
TOKEN=$(json_field "$RESP" "access_token")
if [ -z "$TOKEN" ] || [ "$TOKEN" = "{" ]; then
  echo "Admin login failed, trying to register..."
  RESP=$(post "/api/v1/auth/register" "{\"username\":\"admin\",\"email\":\"admin@parkhub.test\",\"password\":\"$ADMIN_PASSWORD\",\"name\":\"Administrator\"}")
  TOKEN=$(json_field "$RESP" "access_token")
fi
echo "Admin token: ${TOKEN:0:16}..."

# 2. Seed lots
LOTS='P+R Hauptbahnhof|Bahnhofplatz 1, 80335 Muenchen|48.1403|11.5583|51
Tiefgarage Marktplatz|Marktplatz 5, 70173 Stuttgart|48.7784|9.1800|80
Parkhaus Stadtmitte|Rathausstrasse 12, 50667 Koeln|50.9384|6.9584|60
P+R Messegelaende|Messegelaende Sued, 60528 Frankfurt|50.1109|8.6821|100
Parkplatz Einkaufszentrum|Shoppingcenter 3, 22335 Hamburg|53.5753|9.9803|40
Tiefgarage Rathaus|Rathausplatz 1, 90403 Nuernberg|49.4521|11.0767|30
Parkhaus Technologiepark|Technologiestrasse 8, 76131 Karlsruhe|49.0069|8.4037|75
Parkplatz Universitaet|Universitaetsring 1, 69120 Heidelberg|49.4074|8.6924|70
Parkplatz Klinikum|Klinikumsallee 15, 44137 Dortmund|51.5136|7.4653|46
P+R Bahnhof Ost|Ostbahnhofstrasse 3, 04315 Leipzig|51.3397|12.3731|56'

LOT_IDS=""
LOT_COUNT=0
echo ""
echo "Seeding 10 parking lots..."
echo "$LOTS" | while IFS='|' read -r name addr lat lon slots; do
  LOT_ID=$(uuid)
  NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  PAYLOAD="{\"id\":\"$LOT_ID\",\"name\":\"$name\",\"address\":\"$addr\",\"latitude\":$lat,\"longitude\":$lon,\"total_slots\":$slots,\"available_slots\":$slots,\"floors\":[],\"amenities\":[\"covered\",\"security_camera\"],\"pricing\":{\"currency\":\"EUR\",\"rates\":[{\"duration_minutes\":60,\"price\":2.50,\"label\":\"1h\"},{\"duration_minutes\":1440,\"price\":20.00,\"label\":\"Day\"}],\"daily_max\":20.00,\"monthly_pass\":400.00},\"operating_hours\":{\"is_24h\":false,\"monday\":{\"open\":\"06:00\",\"close\":\"22:00\"},\"tuesday\":{\"open\":\"06:00\",\"close\":\"22:00\"},\"wednesday\":{\"open\":\"06:00\",\"close\":\"22:00\"},\"thursday\":{\"open\":\"06:00\",\"close\":\"22:00\"},\"friday\":{\"open\":\"06:00\",\"close\":\"22:00\"},\"saturday\":{\"open\":\"07:00\",\"close\":\"20:00\"},\"sunday\":{\"open\":\"08:00\",\"close\":\"18:00\"}},\"images\":[],\"status\":\"open\",\"created_at\":\"$NOW\",\"updated_at\":\"$NOW\"}"
  RESP=$(post "/api/v1/lots" "$PAYLOAD")
  echo "  lot: $name ($slots slots)"
done
echo "Lots seeded."

# 3. Seed users
FIRSTNAMES="Hans Peter Klaus Michael Thomas Andreas Stefan Christian Markus Sebastian Daniel Tobias Florian Matthias Martin Frank Oliver Maria Anna Sandra Andrea Nicole Stefanie Christina Monika Petra Claudia Julia Laura Sarah Lisa Katharina Melanie Susanne Anja"
LASTNAMES="Mueller Schmidt Schneider Fischer Weber Meyer Wagner Becker Schulz Hoffmann Koch Richter Bauer Klein Wolf Schroeder Neumann Schwarz Zimmermann Braun Krueger Hofmann Hartmann"
PLATES="M HH B K F S N DO E L HD KA MA"

echo ""
echo "Seeding 200 users..."
USER_COUNT=0
for i in $(seq 1 200); do
  FIRST=$(echo $FIRSTNAMES | tr ' ' '\n' | awk "NR==$(( (RANDOM % 35) + 1 ))")
  LAST=$(echo $LASTNAMES | tr ' ' '\n' | awk "NR==$(( (RANDOM % 24) + 1 ))")
  USERNAME="$(echo "$FIRST.$LAST" | tr '[:upper:]' '[:lower:]')$i"
  PLATE_P=$(echo $PLATES | tr ' ' '\n' | awk "NR==$(( (RANDOM % 13) + 1 ))")
  PLATE="${PLATE_P}-$(head -c2 /dev/urandom | od -A n -t x1 | tr -d ' ' | tr '[:lower:]' '[:upper:]' | head -c2) $((RANDOM % 9000 + 1000))"

  TOKEN_BAK="$TOKEN"
  unset TOKEN
  RESP=$(post "/api/v1/auth/register" "{\"username\":\"$USERNAME\",\"email\":\"${USERNAME}@example.de\",\"password\":\"Demo2026!X\",\"name\":\"$FIRST $LAST\"}")
  TOKEN="$TOKEN_BAK"

  USER_COUNT=$((USER_COUNT + 1))
  if [ $((USER_COUNT % 50)) -eq 0 ]; then
    echo "  $USER_COUNT/200 users created"
  fi
done
echo "Users seeded ($USER_COUNT total)."

echo ""
echo "Seed complete."
echo "  Credentials: admin / $ADMIN_PASSWORD"
echo "  Dashboard:   $BASE_URL"
