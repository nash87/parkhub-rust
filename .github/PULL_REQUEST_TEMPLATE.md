## Summary
<!-- Brief description of what this PR does and why -->

## Type of change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Security fix
- [ ] Dependency update
- [ ] Refactor / code quality

## Testing
- [ ] I tested locally with Docker Compose (`docker compose up -d`)
- [ ] I ran the test suite (`cargo test --workspace`)
- [ ] I ran Clippy (`cargo clippy --workspace -- -D warnings`)
- [ ] I ran frontend tests (`cd parkhub-web && npx vitest run`)
- [ ] I checked GDPR compliance impact (does this change affect data handling, storage, or erasure?)

## Security checklist
- [ ] No secrets, credentials, or API keys committed
- [ ] New endpoints have appropriate auth guards and rate limiting
- [ ] User input is validated before use
- [ ] No new `unsafe` blocks without justification

## Checklist
- [ ] Code follows project style (Rust fmt, no Clippy warnings, pedantic + nursery)
- [ ] Documentation updated if needed (docs/, README.md)
- [ ] CHANGELOG.md entry added
- [ ] New endpoints are covered by OpenAPI annotations (`#[utoipa::path]`)
- [ ] Feature-gated behind appropriate `mod-*` feature flag (if applicable)
