# SOTA-2026 local dev setup

One-command bootstrap for parkhub-rust. Designed to be **adoptable**: copy this directory + `.mise.toml` + `.envrc` + `Justfile` + `bacon.toml` + `dprint.json` + `typos.toml` into any sister repo to get the same dev experience.

## Prerequisites (one-time, host-side)

Install **mise** + **direnv** + **just** + **lefthook** + **podman** on the host:

```bash
# Bazzite / Fedora Atomic — via brew (host-side)
flatpak-spawn --host /home/linuxbrew/.linuxbrew/bin/brew install mise direnv just lefthook podman
flatpak-spawn --host bash -lc 'mise activate bash >> ~/.bashrc'
flatpak-spawn --host bash -lc 'direnv hook bash >> ~/.bashrc'
```

(Replace `bash` with `zsh`/`fish` per your shell.)

## Per-repo bootstrap

Clone, then:

```bash
direnv allow .
just bootstrap
```

`just bootstrap` runs:

1. `mise install` — installs every tool pinned in `.mise.toml` (Rust 1.94.1, Node 22.12, sccache, mold, lefthook, typos, gitleaks, zizmor, osv-scanner, trivy, cargo-deny/audit/shear, bacon, dprint)
2. `lefthook install --force` — wires git hooks (pre-commit fmt/typos/actionlint, pre-push full local CI)
3. `cd parkhub-web && npm ci` — installs JS deps for the Astro + React frontend

**That's it.** Everything is on PATH, lefthook fires on commit/push, bacon/dprint/typos available as plain commands.

## Day-to-day

```bash
just              # show all recipes
just dev          # bacon TUI: live cargo check on every save
just web-dev      # astro dev server (parkhub-web)
just check        # fmt + clippy + lib tests (~30 s on warm cache)
just security     # full local security suite (Trivy + osv-scanner + cargo-audit + cargo-deny + gitleaks + zizmor + typos)
just fmt          # cargo fmt + dprint + biome (auto-fix)
just ci           # run lefthook pre-push gates manually before pushing
```

## What's wired

| Tool | Config | Purpose |
|---|---|---|
| `mise` | `.mise.toml` | toolchain pin (rust, node, etc.); one-command install |
| `direnv` | `.envrc` | auto-activate mise on `cd` |
| `just` | `Justfile` | task runner — single source of truth for dev commands |
| `bacon` | `bacon.toml` | live cargo check/clippy/test in a TUI |
| `dprint` | `dprint.json` | unified formatter for json/md/toml/yaml |
| `typos` | `typos.toml` | spell-check with project allow-list |
| `lefthook` | `lefthook.yml` | git hooks (pre-commit + pre-push gates) |
| `cargo-deny` | `deny.toml` | license + advisory + bans |
| `cargo-audit` | `audit.toml` (when present) | RustSec advisories with documented ignores |
| `osv-scanner` | `osv-scanner.toml` | multi-ecosystem SCA |

## CI parity

CI workflows (`.github/workflows/{ci,security,release}.yml`) run the same gates as local lefthook + Justfile recipes. The pattern: **what fails on `just ci` will fail on the GitHub Actions runner**.

Heavy CI workloads (cargo workspace check + e2e) are designed to forward to a desktop Podman runner via `runs-on: [self-hosted, fop-runner-rust]` — see `forge-operator/docs/plans/t-2374-plan.md`. Until that lands, all CI runs on GitHub Actions.

## License

Every tool above is MIT or Apache-2.0. No GPL/AGPL/BSL/SSPL/FSL deps — matches the workspace's commercial-safe license doctrine.

## Adopt this kit in another repo

Copy these 7 files (adjust toolchain pins for the new repo's stack):

```
.mise.toml
.envrc
Justfile
bacon.toml          # Rust repos only
dprint.json
typos.toml
dev/SETUP.md
```

Then `direnv allow .` + `just bootstrap`.

For PHP/Node/Python repos, swap `bacon.toml` for `air.toml` (Go), `mods/dev-loop` (Node), `pytest-watch` (Python), or whatever live-loop the stack supports.
