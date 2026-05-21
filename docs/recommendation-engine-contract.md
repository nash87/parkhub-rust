# ParkHub Recommendation Engine Contract

Status: T-6318 SP1-SP5 draft, Rust side

## Purpose

ParkHub recommendations now have an explicit `weighted_v1` contract. The first
slice codifies the shared deterministic scoring behavior and moves the weights
behind a named config surface so the Rust API can later consume the shared
`fop-pipeline` recommender without another handler-local scoring fork.

## Stable Algorithm

`weighted_v1` is the deterministic rollback algorithm. `fop_pipeline_v1` is the
adapter algorithm for the external fop-pipeline service and must fall back to
`weighted_v1` on every missing endpoint, timeout, non-2xx response, invalid
response, or unknown slot ID.

Default weights:

| Key | Default | Meaning |
| --- | ---: | --- |
| `weight_frequency` | 40 | Maximum points for repeatedly using the same slot. |
| `weight_preferred_lot` | 20 | Maximum points for using the same lot when the exact slot has no history. |
| `weight_availability` | 30 | Points for an available slot. |
| `weight_price` | 20 | Maximum points for lower-priced lots. |
| `weight_distance` | 10 | Maximum points for slots near the entrance. |
| `weight_accessibility_bonus` | 0 | Optional extra points for facility-designated accessible slots. |
| `weight_feature_bonus` | 2 | Tiebreaker points for slot feature metadata. |
| `max_results` | 5 | Maximum results returned by the endpoint. |
| `pipeline_endpoint` | empty | Optional local/cluster fop-pipeline base URL. External hosts are rejected. |
| `pipeline_name` | `parkhub-recommendations` | Pipeline name used by `POST /pipeline/{name}/run`. |
| `pipeline_timeout_ms` | 750 | Request timeout before fallback. |
| `pipeline_fallback_enabled` | true | Fail-closed: fallback to `weighted_v1` is mandatory until certification. |
| `explain` | true | Fail-closed: reasons and badges remain enabled until legal/privacy review approves disabling them. |
| `profile_safe_mode` | true | Fail-closed privacy guardrail for current and future scoring inputs. |

Formula notes:

- `frequency`: `min(slot_usage_count, 10) / 10 * weight_frequency`.
- `preferred_lot`: only applies when the exact slot has no usage history:
  `min(lot_usage_count, 10) / 10 * weight_preferred_lot`.
- `availability`: every available, unbooked slot gets `weight_availability`.
- `price`: normalize within the candidate lot set:
  `(1 - lot_hourly_rate / max_candidate_hourly_rate) * weight_price`, clamped at
  zero for outlier rates; missing or zero rates receive no price bonus.
- `distance`: `weight_distance / max(slot_number, 1)`.
- `accessibility_bonus` and `feature_bonus`: additive opt-in tiebreakers.
  `is_accessible` and `features` are facility attributes only. They must never
  be inferred from user disability, health, or other sensitive personal
  attributes; `accessibility_bonus` stays `0` unless tenant DPIA/privacy review
  and user-facing notice approve changing it.

Changing `weighted_v1` semantics is not allowed. Any ML or tenant-specific
strategy must be introduced as a new algorithm version and must pass parity
fixtures against `weighted_v1` before rollout.

## Config Boundary

The Rust module registry exposes a JSON Schema for `recommendations` through the
existing admin module config editor. Values are persisted under
`module.recommendations.config.*` and loaded by the recommendations API with
legacy-safe defaults. The `explain` and `profile_safe_mode` settings are
reserved, fail-closed fields: attempts to set them to `false` are rejected by
schema and ignored by runtime loading.

`fop_pipeline_v1` uses the fop-pipeline JSON/HTTP boundary:
`POST {pipeline_endpoint}/pipeline/{pipeline_name}/run`. ParkHub sends the
candidate slots, weights, `profile_safe_mode`, explanation requirement, and
`fallback_algorithm=weighted_v1`. The adapter only accepts localhost/loopback,
explicit local-dev `.test` hosts, or Kubernetes service hosts shaped as
`<service>.<namespace>.svc` / `<service>.<namespace>.svc.cluster.local` by
default and records whether the pipeline was attempted, succeeded, or fell back.

The response continues to include reasons and badges. Shared parity fixtures
live under `docs/recommendation-engine-fixtures/` and are the contract for Rust,
PHP, and any future fop-pipeline adapter. `profile_safe_mode` stays enabled by
default and is reserved as the privacy gate for the future fop-pipeline adapter.
The stats endpoint also emits a machine-readable legal boundary:
`legal_review_required=true`, `attorney_review_status=required_before_customer_wording`,
and `execution_allowed=false` for generated/public profiling or legal wording.

