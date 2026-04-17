# ParkHub Rust Helm Chart — Kubernetes Hardening

Status: **enabled by default**.

## What you get out of the box

The chart renders a Deployment that satisfies the Kubernetes [Pod Security
Standards **restricted**](https://kubernetes.io/docs/concepts/security/pod-security-standards/#restricted)
profile:

| Control                           | Setting                                       |
|-----------------------------------|-----------------------------------------------|
| Runs as non-root                  | `runAsNonRoot: true`, `runAsUser: 1000`       |
| Privilege escalation              | `allowPrivilegeEscalation: false`             |
| Linux capabilities                | `drop: [ALL]`                                 |
| Read-only root filesystem         | `readOnlyRootFilesystem: true`                |
| Seccomp profile                   | `seccompProfile.type: RuntimeDefault`         |

`parkhub-server` is a static Rust binary backed by redb, so the only
writable path it needs is the PVC mount at `/data` (see
`persistence.mountPath`). No tmpfs scaffolding is required.

## Verifying after a deploy

```bash
kubectl get pod -l app.kubernetes.io/name=parkhub -o jsonpath='{.items[0].spec.securityContext}'
kubectl get pod -l app.kubernetes.io/name=parkhub -o jsonpath='{.items[0].spec.containers[0].securityContext}'
kubectl describe pod -l app.kubernetes.io/name=parkhub | grep -i -E 'warning|forbidden'
```

## Observability

The chart ships optional Prometheus Operator CRDs behind two flags:

| Flag                                    | Default | Renders                                           |
|-----------------------------------------|---------|---------------------------------------------------|
| `monitoring.serviceMonitor.enabled`     | `false` | `ServiceMonitor` scraping `/metrics` at `30s`     |
| `monitoring.prometheusRule.enabled`     | `false` | `PrometheusRule` with four golden-signal alerts   |

Both flags default to `false` so the chart still installs cleanly on
clusters without `monitoring.coreos.com/v1` CRDs. Flip them on in
your values override once `prometheus-operator` is present:

```yaml
monitoring:
  serviceMonitor:
    enabled: true
    # additionalLabels:
    #   release: kube-prometheus-stack
  prometheusRule:
    enabled: true
    # alertLabels:
    #   team: platform
```

Alerts emitted (`parkhub.rules` group):

- `ParkhubHighErrorRate` — 5xx ratio > 5% for 10m (warning)
- `ParkhubHighLatencyP99` — p99 HTTP latency > 500ms for 10m (warning)
- `ParkhubJobFailureRate` — `job_runs_total{success="false"}` rate > 0.1/s for 15m (warning)
- `ParkhubServiceDown` — `up{job=~"parkhub.*"} == 0` for 5m (critical)

## Relationship to the PHP chart

The `parkhub-php` sibling chart runs Laravel on Apache, which carries
extra writable-path plumbing (Laravel cache, Apache run/lock, `/tmp`).
Both charts share the same `monitoring.*` value keys for parity.
