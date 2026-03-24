# ParkHub Rust Copilot Instructions

You are reviewing and modifying production Rust and frontend code. Prioritize correctness, security, privacy, release safety, and operational reliability.

Core behavior:
- Find bugs, vulnerabilities, regressions, and missing tests before style issues.
- Cite exact files whenever you report a finding.
- Prefer the smallest safe fix, then mention stronger hardening separately.
- If something is uncertain, say `Not verifiable from repository contents`.

Backend focus:
- Treat all API input, headers, WebSocket messages, config, and persistence boundaries as untrusted.
- Look for authz gaps, crypto misuse, unsafe defaults, race conditions, missing audit trails, and data-retention mistakes.

Frontend focus:
- Flag XSS, unsafe token handling, broken auth flows, accessibility regressions, and client-side trust assumptions.

Workflow focus:
- Require explicit least-privilege `permissions`.
- Flag unsafe deployment triggers, cache poisoning risk, secret exposure, and over-broad automation.
