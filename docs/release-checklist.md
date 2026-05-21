# Release Checklist

Use this before tagging a ParkHub release from this repo.

## Product truth

- README, `docs/API.md`, and `docs/FEATURES.md` agree on the shipped contract.
- `docs/parity-governance.md` still matches how the release is being cut.
- `docs/openapi-parity.md` reflects the current Rust vs PHP state.
- `refs/tags/v*`, `Cargo.toml` workspace version, root `package.json`, and
  `parkhub-web/package.json` all match exactly before the tag is pushed.

## Contract and parity

- Regenerate and commit the local OpenAPI snapshot when the contract changed.
- Run `scripts/tests/test-legal-openapi-contract.sh` after changes to legal,
  compliance, module, plugin, export, erasure, or privacy surfaces.
- Review any remaining runtime-sensitive gaps and make sure they are documented.
- Do not silently introduce new shared-frontend branching requirements.

## Legal readiness

- Run `scripts/tests/test-legal-readiness-wording.sh`; public docs must describe
  deployment-dependent readiness, not absolute legal compliance.
- Review `docs/legal-readiness.md` as the operator-facing audit hub for German,
  EU, and international readiness obligations.
- Complete or update `docs/deployment-readiness-record.md` for the target
  deployment before production use, business use, or customer-facing evaluation.
- Review `docs/legal-readiness-parity.md` for Rust/PHP legal-readiness parity
  when a change affects shared legal, privacy, module, plugin, or release policy.
- Confirm the operator checklist in `docs/COMPLIANCE.md` reflects the enabled
  modules, integrations, processors, retention settings, and jurisdictions.
- Confirm privacy notice, Impressum, AVV/DPA, VVT, cookie/TTDSG, BFSG/EAA, and
  AI Act transparency templates are still starting points, not legal advice.
- Confirm any security-sensitive or legally sensitive module change is
  audit-logged and documented with a rollback path before release.
- Treat the Nido/fop legal catalog service (current CLI entrypoint:
  `fop legal catalog --json`; `nido legal` is not exposed by the installed Nido
  CLI yet) as reference-only, not legal advice: attorney review, citation
  verification, human signoff, deployment-specific configuration review, and
  final legal judgment remain required.
- Capture the current legal catalog `source_revision`, `generated_at`,
  `requires_attorney_review`, `requires_human_signoff`, `execution_allowed`, and
  `safety_boundary` values in the deployment readiness record before release.
- For the current `exact_cover_v1` release candidate, review
  `docs/release-evidence/2026-05-21-exact-cover-nido-legal-catalog.md` and
  replace it with a fresh capture if the head SHA, legal catalog revision, or
  deployment target changes before release.

## Quality bar

- Required CI is green.
- The release version parity gate passes without overrides.
- Release workflow uses the same pinned toolchain/runtime assumptions as CI.
- Install/download instructions match the actual published artifacts.

## Cross-repo discipline

- If this release changes a shared customer-visible feature, verify whether
  `parkhub-php` needs a matching change.
- If parity is not yet closed, record the gap explicitly in release notes.
- GitHub `nash87/parkhub-rust` remains the CI/review source of truth. Do not
  base releases on a stale Gitea mirror.
