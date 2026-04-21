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
- Review any remaining runtime-sensitive gaps and make sure they are documented.
- Do not silently introduce new shared-frontend branching requirements.

## Quality bar

- Required CI is green.
- The release version parity gate passes without overrides.
- Release workflow uses the same pinned toolchain/runtime assumptions as CI.
- Install/download instructions match the actual published artifacts.

## Cross-repo discipline

- If this release changes a shared customer-visible feature, verify whether
  `parkhub-php` needs a matching change.
- If parity is not yet closed, record the gap explicitly in release notes.
- Push order remains `origin` first, then `github`.
