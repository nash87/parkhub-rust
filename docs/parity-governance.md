# Cross-Runtime Parity Governance

ParkHub ships as one product with two maintained runtimes:

- `parkhub-rust` for the canonical machine contract and Rust-first deployment.
- `parkhub-php` for Laravel/shared-hosting/container deployment.

The goal is not "roughly equivalent." The goal is **one customer-visible
feature set with explicitly documented runtime-sensitive exceptions**.

## Canonical ownership

- **Product behaviour source of truth**: Rust module registry and shared product
  docs. If a feature is part of the ParkHub product contract, it must be
  reflected in Rust first or called out as runtime-sensitive.
- **Machine contract source of truth**: Rust OpenAPI output. Until a dedicated
  `docs/openapi/canonical.json` lands, treat `docs/openapi/rust.json` plus
  [openapi-parity.md](openapi-parity.md) as the canonical contract.
- **PHP responsibility**: mirror the same customer-visible contract, or record
  the gap explicitly in `docs/openapi-parity.md` and release notes.

## What must stay in sync

- Auth and session semantics exposed to clients.
- Shared REST paths, payload envelopes, and error codes.
- Module toggle/config surfaces used by the shared frontend.
- Public README / API / FEATURES claims about what ParkHub ships.

## Allowed differences

Allowed differences are implementation details, not product drift:

- storage engine
- framework/library choices
- packaging/distribution model
- deployment ergonomics

Customer-visible differences are only acceptable when all three are true:

1. the surface is marked runtime-sensitive in public docs,
2. the gap is captured in `docs/openapi-parity.md`,
3. the release checklist calls it out before tag/release.

## PR rules

If a PR changes a shared feature, route, auth/session behaviour, module
contract, or public product claim, it must also:

- update `docs/openapi-parity.md`,
- update the relevant README / API / FEATURES docs,
- note whether the sibling runtime is already aligned,
- or leave an explicit, reviewable parity gap note.

## Release rule

No release should claim "same feature set" unless README, API docs, FEATURES
docs, and the parity doc all agree.
Release tags must also match the checked-in version surfaces in
`Cargo.toml`, root `package.json`, and `parkhub-web/package.json`.
