---
title: "Specs — Spec-Driven Development index"
type: reference
status: active
last_reviewed: 2026-06-09
---

# Specs

This directory contains spec-driven development artifacts for ParkHub Rust features.
Each feature gets its own subdirectory with three files:

```
specs/<feature-id>/
├── spec.md    — Requirements and user stories (what + why)
├── plan.md    — Technical design and architecture (how)
└── tasks.md   — Dependency-ordered task list with acceptance criteria
```

Templates for each file live in [`.specify/templates/`](../.specify/templates/).

---

## Workflow

1. **Before opening a PR for a non-trivial feature**, create `specs/<feature-id>/spec.md`
   using the template. Open a draft PR so reviewers can comment on requirements before
   implementation begins.

2. **After the spec is agreed**, add `plan.md` with the technical design. Get maintainer
   sign-off before writing the bulk of the code.

3. **Use `tasks.md`** to track implementation slices. Each task maps to one PR or one
   focused working session.

4. **Link the spec from the GitHub issue or PR** using the Spec reference field in the
   PR template.

For small bug fixes and documentation changes, no spec is required.
For features labeled `enhancement` or `feature`, a spec is expected.

---

## Relationship to the nido task lifecycle

ParkHub uses the nido task system for internal work tracking. The relationship is:

- `nido_task: T-xxx` in the spec/plan/tasks frontmatter links the spec artifact to the
  task board entry.
- `nido tasks checkpoint T-xxx "spec authored at specs/<id>/spec.md"` records progress
  for cross-session handoff.
- The spec files are the public-facing artifact; the nido task is the internal tracking
  record. Both are maintained in parallel.

---

## Existing plans

Pre-spec-kit planning documents live in [`docs/plans/`](../docs/plans/). They follow an
earlier convention and are not being migrated retroactively. New features use the
`specs/<feature-id>/` layout.

---

## Index

| Feature ID | Title | Status |
|------------|-------|--------|
| *(no specs yet — add the first one!)* | | |