Every served recommendation batch writes a best-effort `RecommendationServed`
audit event keyed by `batch_id`; each returned slot has its own
`recommendation_id`. The event stores the algorithm, SHA-256 config hash,
SHA-256 weights hash, `profile_safe_mode`, `explain`, adapter status,
per-candidate recommendation IDs, candidate slot IDs, scores, reason badges,
reasons, and the legal boundary. The stats endpoint is derived from these served
audit events. Acceptance metrics remain `null` with
`acceptance_metric_source=not_tracked` until explicit accept/reject events exist,
so the endpoint does not infer acceptance from unrelated booking state.

## Compliance Boundary

This is engineering compliance, not legal advice. For German/EU/international
use, the recommendation surface must keep:

- data minimization: no sensitive categories, location history beyond parking
  usage, or unrelated profile attributes in the score inputs;
- explainability: every score must keep a reason or badge that can be audited;
- operator control: weight changes must be authenticated, audited, and reversible;
- security evidence: SBOM/provenance/vulnerability handling remains part of the
  ParkHub CI/CD baseline before business rollout;
- legal review: public ToS/privacy/profiling wording must go through `fop legal`
  plus attorney review before being treated as customer-ready.

The Nido/fop legal catalog service (current CLI entrypoint:
`fop legal catalog --json`; `nido legal` is not exposed by the installed Nido
CLI yet) currently marks the local Claude-for-Legal catalog as reference-only.
Its release evidence fields include `source_revision`, `generated_at`,
`requires_attorney_review=true`, `requires_human_signoff=true`,
`execution_allowed=false`, and `safety_boundary`. ParkHub mirrors that boundary
in recommendation stats so operators can see that compliance support is present
but not a substitute for counsel.

2026 compliance posture gates before business rollout:

- SBOM, provenance, image digest, and VEX/vulnerability evidence attached to the
  ParkHub Rust/PHP release artifacts;
- documented vulnerability disclosure and security update process;
- audit evidence retention for module config changes and served
  `RecommendationServed` decisions;
- CRA/NIS2/AI Act/GDPR milestone tracking in the fop task board before
  customer-facing profiling language ships.

Relevant current public references:

- European Commission, GDPR data protection by design and by default:
  https://commission.europa.eu/law/law-topic/data-protection/rules-business-and-organisations/obligations/what-does-data-protection-design-and-default-mean_en
- European Commission, Cyber Resilience Act:
  https://digital-strategy.ec.europa.eu/en/policies/cyber-resilience-act
- European Commission, CRA summary:
  https://digital-strategy.ec.europa.eu/en/policies/cra-summary
- European Commission, NIS2 Directive overview:
  https://digital-strategy.ec.europa.eu/en/policies/nis2-directive
- European Commission, AI Act transparency guidance:
  https://digital-strategy.ec.europa.eu/en/faqs/guidelines-and-code-practice-transparent-ai-systems
- BSI IT-Grundschutz-Kompendium:
  https://www.bsi.bund.de/DE/Themen/Unternehmen-und-Organisationen/Standards-und-Zertifizierung/IT-Grundschutz/IT-Grundschutz-Kompendium/it-grundschutz-kompendium_node.html

## Legal Review Packet

`fop legal` can draft the supporting documents, and
`fop legal catalog --json` can provide the current review catalog provenance and
safety flags, but generated text and catalog entries are not shipping approval.
Treat the commands below as review inputs only:

```bash
NO_COLOR=true fop legal privacy "ParkHub"
NO_COLOR=true fop legal tos "ParkHub"
```

Before enabling `fop_pipeline_v1` for any customer tenant, the rollout packet
must contain:

1. product counsel approval for the privacy-policy and ToS wording that names
   recommendation logic, parking-history use, explanation output, and opt-out or
   operator override behavior;
2. a tenant data-processing note that confirms the legal basis for parking
   history, lot/slot metadata, and recommendation audit retention;
3. a DPIA or explicit DPIA-not-required decision before changing
   `weight_accessibility_bonus` above `0` or adding any tenant-specific
   behavioral/personalization input;
4. an Art. 30/records-of-processing update for the
   `RecommendationServed` audit event, including retention and export paths;
5. a security release packet with SBOM, provenance, image digest,
   vulnerability/VEX status, dependency license review, and incident/update
   process evidence;
6. an operational acceptance record showing local/cluster-only
   `pipeline_endpoint` allowlisting, timeout/fallback behavior, health checks,
   and a tested rollback to `weighted_v1`.

For personal or local evaluation, keep `weighted_v1` and the default
`execution_allowed=false` legal boundary. For business/customer operation,
do not present generated recommendation or legal text as approved until the
packet above is complete and signed off.

## Next Slice

1. Keep the shared JSON fixture wired into Rust and PHP tests whenever
   recommendation scoring changes.
2. Add runtime certification/health gates for `fop_pipeline_v1` before enabling
   it outside local/cluster controlled endpoints.
3. Keep `weighted_v1` as the rollback default until CI proves parity and the
   legal/privacy review has accepted the customer-facing wording.
