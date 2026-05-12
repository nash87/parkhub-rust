# OpenAPI Parity — parkhub-rust ↔ parkhub-php

## Why this document exists

ParkHub ships as **two interoperable implementations** of the same HTTP API:

- `parkhub-rust` (axum 0.8, utoipa) — primary performance target.
- `parkhub-php` (Laravel 13, dedoc/scramble) — primary integration target
  and shared-hosting deployment option.

Clients (the shared `parkhub-web` SPA, mobile apps, operator-written
integrations) must not see a behavioural difference between the two
backends. A silent endpoint gap on either side is exactly the kind of
"works on my dev box" bug that shows up in production when an operator
migrates between the two.

This file captures the current parity state, the diff methodology, and the
TODOs needed to close the gap.

See also:

- [parity-governance.md](parity-governance.md)
- [release-checklist.md](release-checklist.md)

---

## Current parity (2026-05-12)

Latest alias-tranche comparison from regenerated local OpenAPI dumps:

- Rust input: `parkhub-rust@65752756ea9c775abca0a317c0ff42ac4535891e`
  on branch `t-parkhub-openapi-alias-tranche`, based on
  `github/main@24b763193130f1761d018893ed46334390cfd6ae`
  (`docs/openapi/rust.json`)
- PHP input: `parkhub-php@63a7a5228039657938733538aa53a24b7cf0b352`
  on branch `t-parkhub-openapi-alias-tranche`, based on
  `github/main@83a132283550d80e4a0553495bd05daf25093f8b`
  (`docs/openapi/php.json`)

| Source | Path count (normalised) |
|--------|-------------------------|
| Rust (`utoipa`) | 239 |
| PHP (Scramble) | 318 |
| Shared | 210 |
| Rust-only drift | 29 |
| PHP-only drift | 108 |
| Total drift | 137 |

The numbers above come from regenerated OpenAPI dumps, not grep/static route
extractors. This alias tranche reduced total drift from `145` to `137` by adding
thin compatibility aliases for `login`, `register`, `refresh`,
`auth/change-password`, `health/detailed`, `status`, and the public docs
surfaces.

Current drift clusters:

| Cluster | Rust-only | PHP-only |
|---|---:|---:|
| Admin/reporting/settings | 14 | 39 |
| Auth/profile/setup aliases | 1 | 7 |
| Booking/QR/calendar | 1 | 15 |
| Health/docs/status | 5 | 3 |
| Import/export | 1 | 4 |
| Payments/billing/pricing | 4 | 1 |
| Demo/discovery/public | 1 | 15 |
| User/tenant/vehicle/notification | 2 | 12 |
| Other | 0 | 12 |

## Methodology

### `scripts/diff-openapi.sh`

Runs in either repo. Hands two URLs (or committed JSON dumps) to `jq`,
normalises path parameters (`{id}` / `{uuid}` / `{slug}` → `{id}`) so
routes that differ only in parameter name don't look drifted, and `comm`s
the two sorted path lists.

Exit code `1` on any drift → safe to gate a CI step on.

```bash
# Against live servers (Rust on 8081, PHP on 8000)
./scripts/diff-openapi.sh \
  http://localhost:8081/api-docs/openapi.json \
  http://localhost:8000/docs/api.json

# Against committed dumps
./scripts/diff-openapi.sh \
  docs/openapi/rust.json \
  docs/openapi/php.json
```

### Committing dumps (recommended)

To avoid requiring both servers to be running during CI, add a job that:

1. Builds the Rust server and curls its `/api-docs/openapi.json` endpoint.
2. Boots the PHP server (via `php artisan serve` or the Docker image) and
   curls its Scramble endpoint.
3. Writes both to `docs/openapi/{rust,php}.json`.
4. Runs `./scripts/diff-openapi.sh docs/openapi/rust.json docs/openapi/php.json`.
5. Fails the job if the diff is non-empty and no `docs/openapi/drift-allow`
   allow-list entry covers it.

The dumps themselves should be **committed** so reviewers can see contract
changes in the PR diff, not hidden inside a CI artifact.

Until a dedicated `docs/openapi/canonical.json` lands, treat
`docs/openapi/rust.json` as the working machine-contract baseline for parity
review.

## Known drift categories

With the current committed snapshots and the input-specific normalisation in
`scripts/diff-openapi.sh`, the parity diff is still materially open:

- Rust-only paths: `29`
- PHP-only paths: `108`

That means parity is **not** currently "just static-extractor noise". The
remaining drift falls into four broad buckets:

### 1. Admin routing prefix chains (PHP side)

Many PHP admin endpoints use `Route::middleware('admin')->prefix('admin')->group(...)`
blocks. A naive static extractor captures just the inner path, making them
look like drift. **Effect on real parity: zero** — the routes exist, the
extractor just didn't see them correctly.

**Action**: rely on the Scramble JSON dump (runtime-accurate), not `grep`.

### 2. Genuine Rust-only contract surfaces

Rust still exposes paths the PHP contract does not currently publish, including
top-level operational surfaces (`/status`, `/health*`, `/handshake`), admin
export/settings endpoints, booking QR under `/api/v1/bookings/{id}/qr`, and the
Rust-style payments/config surface.

**Action**: close these in small batches instead of one mega-port:
auth/profile/public aliases, health/docs surfaces, booking/payment aliases,
then admin/export/settings tails.

### 3. Genuine PHP-only contract surfaces

PHP still publishes a substantially larger surface, including profile/setup
aliases, demo/discovery endpoints, broader admin analytics/settings/reporting
routes, and several booking/user convenience routes.

**Action**: for each cluster decide whether it is
(a) a missing Rust alias/annotation,
(b) a real feature port still needed,
or (c) an intentional divergence that must be documented explicitly.

### 4. Parameter-name noise

`/api/v1/lots/{id}` vs `/api/v1/lots/{uuid}` — same endpoint, different
OpenAPI parameter name. The diff script already normalises these; should
never appear in a real drift report.

## Open follow-up tasks

- **Truthful repo messaging**: README/AGENTS must say parity is tracked, not yet hard-enforced end-to-end.
- **Real cross-repo CI gate**: current workflows check only self-snapshot drift; add a second-repo checkout and run `diff-openapi.sh` for real Rust-vs-PHP gating once the diff is smaller.
- **Alias tranche**: continue with the remaining cheap mismatches
  (booking QR/payment/config, import aliases, profile/setup aliases) and classify
  Rust top-level operational endpoints explicitly.
- **Feature tranche**: close the remaining admin/reporting/demo/user feature gaps or explicitly classify intentional divergences.
