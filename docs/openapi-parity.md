# OpenAPI Parity — parkhub-rust ↔ parkhub-php

## Why this document exists

ParkHub ships as **two interoperable implementations** of the same HTTP API:

- `parkhub-rust` (axum 0.8, utoipa) — primary performance target.
- `parkhub-php` (Laravel 12/13, dedoc/scramble) — primary integration target
  and shared-hosting deployment option.

Clients (the shared `parkhub-web` SPA, mobile apps, operator-written
integrations) must not see a behavioural difference between the two
backends. A silent endpoint gap on either side is exactly the kind of
"works on my dev box" bug that shows up in production when an operator
migrates between the two.

This file captures the current parity state, the diff methodology, and the
TODOs needed to close the gap.

---

## Current parity (2026-04-17)

A cross-repo diff from the running servers against the committed route
listings shows roughly:

| Source | Path count (normalised, `/api/v1/*`) |
|--------|--------------------------------------|
| Rust (`utoipa::path` macros) | ~223 |
| PHP (Scramble-derived)       | ~279 |
| Shared                       | ~140 |
| Drift                        | ~160 (split ~80 / 80 between the two repos) |

The real shared set is almost certainly larger than 140 because the naive
static extractor used for that number doesn't always resolve Laravel's
nested `Route::prefix('admin')->group(...)` chains — an admin route
declared inside that group as `Route::get('/compliance/report')` becomes
`/api/v1/admin/compliance/report` at runtime, not the `/api/v1/compliance/report`
a grep-level script captures.

For that reason the numbers above are **upper bounds on drift**; the real
drift is smaller. The only way to get a ground-truth number is to run both
servers and compare the emitted OpenAPI JSON. `scripts/diff-openapi.sh`
does exactly that — see "Methodology" below.

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

## Known drift categories

With the current committed snapshots and the input-specific normalisation in
`scripts/diff-openapi.sh`, the parity diff is still materially open:

- Rust-only paths: `32`
- PHP-only paths: `108`

That means parity is **not** currently “just static-extractor noise”. The
remaining drift falls into four broad buckets:

### 1. Admin routing prefix chains (PHP side)

Many PHP admin endpoints use `Route::middleware('admin')->prefix('admin')->group(...)`
blocks. A naive static extractor captures just the inner path, making them
look like drift. **Effect on real parity: zero** — the routes exist, the
extractor just didn't see them correctly.

**Action**: rely on the Scramble JSON dump (runtime-accurate), not `grep`.

### 2. Genuine Rust-only contract surfaces

Rust still exposes paths the PHP contract does not currently publish, including
top-level operational surfaces (`/status`, `/health/detailed`, docs endpoints),
admin export/settings endpoints, booking QR under `/api/v1/bookings/{id}/qr`,
and the Rust-style payments/config surface.

**Action**: close these in small batches instead of one mega-port:
auth/profile/public aliases, health/docs surfaces, booking/payment aliases,
then admin/export/settings tails.

### 3. Genuine PHP-only contract surfaces

PHP still publishes a substantially larger surface, including legacy public auth
aliases (`/api/v1/login`, `/register`, `/refresh`), health/info aliases,
demo/discovery endpoints, broader admin analytics/settings/reporting routes,
and several booking/user convenience routes.

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
- **Alias tranche**: eliminate the cheap path mismatches first (`login/register/refresh`, health/detail, QR/payment/config, import aliases).
- **Feature tranche**: close the remaining admin/reporting/demo/user feature gaps or explicitly classify intentional divergences.
