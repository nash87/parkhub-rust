Apply these instructions when working in `parkhub-common/**`, `parkhub-server/**`, `parkhub-client/**`, and backend test code.

- Prioritize authn/authz, crypto safety, persistence correctness, and operational resilience.
- Flag any place where user input crosses trust boundaries without validation or authorization.
- Treat GDPR/privacy paths, audit logging, and destructive actions as high-risk.
- Require tests for negative paths, permission failures, and data-integrity-sensitive changes.
- Prefer deterministic error handling, timeouts, and bounded resource usage.
