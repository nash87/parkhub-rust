#!/bin/sh
set -e

CONFIG_FILE="/data/config.toml"

# Start server in background (--unattended creates config + admin + dummy users on first run)
/app/parkhub-server --headless --unattended --data-dir /data --port 10000 &
SERVER_PID=$!

# Wait for server to be healthy
echo "Waiting for server to start..."
for i in $(seq 1 60); do
    if wget -q --spider http://localhost:10000/health 2>/dev/null; then
        echo "Server is healthy."
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "WARNING: Server did not become healthy in 60s, skipping seeder."
        wait $SERVER_PID
        exit $?
    fi
    sleep 1
done

# Seed demo data if DEMO_MODE is enabled and database is fresh
if [ "${DEMO_MODE}" = "true" ]; then
    echo "DEMO_MODE=true — checking if seeding is needed..."
    LOT_COUNT=$(wget -qO- http://localhost:10000/api/v1/lots 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('total',len(d.get('data',d if isinstance(d,list) else []))))" 2>/dev/null || echo "0")
    if [ "$LOT_COUNT" -le 1 ]; then
        echo "Seeding demo data (10 lots, 200 users, ~3500 bookings)..."

        # Enable self-registration temporarily so seed script can create users
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        if [ -f "$CONFIG_FILE" ]; then
            sed -i 's/allow_self_registration = false/allow_self_registration = true/' "$CONFIG_FILE"
            echo "  → Temporarily enabled self-registration for seeding."
        fi

        # Restart server with self-registration enabled
        /app/parkhub-server --headless --data-dir /data --port 10000 &
        SERVER_PID=$!
        echo "  → Restarting server for seeding..."
        for i in $(seq 1 30); do
            if wget -q --spider http://localhost:10000/health 2>/dev/null; then
                break
            fi
            sleep 1
        done

        # Run the seed script
        python3 /app/seed_demo.py --base-url http://localhost:10000 --admin-password "${PARKHUB_ADMIN_PASSWORD:-ParkHub2026!}" 2>&1 || echo "WARNING: Demo seeding failed"
        echo "Demo seeding complete."

        # Disable self-registration again
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        if [ -f "$CONFIG_FILE" ]; then
            sed -i 's/allow_self_registration = true/allow_self_registration = false/' "$CONFIG_FILE"
            echo "  → Disabled self-registration after seeding."
        fi

        # Start server for real
        /app/parkhub-server --headless --data-dir /data --port 10000 &
        SERVER_PID=$!
        echo "Server restarted in production mode."
    else
        echo "Demo data already exists ($LOT_COUNT lots), skipping seed."
    fi
fi

# Wait for server process
wait $SERVER_PID
