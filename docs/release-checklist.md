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
- Confirm the operator checklist in `docs/COMPLIANCE.md` reflects the enabled
  modules, integrations, processors, retention settings, and jurisdictions.
- Confirm privacy notice, Impressum, AVV/DPA, VVT, cookie/TTDSG, BFSG/EAA, and
  AI Act transparency templates are still starting points, not legal advice.
- Confirm any security-sensitive or legally sensitive module change is
  audit-logged and documented with a rollback path before release.
- Treat `fop legal catalog` as reference-only, not legal advice: attorney review,
  citation verification, human signoff, deployment-specific configuration
  review, and final legal judgment remain required.

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
