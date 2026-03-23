# ParkHub Helm Chart

Deploy ParkHub to Kubernetes using Helm.

## Prerequisites

- Kubernetes 1.25+
- Helm 3.10+

## Install

```bash
helm install parkhub ./helm/parkhub \
  --namespace parkhub --create-namespace
```

## Install with custom values

```bash
helm install parkhub ./helm/parkhub \
  --namespace parkhub --create-namespace \
  -f my-values.yaml
```

## Configuration

Key values (see `helm/parkhub/values.yaml` for full reference):

| Parameter | Default | Description |
|-----------|---------|-------------|
| `replicaCount` | `1` | Number of replicas |
| `image.repository` | `ghcr.io/nash87/parkhub-rust` | Container image |
| `image.tag` | `appVersion` | Image tag |
| `service.type` | `ClusterIP` | Service type |
| `ingress.enabled` | `false` | Enable ingress |
| `persistence.enabled` | `true` | Enable persistent storage |
| `persistence.size` | `1Gi` | PVC size |
| `config.adminPassword` | `""` | Admin password (auto-generated if empty) |
| `config.dbPassphrase` | `""` | AES-256-GCM encryption key |
| `config.smtp.*` | `""` | SMTP settings |
| `config.stripe.*` | `""` | Stripe payment keys |
| `config.oauth.*` | `""` | OAuth provider credentials |
| `modules.*` | `true` | Feature flag toggles |
| `autoscaling.enabled` | `false` | Enable HPA |
| `resources.limits.memory` | `256Mi` | Memory limit |

## Module flags

All 51 module flags are exposed in `values.yaml` under `modules.*`. Set any to `false` to disable:

```yaml
modules:
  evCharging: false
  dynamicPricing: false
```

## Ingress with TLS

```yaml
ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: parking.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: parkhub-tls
      hosts:
        - parking.example.com
```

## Upgrade

```bash
helm upgrade parkhub ./helm/parkhub --namespace parkhub
```

## Uninstall

```bash
helm uninstall parkhub --namespace parkhub
```
