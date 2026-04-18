# Service Level Objectives (SLOs)

This document defines the production SLOs for self-hosted ParkHub deployments. The hosted demo at `parkhub-rust-demo.onrender.com` runs on Render's free tier and is **not** covered — it ships with a cold-start window and is an evaluation surface, not a production surface.

SLOs give operators a measurable contract against which to tune alerting + capacity. Error-budget burn-rate alerts are the recommended paging signal; absolute thresholds are noisy and brittle.

## Scope

SLOs are measured at the ingress layer (reverse proxy / Kubernetes Service) and exclude:
- Scheduled background jobs (AutoRelease, ExpandRecurring, PurgeExpired, AggregateOccupancy — these get separate `parkhub_job_runs_total{success}` alerts).
- Admin-only endpoints under `/api/v1/admin/*` (lower traffic, lower criticality).
- `/health/live` + `/health/ready` — these are probe endpoints, not user-facing.

## Target SLOs

| SLI | Target | Window | Rationale |
|-----|--------|--------|-----------|
| **Availability** — successful HTTP responses (status `<500`) | **99.9 %** | 30-day rolling | User-facing API must not fail on 1 in 1000 requests. 0.1 % budget = 43 min/month. |
| **Latency** — p95 request duration at `/api/v1/bookings*` | **≤ 500 ms** | 30-day rolling | Booking flow is the primary user interaction; above 500 ms users perceive the UI as slow. |
| **Latency** — p99 request duration at `/api/v1/bookings*` | **≤ 2000 ms** | 30-day rolling | Tail latency cap — anything above 2s cross-request is a bug, not load. |
| **Booking success rate** — 2xx responses on `POST /api/v1/bookings` | **99.5 %** | 7-day rolling | Core money-flow; budget = 3.6 failures per 1000 bookings. |
| **Data durability** — 0 data-loss incidents | **100 %** | All-time | redb with daily backup + WAL replay. Zero tolerance for silent corruption. |

## Burn-rate alerts

The recommended paging pattern is **multi-window multi-burn-rate**: alert when the error budget is consumed faster than sustainable.

For the 99.9 % availability SLO with a 30-day window (0.1 % monthly budget ≈ 43 min):

| Severity | Condition | Window | Ticket |
|----------|-----------|--------|--------|
| **Page** | Burn rate ≥ **14.4×** (1 h window) AND ≥ **14.4×** (5 min window) | Short | 2 % of monthly budget burned in 1 hour — urgent |
| **Page** | Burn rate ≥ **6×** (6 h window) AND ≥ **6×** (30 min window) | Medium | 5 % of monthly budget burned in 6 hours — urgent |
| **Ticket** | Burn rate ≥ **3×** (24 h window) AND ≥ **3×** (2 h window) | Long | 10 % of monthly budget burned in a day — investigate |
| **Ticket** | Burn rate ≥ **1×** (72 h window) AND ≥ **1×** (6 h window) | Very long | Sustained degradation — capacity or dep issue |

PrometheusRule with these burn-rate queries ships in `helm/parkhub/templates/prometheusrule.yaml`.

## Computing burn rate (Prometheus)

```promql
# 1-hour window burn rate for availability SLO
(
  sum(rate(parkhub_http_requests_total{status=~"5.."}[1h]))
  /
  sum(rate(parkhub_http_requests_total[1h]))
) / (1 - 0.999)
```

Values above 1.0 = budget burning. 14.4 = burning 14.4× sustainable rate.

## Dashboard

The default Grafana dashboard (`helm/parkhub/dashboards/parkhub-overview.json`) includes panels for availability, latency percentiles, and burn rate. Opt-in via `grafana.dashboardsEnabled=true` in `values.yaml`.

## Review cadence

Review SLOs quarterly. Adjust targets based on observed baselines — SLOs should push the system slightly harder than current performance, not enshrine the status quo. If a target is never missed, it's too loose; if it's missed monthly, it's too tight or the system has real problems.

## Related docs

- [Helm chart README](../helm/README.md) — deployment knobs
- [ARCHITECTURE](../ARCHITECTURE.md#observability) — metrics inventory
- [COMPLIANCE](./COMPLIANCE.md) — regulatory context for downtime
