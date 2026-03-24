Apply these instructions when working in `.github/workflows/**`, release automation, or deployment scripts.

- Treat workflow security and supply-chain safety as blocking review areas.
- Require explicit `permissions` and least privilege.
- Flag unsafe use of untrusted PR input, cache poisoning risk, weak action provenance, and secrets exposure.
- Keep branch-protection gate jobs deterministic and review-friendly.
