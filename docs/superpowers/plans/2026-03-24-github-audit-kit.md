# ParkHub Rust GitHub Audit Kit Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a complete GitHub Copilot and GitHub Security audit kit to the ParkHub Rust workspace without conflicting with the existing CI and release automation.

**Architecture:** Keep the existing release and security workflows, but add a standardized Copilot guidance layer, focused custom agents, and upgrade CodeQL and dependency review for the mixed Rust and frontend workspace. Copilot setup steps should provision both Rust and Node workflows.

**Tech Stack:** Rust workspace, Axum, npm, Astro/React frontend, GitHub Actions, CodeQL, Dependabot, GitHub Copilot coding agent.

---

## Chunk 1: Audit Guidance Files

### Task 1: Add Copilot guidance

**Files:**
- Create: `.github/copilot-instructions.md`
- Create: `.github/instructions/backend.instructions.md`
- Create: `.github/instructions/frontend.instructions.md`
- Create: `.github/instructions/tests.instructions.md`
- Create: `.github/instructions/workflows.instructions.md`
- Modify: `AGENTS.md`

- [ ] Add repository-wide audit guidance.
- [ ] Add path-specific instructions for Rust backend, embedded frontend, tests, and workflows.
- [ ] Update root `AGENTS.md` so Copilot agents get explicit audit and security expectations.

### Task 2: Add custom GitHub Copilot agents

**Files:**
- Create: `.github/agents/repo-auditor.agent.md`
- Create: `.github/agents/security-auditor.agent.md`
- Create: `.github/agents/release-readiness.agent.md`

- [ ] Define custom agents for broad audit, security review, and ship readiness.

## Chunk 2: GitHub Automation

### Task 3: Upgrade GitHub automation

**Files:**
- Modify: `.github/workflows/codeql.yml`
- Create: `.github/workflows/dependency-review.yml`
- Create: `.github/workflows/copilot-setup-steps.yml`
- Modify: `.github/dependabot.yml`

- [ ] Expand CodeQL to Rust and JavaScript/TypeScript.
- [ ] Add dependency review for pull requests.
- [ ] Add Copilot setup steps that install Rust, Node, and workspace dependencies.
- [ ] Expand Dependabot coverage for Cargo, npm, and Actions.
