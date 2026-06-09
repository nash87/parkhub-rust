---
id: "{{feature-id}}"
title: "{{Feature Title}}"
type: tasks
status: draft
plan: "specs/{{feature-id}}/plan.md"
created: {{YYYY-MM-DD}}
author: "@{{github-handle}}"
nido_task: "T-{{task-id}}"
---

# Tasks: {{Feature Title}}

> **What this is:** Dependency-ordered, acceptance-criteria-linked task list.
> Each task should be completable in a single PR or a short working session.

---

## Status legend

| Symbol | Meaning |
|--------|---------|
| `[ ]` | Not started |
| `[~]` | In progress |
| `[x]` | Done |
| `[-]` | Skipped / deferred |

---

## Task list

### Phase 1: Foundation

- [ ] **T1** — {{Description}}
  - AC: {{Acceptance criterion from spec.md}}
  - Blocks: T2, T3
  - Estimate: S / M / L
  - Branch: `feature/{{feature-id}}-t1`

- [ ] **T2** — {{Description}}
  - AC: {{Acceptance criterion}}
  - Depends on: T1
  - Estimate: S / M / L

### Phase 2: Feature

- [ ] **T3** — {{Description}}
  - AC: {{Acceptance criterion}}
  - Depends on: T1
  - Estimate: S / M / L

### Phase 3: Polish

- [ ] **T4** — Add / update OpenAPI annotations and run `make drift`
  - AC: `make drift` exits 0; `docs/openapi/rust.json` committed
  - Depends on: T3

- [ ] **T5** — Write or update Vitest + Playwright tests
  - AC: `make frontend` green; new test covers the happy path and one error path

- [ ] **T6** — Update docs (API.md, CONFIGURATION.md, or CHANGELOG.md as applicable)
  - AC: docs updated, no broken links

---

## Dependency graph

```
T1 → T2 → T3 → T4
          ↓
          T5
          ↓
          T6
```

---

## Definition of done

A task is done when:
- Code is merged to `main`
- `make ci` is green (fmt + clippy + check + test + frontend + drift)
- The linked acceptance criterion from spec.md is met
- CHANGELOG.md has an entry under `[Unreleased]`

The feature is done when all tasks are `[x]` and the spec's acceptance criteria are
all met on `main`.
