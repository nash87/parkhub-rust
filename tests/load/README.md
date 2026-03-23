# ParkHub Load Tests (k6)

Performance and load testing scripts using [k6](https://grafana.com/docs/k6/).

## Prerequisites

Install k6:

```bash
# macOS
brew install k6

# Linux (Debian/Ubuntu)
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
  --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" \
  | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update && sudo apt-get install k6

# Docker
docker run --rm -i grafana/k6 run - <tests/load/smoke.js
```

## Test Scenarios

| Script | VUs | Duration | Purpose |
|--------|-----|----------|---------|
| `smoke.js` | 1 | 30s | Sanity check — health, login, bookings |
| `load.js` | 50 | 5min | Sustained load — full booking flow |
| `stress.js` | 100 | 10min | All endpoints under heavy load |
| `spike.js` | 200 | ~4min | Sudden traffic surge (1 to 200 VUs) |

## Running

```bash
# Start ParkHub first
./target/release/parkhub-server --headless --unattended

# Smoke test (quick sanity)
k6 run tests/load/smoke.js

# Load test
k6 run tests/load/load.js

# Stress test
k6 run tests/load/stress.js

# Spike test
k6 run tests/load/spike.js
```

## Configuration

Override defaults via environment variables:

```bash
K6_BASE_URL=https://parking.example.com \
K6_ADMIN_EMAIL=admin@corp.com \
K6_ADMIN_PASSWORD=secret \
K6_USER_EMAIL=user@corp.com \
K6_USER_PASSWORD=secret \
  k6 run tests/load/load.js
```

## Interpreting Results

Key metrics to watch:

| Metric | Target | Description |
|--------|--------|-------------|
| `http_req_duration (p95)` | < 500ms | 95th percentile response time |
| `http_req_duration (p99)` | < 1500ms | 99th percentile response time |
| `http_req_failed` | < 1% | Error rate |
| `http_reqs` | > 10/s | Throughput |

## Output

Export results to JSON or InfluxDB for dashboarding:

```bash
# JSON output
k6 run --out json=results.json tests/load/load.js

# InfluxDB (for Grafana dashboards)
k6 run --out influxdb=http://localhost:8086/k6 tests/load/load.js
```
