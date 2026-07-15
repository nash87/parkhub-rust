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

## Current parity (2026-06-14, committed snapshots)

Recomputed from the two **committed** OpenAPI snapshots via the guarded
`parkhub-php` normaliser (`scripts/diff-openapi.sh`, the awk-guarded one; the
`parkhub-rust` copy rewrites Rust's own paths and mis-counts):

- Rust input: `docs/openapi/rust.json` — last regenerated at
  `parkhub-rust@3aa68fc7` (2026-06-10, #686), == current `github/main`.
- PHP input: `docs/openapi/php.json` — last regenerated at
  `parkhub-php@120cdc95` (2026-06-14, #542), == current `github/main`.

Both snapshots track their repo's current main head, so these figures are the
live parity state as of 2026-06-14, not a stale branch comparison.

| Source | Path count (normalised) |
|--------|-------------------------|
| Rust (`utoipa`) | 249 |
| PHP (Scramble) | 329 |
| Shared | 219 |
| Rust-only drift | 30 |
| PHP-only drift | 110 |
| Total drift | 140 |

These supersede the stale 2026-05-12 figures (239/318/210/29/108). The snapshots
moved ~4 weeks of feature commits (rust.json → #686, php.json → #542) that this
doc never absorbed; total drift rose 137 → 140 as both sides shipped features.

**Load-bearing finding — the 110 PHP-only paths are NOT 110 missing features.**
A src-level audit splits them into:

- **~49 annotation gaps** — the handler already exists and is routed in
  `parkhub-server`, it only lacks a `#[utoipa::path]` annotation, so it never
  reaches the spec (e.g. `admin/analytics/*`, `admin/reports/schedules`,
  `bookings/history`, `bookings/stats`, `me`, `setup/wizard`). Several are
  **live SPA consumers** (`AdminAnalytics.tsx:111`, `AdminScheduledReports.tsx:55`)
  reading as gaps. Closing these is pure contract work, zero business logic —
  the shape of the merged #631 alias tranche.
- **~61 genuine functional gaps** — no Rust handler yet (booking lifecycle
  extend/cancel, parts of admin bookings, etc.).

So ~45% of the drift closes by annotation alone. Sequencing and the TDD slice
queue live in `_handoffs/parkhub/2026-07-15-enhance-wave-tdd-plan.md`.

Current drift clusters (recomputed 2026-06-14):

| Cluster | Rust-only | PHP-only |
|---|---:|---:|
| Admin/reporting/settings | 14 | 54 |
| Bookings/QR/calendar | 2 | 15 |
| Lots/zones/pricing | 5 | 7 |
| Setup/system/modules | 0 | 9 |
| User/me/vacation/absence | 3 | 14 |
| Demo/discovery/public | 0 | 4 |
| Payments/notifications/other | 6 | 7 |

> Caveat: this is a **committed-snapshot recompute**, not a fresh server boot.
> An authoritative recount (`--features full,headless` + boot `:18181` +
> `scripts/dump-openapi.sh`) should be run after the next annotation tranches
> land to confirm the 110 → ~61 drop.

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


