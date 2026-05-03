# Changelog

All notable changes to ParkHub Rust are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---
## [5.0.9] - 2026-05-03

### Added

- Cargo build timeline visualization (--timings=html wrapper) (#533) ([#533](https://github.com/nash87/parkhub-rust/pull/533))
- Cargo dep hygiene — machete + sort in full/cd profiles (#531) ([#531](https://github.com/nash87/parkhub-rust/pull/531))
- Unified Rust + frontend coverage report (cargo-llvm-cov + vitest) (#528) ([#528](https://github.com/nash87/parkhub-rust/pull/528))
- SLSA-3 reproducibility check (build twice + hash compare) (#527) ([#527](https://github.com/nash87/parkhub-rust/pull/527))
- Release-rehearsal + visual-regression local mirrors (#523) ([#523](https://github.com/nash87/parkhub-rust/pull/523))
- Mutation testing + fuzz smoke local mirrors (#522) ([#522](https://github.com/nash87/parkhub-rust/pull/522))
- Wire fop ci-audit + workflow-drift detector into Stage 2 (#521) ([#521](https://github.com/nash87/parkhub-rust/pull/521))
- Workflow drift detector — gitea ↔ github sync watcher (#519) ([#519](https://github.com/nash87/parkhub-rust/pull/519))
- Prebuilt image publish workflow → ghcr.io (10× faster first-time devcontainer up) (#518) ([#518](https://github.com/nash87/parkhub-rust/pull/518))
- Local install-smoke + multi-browser E2E runners (#517) ([#517](https://github.com/nash87/parkhub-rust/pull/517))
- Supply-chain visibility — local SBOM + Scorecard + gitea ci.yml --add-host fix (#516) ([#516](https://github.com/nash87/parkhub-rust/pull/516))
- Full local CI/CD toolchain in a single dev container (#515) ([#515](https://github.com/nash87/parkhub-rust/pull/515))
- Lighthouse + --background flag (PR-C / Slices 5+6, ladder complete) (#514) ([#514](https://github.com/nash87/parkhub-rust/pull/514))
- Change-gated local container image scan — PR-B / Slice 4 of 6 (#513) ([#513](https://github.com/nash87/parkhub-rust/pull/513))
- Close GHA→local gap — yamllint + typos + helm-validate + cargo-audit + cargo-geiger (PR-A of 3) (#512) ([#512](https://github.com/nash87/parkhub-rust/pull/512))


### CI

- Bot-friendly + non-spammy + professional workflow polish (#520) ([#520](https://github.com/nash87/parkhub-rust/pull/520))
- Make mutants target — Rust counterpart of parkhub-php#436 (#492) ([#492](https://github.com/nash87/parkhub-rust/pull/492))
- Keep changelog workflow green without bot token (#485) ([#485](https://github.com/nash87/parkhub-rust/pull/485))
- Open changelog regeneration PRs (#484) ([#484](https://github.com/nash87/parkhub-rust/pull/484))
- Repin git-cliff changelog action (#483) ([#483](https://github.com/nash87/parkhub-rust/pull/483))
- Require explicit fop local attestation (#481) ([#481](https://github.com/nash87/parkhub-rust/pull/481))
- Make Trivy FS SARIF upload advisory (rate-limit fix) (#477) ([#477](https://github.com/nash87/parkhub-rust/pull/477))
- Local OSS mirror of security.yml (T-2268, parity with parkhub-php) (#470) ([#470](https://github.com/nash87/parkhub-rust/pull/470))
- Use --format templating for manifest digest (#464) ([#464](https://github.com/nash87/parkhub-rust/pull/464))


### Changed

- HeroEyebrow icon prop now optional — sweep 3 iconless usages (#532) ([#532](https://github.com/nash87/parkhub-rust/pull/532))
- Full sweep — migrate 40 pages to <HeroEyebrow> component (#526) ([#526](https://github.com/nash87/parkhub-rust/pull/526))
- Extract <HeroEyebrow> component + 3-page PoC migration (#525) ([#525](https://github.com/nash87/parkhub-rust/pull/525))
- Extract <V11Meter> component — single source of truth for stat-card chrome (#524) ([#524](https://github.com/nash87/parkhub-rust/pull/524))


### Chore

- Expand short SHAs in `--sha` to full 40-char form (#583) ([#583](https://github.com/nash87/parkhub-rust/pull/583))
- Replace 3 `any` request bodies with explicit interface types (#581) ([#581](https://github.com/nash87/parkhub-rust/pull/581))
- Backtick config/discovery/demo doc identifiers (#579) ([#579](https://github.com/nash87/parkhub-rust/pull/579))
- Add `--sha` flag + state validation to post-attestation-deferred.sh (#582) ([#582](https://github.com/nash87/parkhub-rust/pull/582))
- Backtick X-API-Key/bucket_label/X-RateLimit-Bucket (#578) ([#578](https://github.com/nash87/parkhub-rust/pull/578))
- Backtick OpenAPI/WhatsApp in doc comments (#577) ([#577](https://github.com/nash87/parkhub-rust/pull/577))
- Fix 3 clippy::doc_markdown warnings in api/mod.rs (#576) ([#576](https://github.com/nash87/parkhub-rust/pull/576))
- Fix 10 clippy::doc_markdown warnings (jobs + db/bookings + circuit_breaker) (#575) ([#575](https://github.com/nash87/parkhub-rust/pull/575))
- Tighten 7 any types — SOTA-2026 type safety (#574) ([#574](https://github.com/nash87/parkhub-rust/pull/574))
- Wrap 2 SidebarV3 hardcoded strings in t() (#573) ([#573](https://github.com/nash87/parkhub-rust/pull/573))
- Fix the 1 doc_markdown warning #570 missed (#571) ([#571](https://github.com/nash87/parkhub-rust/pull/571))
- Fix 5 clippy::doc_markdown warnings (ParkHub backticks) (#570) ([#570](https://github.com/nash87/parkhub-rust/pull/570))
- Fix 17 clippy::doc_markdown warnings (co2 + api_docs + admin_ext) (#569) ([#569](https://github.com/nash87/parkhub-rust/pull/569))
- Fix 5 clippy::doc_markdown warnings (#568) ([#568](https://github.com/nash87/parkhub-rust/pull/568))
- Wrap final 3 hardcoded strings in t() (LoginHistory + AdminAnalytics) (#565) ([#565](https://github.com/nash87/parkhub-rust/pull/565))
- Wrap AdminAnalytics hardcoded strings in t() (i18n + test mock) (#564) ([#564](https://github.com/nash87/parkhub-rust/pull/564))
- Remove obsolete @ts-ignore on JSON import (#563) ([#563](https://github.com/nash87/parkhub-rust/pull/563))
- Add aria-label to 3 SidebarV3 buttons (a11y) (#562) ([#562](https://github.com/nash87/parkhub-rust/pull/562))
- Add aria-label to 13 row-action icon buttons (a11y) (#561) ([#561](https://github.com/nash87/parkhub-rust/pull/561))
- Add aria-label to 7 icon-only buttons (a11y) (#560) ([#560](https://github.com/nash87/parkhub-rust/pull/560))
- Astro check sweep — 33 → 8 hints (-25, 76% reduction) (#549) ([#549](https://github.com/nash87/parkhub-rust/pull/549))
- Clean Icon-suffix leakage from 9 source comments (#555) ([#555](https://github.com/nash87/parkhub-rust/pull/555))
- Zod v4 — z.string().email() → z.email() (-2 deprecation hints) (#552) ([#552](https://github.com/nash87/parkhub-rust/pull/552))
- Remove 3 unused TS type-only named imports (#543) ([#543](https://github.com/nash87/parkhub-rust/pull/543))
- Prefix 8 more unused mock-callback params (round 2 — handles `opts?:`) (#542) ([#542](https://github.com/nash87/parkhub-rust/pull/542))
- Prefix 11 unused mock-callback params with `_` (TS convention) (#541) ([#541](https://github.com/nash87/parkhub-rust/pull/541))
- Remove 10 unused named imports flagged by ts(6133) (#540) ([#540](https://github.com/nash87/parkhub-rust/pull/540))
- Remove 23 stale `import React` lines from test files (React 17+ JSX) (#539) ([#539](https://github.com/nash87/parkhub-rust/pull/539))
- Exclude coverage/ from astro check + gitignore (#538) ([#538](https://github.com/nash87/parkhub-rust/pull/538))
- Replace deprecated FormEvent with SyntheticEvent (React 19 path) (#537) ([#537](https://github.com/nash87/parkhub-rust/pull/537))
- Data-driven deprecated phosphor sweep — 994 → 95 hints (-899) (#535) ([#535](https://github.com/nash87/parkhub-rust/pull/535))
- Wire workflow-drift + image-scan into pre-push (file-glob gated) (#529) ([#529](https://github.com/nash87/parkhub-rust/pull/529))
- Close 2 SOTA-2026 a11y/perf gaps — WCAG 2.2 tags + Lighthouse INP threshold (#510) ([#510](https://github.com/nash87/parkhub-rust/pull/510))
- Rename 31 deprecated phosphor icons → IconName variants + fix de.ts dupes (#508) ([#508](https://github.com/nash87/parkhub-rust/pull/508))
- SOTA-2026 local dev kit (mise + just + bacon + dprint + typos) (#478) ([#478](https://github.com/nash87/parkhub-rust/pull/478))
- Drop COSIGN_EXPERIMENTAL=true (no-op since cosign 3.x) (#471) ([#471](https://github.com/nash87/parkhub-rust/pull/471))


### Dependencies

- Cargo-machete metadata.ignored for documented false positives (#534) ([#534](https://github.com/nash87/parkhub-rust/pull/534))
- Override yaml to ^2.8.3 — CVE-2026-33532 (#475) ([#475](https://github.com/nash87/parkhub-rust/pull/475))


### Documentation

- Document SOTA-2026 local CI mirrors + dev container (#544) ([#544](https://github.com/nash87/parkhub-rust/pull/544))
- Comprehensive local CI/CD script index (13 mirrors + invariants) (#530) ([#530](https://github.com/nash87/parkhub-rust/pull/530))
- Cosign-verify quickstart (#461) ([#461](https://github.com/nash87/parkhub-rust/pull/461))


### Fixed

- Disable lot selector + drop lot_id from PUT body on edit (T-2652) (#584) ([#584](https://github.com/nash87/parkhub-rust/pull/584))
- 3 v11-rollout aftershocks (label leak + act type + refresh aria) (#558) ([#558](https://github.com/nash87/parkhub-rust/pull/558))
- Add aria-label to bulk buttons (a11y + 4 tests) (#556) ([#556](https://github.com/nash87/parkhub-rust/pull/556))
- Icon rename leaked into 4 string-literal fallbacks (#554) ([#554](https://github.com/nash87/parkhub-rust/pull/554))
- Phosphor mock cascade v3 — 102 admin tests resurrected (#553) ([#553](https://github.com/nash87/parkhub-rust/pull/553))
- Render user count badge in table column header (#550) ([#550](https://github.com/nash87/parkhub-rust/pull/550))
- Inject missing motion.section mock — 96 tests resurrected across 8 files (#551) ([#551](https://github.com/nash87/parkhub-rust/pull/551))
- Render filtered lot count badge in table header (#547) ([#547](https://github.com/nash87/parkhub-rust/pull/547))
- Rename phosphor icon mocks in 69 test files to match post-deprecation IconName (#545) ([#545](https://github.com/nash87/parkhub-rust/pull/545))
- Update Gitea workflow IPs 192.168.178.212 → 192.168.178.233 (T-2501) (#488) ([#488](https://github.com/nash87/parkhub-rust/pull/488))
- Allow dev http cookies (#487) ([#487](https://github.com/nash87/parkhub-rust/pull/487))
- Allow OpenStreetMap tiles in CSP (#486) ([#486](https://github.com/nash87/parkhub-rust/pull/486))
- Correct Cargo.toml version parse in release-preview (#480) ([#480](https://github.com/nash87/parkhub-rust/pull/480))
- Correct download-artifact version comment v7 → v8 (#476) ([#476](https://github.com/nash87/parkhub-rust/pull/476))
- Bump fop local CI attestation statuses:read -> write (#474) ([#474](https://github.com/nash87/parkhub-rust/pull/474))
- Tighten dashboard and reports parity (#473) ([#473](https://github.com/nash87/parkhub-rust/pull/473))
- /sw.js + non-hashed root assets must be no-cache (v4.15.0 trap) (#469) ([#469](https://github.com/nash87/parkhub-rust/pull/469))
- Cosign-verify uses --type spdxjson (matches docker-publish) (#468) ([#468](https://github.com/nash87/parkhub-rust/pull/468))
- Focus-visible utility, scoped transitions, link-based bottom nav (#467) ([#467](https://github.com/nash87/parkhub-rust/pull/467))
- Heatmap ARIA grid wrapping + breadcrumb contrast (#465) ([#465](https://github.com/nash87/parkhub-rust/pull/465))


### Security

- SOTA-2026 batch4 — Analytics+Dashboard+Settings+Roles+SSO+Zones+DataMgmt+RateLimits (19/24) (#497) ([#497](https://github.com/nash87/parkhub-rust/pull/497))


### Tests

- Add 8 property tests for is_valid_email rejection paths
- Expand property_roundtrip 35 → 39 (+4) — close last 2 enum gaps (#567) ([#567](https://github.com/nash87/parkhub-rust/pull/567))
- Expand property_roundtrip 19 → 35 (+16) for 8 domain enums (#566) ([#566](https://github.com/nash87/parkhub-rust/pull/566))
- Add 16 property tests for src/validation.rs (#559) ([#559](https://github.com/nash87/parkhub-rust/pull/559))
- Expand validation property tests 7 → 20 (boundary + adversarial) (#548) ([#548](https://github.com/nash87/parkhub-rust/pull/548))
- Relax locales.test.ts to allow DE-superset (mirrors #546) (#557) ([#557](https://github.com/nash87/parkhub-rust/pull/557))
- Allow DE to be a superset of EN (eyebrow labels added in #505-#511) (#546) ([#546](https://github.com/nash87/parkhub-rust/pull/546))
- Expand property roundtrip suite — +12 tests for lifecycle enums (#536) ([#536](https://github.com/nash87/parkhub-rust/pull/536))
- Stabilize WebKit E2E navigation (#482) ([#482](https://github.com/nash87/parkhub-rust/pull/482))


### Design

- SOTA-2026 v11 hero on 6 feature pages (UF batch5 — 19/24) (#504) ([#504](https://github.com/nash87/parkhub-rust/pull/504))
- SOTA-2026 v11 hero on Team + Leaderboard + Notifications + Favorites (UF batch4 — 13/24) (#503) ([#503](https://github.com/nash87/parkhub-rust/pull/503))
- SOTA-2026 v11 hero on Vehicles + Credits + Absences (UF batch3 — 9/24) (#502) ([#502](https://github.com/nash87/parkhub-rust/pull/502))
- SOTA-2026 v11 hero on Book + Calendar + MapView (UF batch2 — 6/24) (#501) ([#501](https://github.com/nash87/parkhub-rust/pull/501))
- SOTA-2026 v11 hero on Profile + Settings + Bookings (UF batch1) (#500) ([#500](https://github.com/nash87/parkhub-rust/pull/500))
- SOTA-2026 batch6 — Reports/Modules/ScheduledReports stragglers (true 26/26) (#499) ([#499](https://github.com/nash87/parkhub-rust/pull/499))
- SOTA-2026 batch5 — Plugins+Webhooks+Translations+Updates (24/24 complete) (#498) ([#498](https://github.com/nash87/parkhub-rust/pull/498))
- SOTA-2026 batch3 — Users + Lots + Fleet + Tenants (#496) ([#496](https://github.com/nash87/parkhub-rust/pull/496))
- SOTA-2026 batch2 — Announcements + Compliance + AuditLog (#495) ([#495](https://github.com/nash87/parkhub-rust/pull/495))
- SOTA-2026 batch1 — Accessible + Maintenance + 2 tone variants + warn-banner (#494) ([#494](https://github.com/nash87/parkhub-rust/pull/494))
- SOTA-2026 hero + meters with emerald tone (#493) ([#493](https://github.com/nash87/parkhub-rust/pull/493))
- SOTA-2026 sidebar nav pill (slice 3/3 — final) (#491) ([#491](https://github.com/nash87/parkhub-rust/pull/491))
- SOTA-2026 stat cards → v11 metric meters (slice 2/3) (#490) ([#490](https://github.com/nash87/parkhub-rust/pull/490))
- SOTA-2026 hero card with v11 chrome — first slice (#489) ([#489](https://github.com/nash87/parkhub-rust/pull/489))


### I18n

- Broad sweep of 67 ASCII-only umlaut typos in DE locale (#511) ([#511](https://github.com/nash87/parkhub-rust/pull/511))
- Fix 17 missing-umlaut typos in DE locale (Parkplätze, Ankündigungen, Für Reset) (#509) ([#509](https://github.com/nash87/parkhub-rust/pull/509))
- Admin sidebar nav + group headers + OPERATIONAL FOCUS card (#507) ([#507](https://github.com/nash87/parkhub-rust/pull/507))
- Admin v11 hero eyebrow keys + 4 hardcoded eyebrows refactored to t() (#506) ([#506](https://github.com/nash87/parkhub-rust/pull/506))
- V11 hero eyebrow keys for 19 user-facing pages (#505) ([#505](https://github.com/nash87/parkhub-rust/pull/505))


## [5.0.8] - 2026-04-29

### CI

- Native amd64 + arm64 split + manifest list (#462) ([#462](https://github.com/nash87/parkhub-rust/pull/462))
- Make SBOM cosign attestation advisory (cosign 3.x compat) (#459) ([#459](https://github.com/nash87/parkhub-rust/pull/459))


## [5.0.7] - 2026-04-29

### CI

- Make Attest provenance advisory in release.yml + docker-publish.yml (#457) ([#457](https://github.com/nash87/parkhub-rust/pull/457))


## [5.0.6] - 2026-04-29

### CI

- Release-container profile (thin LTO) to fit 90-min timeout (#455) ([#455](https://github.com/nash87/parkhub-rust/pull/455))
- Drop ARM64 — QEMU exceeds 90-min timeout (#454) ([#454](https://github.com/nash87/parkhub-rust/pull/454))
- Dormant deploy.yml (render + fly + koyeb, T-2272 Phase B) (#453) ([#453](https://github.com/nash87/parkhub-rust/pull/453))
- Add release-rehearsal.yml (#452) ([#452](https://github.com/nash87/parkhub-rust/pull/452))


## [5.0.5] - 2026-04-29

### CI

- Cosign sign-blob + SPDX SBOMs for release archives (#443) ([#443](https://github.com/nash87/parkhub-rust/pull/443))
- Tauri desktop installers + multi-arch container + PWA prep (#441) ([#441](https://github.com/nash87/parkhub-rust/pull/441))


### Chore

- Add app-version drift guard (#442) ([#442](https://github.com/nash87/parkhub-rust/pull/442))


### Fixed

- Regenerate icons as 8-bit PNG + multi-res ICO (#451) ([#451](https://github.com/nash87/parkhub-rust/pull/451))
- Add icon.png for tauri::generate_context!() (#450) ([#450](https://github.com/nash87/parkhub-rust/pull/450))
- Cosign 3.x needs explicit --bundle flag (#449) ([#449](https://github.com/nash87/parkhub-rust/pull/449))
- BeforeBuildCommand relative to repo root, not projectPath (#447) ([#447](https://github.com/nash87/parkhub-rust/pull/447))
- Point tauri.conf.json version at package.json (#446) ([#446](https://github.com/nash87/parkhub-rust/pull/446))
- Register service worker + use PNG apple-touch-icon (#444) ([#444](https://github.com/nash87/parkhub-rust/pull/444))
- Fop-local-ci.sh auto-fallback to direct mode when fop missing (#440) ([#440](https://github.com/nash87/parkhub-rust/pull/440))


## [5.0.3] - 2026-04-29

### CI

- SOTA-2026 pipeline + fop attestation + zizmor ERROR fixes (#437) ([#437](https://github.com/nash87/parkhub-rust/pull/437))
- Make SARIF upload advisory (#431) ([#431](https://github.com/nash87/parkhub-rust/pull/431))
- Avoid metadata API rate limit (#430) ([#430](https://github.com/nash87/parkhub-rust/pull/430))


### Dependencies

- Bump reqwest from 0.13.2 to 0.13.3 in the rust-deps group (#436) ([#436](https://github.com/nash87/parkhub-rust/pull/436))
- Bump the github-actions group with 2 updates (#434) ([#434](https://github.com/nash87/parkhub-rust/pull/434))
- Bump the npm-deps group in /parkhub-web with 6 updates (#432) ([#432](https://github.com/nash87/parkhub-rust/pull/432))


### Fixed

- Delete orphan design-v5/Sidebar.tsx + bump v5.0.3 (#439) ([#439](https://github.com/nash87/parkhub-rust/pull/439))


## [5.0.1] - 2026-04-26

### Added

- Full customization framework — settings + sidebar variants + density + fonts + feature toggles (Rust mirror) (#392) ([#392](https://github.com/nash87/parkhub-rust/pull/392))
- Lokal badge + Vorschläge eyebrow + privacy footer (#393) ([#393](https://github.com/nash87/parkhub-rust/pull/393))
- Local-first CI workflow (Lefthook + drift gates + Biome + native auto-merge) (#395) ([#395](https://github.com/nash87/parkhub-rust/pull/395))
- Tier-2 polish — conflict check, iCal button, PDF export, undo, filter persist (#387) ([#387](https://github.com/nash87/parkhub-rust/pull/387))
- Tier-1 2026 UX quick-wins (T-1977) (#386) ([#386](https://github.com/nash87/parkhub-rust/pull/386))
- Export FleetEvent via ts-rs + harden stray-bindings cleanup (#380) ([#380](https://github.com/nash87/parkhub-rust/pull/380))
- Upgrade Analytics bar chart to uPlot canvas (#379) ([#379](https://github.com/nash87/parkhub-rust/pull/379))
- SSE fleet events for Einchecken/EV/Tausch (#378) ([#378](https://github.com/nash87/parkhub-rust/pull/378))
- Auto-generate TypeScript types from Rust via ts-rs (#377) ([#377](https://github.com/nash87/parkhub-rust/pull/377))
- Wave 4+5 — port 11 admin screens (26/26 parity with PHP #337) (#376) ([#376](https://github.com/nash87/parkhub-rust/pull/376))
- Wave 3 — port 7 Fleet screens (parity with PHP #333) (#374) ([#374](https://github.com/nash87/parkhub-rust/pull/374))
- Wave 2 — port Buchen/Kalender/Karte/Profil (parity with PHP) (#373) ([#373](https://github.com/nash87/parkhub-rust/pull/373))
- V5 user-core screens (#371) ([#371](https://github.com/nash87/parkhub-rust/pull/371))
- V5 follow-up (#370) ([#370](https://github.com/nash87/parkhub-rust/pull/370))
- V5 follow-up (#369) ([#369](https://github.com/nash87/parkhub-rust/pull/369))


### CI

- Add typos + zizmor as advisory CI checks (Wave 5b) (#428) ([#428](https://github.com/nash87/parkhub-rust/pull/428))
- Backport --resource-profile pattern from parkhub-php (#385) (#427) ([#427](https://github.com/nash87/parkhub-rust/pull/427))
- Cutting-edge 2026 local-first CI/CD pipeline (#425) ([#425](https://github.com/nash87/parkhub-rust/pull/425))
- Pilot fop local-first PR attestation (#423) ([#423](https://github.com/nash87/parkhub-rust/pull/423))
- Bump trivy-action 0.35.0 → v0.36.0 (verified clean) (#408) ([#408](https://github.com/nash87/parkhub-rust/pull/408))
- Close silent-pass holes in typecheck-ts + vitest (#412) ([#412](https://github.com/nash87/parkhub-rust/pull/412))
- Dependabot cooldown + tailwind 4.2.3 ignore (#409) ([#409](https://github.com/nash87/parkhub-rust/pull/409))
- Swap trufflehog (AGPL) for gitleaks (MIT) (#403) ([#403](https://github.com/nash87/parkhub-rust/pull/403))
- Add actions language analysis (#402) ([#402](https://github.com/nash87/parkhub-rust/pull/402))
- Pin past tailwind 4.2.4 vite regression (#404) ([#404](https://github.com/nash87/parkhub-rust/pull/404))
- Unblock Render demo deploy (#368) ([#368](https://github.com/nash87/parkhub-rust/pull/368))


### Changed

- Harden useDraftFromActive edge cases (#426) ([#426](https://github.com/nash87/parkhub-rust/pull/426))
- Replace Policies eslint-disable with useDraftFromActive hook (#424) ([#424](https://github.com/nash87/parkhub-rust/pull/424))


### Chore

- Bump root + parkhub-web/package.json to 4.15.0 (#422) ([#422](https://github.com/nash87/parkhub-rust/pull/422))
- Bump to 4.15.0 — 2026-04-25 release wave + parkhub-php parity (#420) ([#420](https://github.com/nash87/parkhub-rust/pull/420))
- Install typescript + @astrojs/check, exclude stories from tsc (Phase 1) (#414) ([#414](https://github.com/nash87/parkhub-rust/pull/414))
- Retire PlaceholderV5 + add visual-regression update_snapshots input (#383) ([#383](https://github.com/nash87/parkhub-rust/pull/383))


### Dependencies

- Bump actions/download-artifact from 6.0.0 to 8.0.1 (#399) ([#399](https://github.com/nash87/parkhub-rust/pull/399))
- Bump the rust-deps group with 2 updates (#400) ([#400](https://github.com/nash87/parkhub-rust/pull/400))
- Bump rust from `c03ea15` to `8109983` (#396) ([#396](https://github.com/nash87/parkhub-rust/pull/396))
- Bump grid from 1.0.0 to 1.0.1 (#388) ([#388](https://github.com/nash87/parkhub-rust/pull/388))
- Bump rust from `275c320` to `c03ea15` (#389) ([#389](https://github.com/nash87/parkhub-rust/pull/389))


### Documentation

- Align parity-governance.md with PHP canonical version (#413) ([#413](https://github.com/nash87/parkhub-rust/pull/413))
- Replace parkhub-web/README boilerplate with real overview (#411) ([#411](https://github.com/nash87/parkhub-rust/pull/411))
- Post-merge-train drift cleanup (#410) ([#410](https://github.com/nash87/parkhub-rust/pull/410))
- V5 design showcase in README (#390) ([#390](https://github.com/nash87/parkhub-rust/pull/390))


### Fixed

- Unblock merge train — wait for lazy UPlotChart canvases (#416) ([#416](https://github.com/nash87/parkhub-rust/pull/416))
- Only re-init draft on activeId change (#407) ([#407](https://github.com/nash87/parkhub-rust/pull/407))
- Remove KI/AI from v5 user-facing strings (#394) ([#394](https://github.com/nash87/parkhub-rust/pull/394))
- Unwrap PaginatedResponse so NutzerV5 renders (T-1954) (#384) ([#384](https://github.com/nash87/parkhub-rust/pull/384))


### Tests

- Phase 4c — kill 8 file-level tsc errors with mixed patterns (#421) ([#421](https://github.com/nash87/parkhub-rust/pull/421))
- -41 tsc errors in admin/EV test suites (Phase 4b) (#419) ([#419](https://github.com/nash87/parkhub-rust/pull/419))
- Kill 37 tsc errors in Visitors+AdminUpdates (Phase 4a) (#418) ([#418](https://github.com/nash87/parkhub-rust/pull/418))
- Kill 42 tsc errors via wsAt() helper (Phase 3) (#417) ([#417](https://github.com/nash87/parkhub-rust/pull/417))
- Kill ~80 tsc errors via firstCall/nthCall helpers (Phase 2) (#415) ([#415](https://github.com/nash87/parkhub-rust/pull/415))
- Dashboard/Profil regression guards + PWA OfflineIndicator wire-up (#391) ([#391](https://github.com/nash87/parkhub-rust/pull/391))
- Axe-core audit + WCAG 2.1 AA fixes for v5 (T-1974) (#385) ([#385](https://github.com/nash87/parkhub-rust/pull/385))
- Rust mirror — 26 screens × visual + happy-paths (T-1952) (#382) ([#382](https://github.com/nash87/parkhub-rust/pull/382))


### Release

- Cut v5.0.1 (#429) ([#429](https://github.com/nash87/parkhub-rust/pull/429))


### Sync

- Cherry-pick Gitea test-stability fixes to unblock #371 (#372) ([#372](https://github.com/nash87/parkhub-rust/pull/372))


## [5.0.0] - 2026-04-23

### Added

- ParkHub v5 design system foundation for Rust runtime (#367) ([#367](https://github.com/nash87/parkhub-rust/pull/367))
- Density actually applies + Assistant queries real data (#350) ([#350](https://github.com/nash87/parkhub-rust/pull/350))
- Implement Rail, Top-tabs, Dock layouts + version-sync footer (#349) ([#349](https://github.com/nash87/parkhub-rust/pull/349))


### Chore

- Translate nav/shortcuts/assistant/settings keys across 9 locales (#352) ([#352](https://github.com/nash87/parkhub-rust/pull/352))


### Dependencies

- Bump openssl from 0.10.77 to 0.10.78 (#363) ([#363](https://github.com/nash87/parkhub-rust/pull/363))
- Bump openssl from 0.10.77 to 0.10.78 in /parkhub-server/fuzz in the cargo group across 1 directory (#362) ([#362](https://github.com/nash87/parkhub-rust/pull/362))
- Bump redb from 3.1.3 to 4.1.0 (#360) ([#360](https://github.com/nash87/parkhub-rust/pull/360))
- Bump the actions group with 5 updates (#358) ([#358](https://github.com/nash87/parkhub-rust/pull/358))
- Bump the cargo-minor-patch group with 3 updates (#359) ([#359](https://github.com/nash87/parkhub-rust/pull/359))
- Printpdf 0.8 → 0.9 + Op API migration (#347) ([#347](https://github.com/nash87/parkhub-rust/pull/347))


### Fixed

- Reduce Rust scanner findings (#365) ([#365](https://github.com/nash87/parkhub-rust/pull/365))
- Key mapped React fragments in heatmap views (#357) ([#357](https://github.com/nash87/parkhub-rust/pull/357))
- Wire /settings route + redesign Admin shell (no horizontal scroll) (#356) ([#356](https://github.com/nash87/parkhub-rust/pull/356))
- Login footer reads workspace version + sync docs to v4.14.2 (#355) ([#355](https://github.com/nash87/parkhub-rust/pull/355))
- Regen visual baselines in Playwright Jammy container (CI-matched) (#354) ([#354](https://github.com/nash87/parkhub-rust/pull/354))
- Tag DemoOverlay + regen 6 failing mobile visual baselines (#353) ([#353](https://github.com/nash87/parkhub-rust/pull/353))
- Drop parkhub-desktop from container workspace (#346) ([#346](https://github.com/nash87/parkhub-rust/pull/346))


### Tests

- Stabilize local browser and harness paths (#361) ([#361](https://github.com/nash87/parkhub-rust/pull/361))
- 2026 AI-driven stack + design-component coverage (#348) ([#348](https://github.com/nash87/parkhub-rust/pull/348))


## [4.14.2] - 2026-04-19

### Chore

- Bump workspace version to 4.14.2 (#345) ([#345](https://github.com/nash87/parkhub-rust/pull/345))


### Fixed

- Vendor OpenSSL per-target for macOS universal build (#344) ([#344](https://github.com/nash87/parkhub-rust/pull/344))


## [4.14.1] - 2026-04-19

### Chore

- Bump workspace version to 4.14.1 (#343) ([#343](https://github.com/nash87/parkhub-rust/pull/343))


### Fixed

- Explicit rustup target add for macOS universal build (#342) ([#342](https://github.com/nash87/parkhub-rust/pull/342))


## [4.14.0] - 2026-04-18

### Added

- Tauri 2 scaffold (coexists with Slint parkhub-client) (#336) ([#336](https://github.com/nash87/parkhub-rust/pull/336))
- Claude.ai/design v3+v4 integration + React 19 refactor (#335) ([#335](https://github.com/nash87/parkhub-rust/pull/335))
- Multi-country VAT profiles + EU B2B reverse-charge
- Per-module JSON Schema config editor modal
- Per-module JSON Schema config editor
- Runtime enable/disable toggle in ModulesDashboard
- Runtime enable/disable for safe modules + PATCH admin/modules/{name}
- Command Palette (Cmd+K) + Modules Dashboard
- Enrich api/v1/modules endpoint with ModuleInfo metadata
- Wire Redis revocation + per-identity rate-limit middleware
- Pluggable Redis revocation + refresh-token family rotation
- Per-identity limiter layered on per-IP
- Add ServiceMonitor + PrometheusRule templates
- Add RuntimeDefault seccomp profile + document PSS
- Add per-host circuit breaker on outbound webhook HTTP
- Dashboard CO₂ KPI tile + Co2Summary API typing
- Per-job run counter + duration histogram in scheduler
- Admin Modules Dashboard + plugin-native command registry
- Enriched ModuleInfo registry + GET api/v1/modules/info
- FuelType enum + CO2 summary endpoint (user scope v1)


### CI

- MacOS universal + Linux ARM64 build jobs (#337) ([#337](https://github.com/nash87/parkhub-rust/pull/337))
- Tier-1 workflow cleanup (#333) ([#333](https://github.com/nash87/parkhub-rust/pull/333))
- Add helm lint + template validation gate on chart changes
- Promote advisory checks to required now that CI is reliably green
- Cache Rust build with Swatinem/rust-cache
- Loosen gates to achievable-today + keep CWV as aspirational floor
- Install cargo-deny + cargo-audit via prebuilt binaries, cut 5-10 min/day
- Build frontend before cargo so rust_embed resolves
- Add openapi-drift workflow counterpart on Rust side


### Changed

- Split db.rs (4528 LOC) into domain-oriented sub-modules
- Extract bootstrap helpers from main.rs
- Preserve ModuleInfo doc-comment to keep openapi drift = 0
- Split api/modules.rs (3066 LOC) into focused sub-modules
- Decompose create_router into per-group helpers
- Tighten lock scopes to reduce contention under load


### Chore

- Cosign sign + PDB + topology-spread + Lighthouse CWV gates
- State-of-the-art 2025 local CI mirror + workflow cleanup
- Add trufflehog secret scan + document accepted RUSTSEC advisories
- Pin every GitHub Action to a SHA (v-tag as comment)


### Dependencies

- Bump reqwest 0.13, jsonschema 0.46, npm patches (#341) ([#341](https://github.com/nash87/parkhub-rust/pull/341))
- Bump rust from 1.94-slim to 1.95-slim (#329)
- Bump distroless/cc-debian13 from `9d41206` to `56aaf20` (#330)


### Documentation

- Fix stale steps + missing secrets + broken links across install paths
- Add SLO definitions + error-budget burn-rate alert guidance
- Refresh commit-SHA references after history rewrite
- Scrub internal task IDs from external-facing docs
- Fresh v4.13.0 screenshots + install-flow fix + capture script
- Refresh README + ARCHITECTURE for v4.13.0 Modular UX + refactors
- Wire remaining handlers — close coverage to ~100%
- Annotate 50 handlers — raise coverage from 9% to ~27%
- Document Modular UX platform
- Add drop-tightening note to v4.13.0
- Cut v4.13.0 for the Modular UX + security/testing cycle
- Sync README + AGENTS with parkhub-php sprint shipments
- Commit actual Rust OpenAPI dump + fix prefix normalisation
- Add Rust OpenAPI dump script (counterpart to PHP)
- OpenAPI parity methodology + diff script
- Mirror BFSG + EU AI Act templates from parkhub-php


### Fixed

- Bump UID/GID > 10000 (Trivy KSV-0020 + KSV-0021) ([#339](https://github.com/nash87/parkhub-rust/pull/339))
- Clear CodeQL warnings in Settings + AdminModules (#338) ([#338](https://github.com/nash87/parkhub-rust/pull/338))
- Fortlaufende invoice numbers + webhook idempotency
- Guard admin user writes against cross-tenant targets
- Regenerate rust.json for v2 ModuleInfo docstring update
- Regenerate rust.json openapi snapshot for ModuleInfo endpoint
- Resolve caller tenant_id on all domain-object creation paths
- Harden three remaining flaky specs on hydration timing
- Diff-openapi.sh handles both Scramble prefix variants
- Wait for client-side ProtectedRoute redirect in admin-route tests
- HTTP timeouts on all outbound reqwest clients + Helm probe/drain
- Render lot QR codes locally; clippy cleanup on main
- Gate PARKHUB_DISABLE_RATE_LIMITS behind e2e-bypass feature


### Performance

- Reduce LCP via route-preload gating + overlay defer + CSS/chunk hints
- Enable tower-http compression (gzip/br/zstd)


### Tests

- Add Playwright visual regression suite
- Add cargo-mutants nightly + insta snapshots
- Add proptest coverage for validators
- Wait for auth cookie after login to unblock mobile-safari
- Add cargo-fuzz skeleton + JWT/HMAC targets
- Align Dashboard.test with KpiCard migration
- Wire getCo2Summary mock into Dashboard test setup
- Cargo-mutants config + weekly CI sweep for coverage gaps
- Proptest envelope coverage (ApiError, meta, requests)
- Proptest round-trip coverage for cross-boundary enums


### Marathon

- Sim audit shape + visual expand + a11y login contrast (#332) ([#332](https://github.com/nash87/parkhub-rust/pull/332))


### Ops

- Add Fly.io + Railway templates + nightly install smoke test
- Ship default Grafana dashboard (opt-in via values)


## [4.12.0] - 2026-04-16

### Added

- Add locale-coverage script as a drift guard
- Wire Sentry SDK for error tracking (opts in via SENTRY_DSN)


### Documentation

- Bump Release badge to v4.11.0


### Fixed

- Give admin a real credits allowance on first setup


### Performance

- Lazy-load non-English locales to shave ~450KB
- Lazy-load Layout to shrink pre-auth critical path


### Tests

- Replace deprecated networkidle waits with domcontentloaded


### Release

- V4.12.0


### Sec

- Add Cross-Origin-{Opener,Resource}-Policy: same-origin
- Add object-src 'none' to the rust CSP


## [4.11.0] - 2026-04-16

### CI

- Ping Render demo every 10 min to prevent spin-down
- Set APP_URL so build_auth_cookie drops the Secure flag


### Chore

- Sync Cargo.lock with 4.11.0 workspace version bump


### Dependencies

- Bump astro 6.1.6 -> 6.1.7


### Documentation

- Add Astro 6 + Tailwind CSS 4 badges


### Fixed

- Retire dead sw-v2.js route and enhanced_service_worker
- Digest-pin all FROM directives for supply-chain hardening
- Pin busybox:latest to busybox:1.37.0
- Also stop shadowing Astro-built manifest.json
- Stop shadowing Astro-built service worker with inline stub


### Performance

- Instant navigation via prefetch + View Transitions API


### Tests

- Wait for cancel button before clicking it
- Anchor max-per-day test to tomorrow 09:00 UTC
- Widen cross-env tolerance to 10%


### Release

- V4.11.0


## [4.10.0] - 2026-04-15

### Added

- GET api/v1/bookings/guest returns current user's passes
- P0 sidebar regrouping + empty-state onboarding + test mocks
- React 19 useOptimistic on cancel for instant UI
- Locale-aware KPI counter + transparent token refresh interceptor
- Kinetic Observatory dashboard — KPI cards, trend, sensor feed, activity table
- Enforce 2FA at login with temp token flow


### Build

- Upgrade rand 0.9→0.10 + redb 2→3
- Bump tray-icon from 0.21.3 to 0.22.0 (#328)
- Bump rustls from 0.23.37 to 0.23.38 in the cargo-minor-patch group (#324)
- Bump mdns-sd from 0.18.2 to 0.19.0 (#327)
- Bump astro from 6.1.5 to 6.1.6 in parkhub-web in the npm-minor-patch group (#323)


### CI

- Shard E2E across chromium + mobile-chrome + mobile-safari
- Remove continue-on-error masks on CI + E2E jobs
- Drop --retries=2 CLI flag, defer to playwright.config.ts
- Build amd64 only (drop arm64 multi-arch)


### Chore

- Collapse nested ifs to satisfy clippy pedantic
- Regenerate Cargo.lock after rebase
- Add coverage to.gitignore


### Dependencies

- Bump rustls-webpki 0.103.11 → 0.103.12 (RUSTSEC-2026-0098)


### Documentation

- Append v4.10.0 session fixes
- Bump to v4.10.0 + changelog for Kinetic Observatory + deploy fixes
- Add CODE_OF_CONDUCT and NOTICE for public release


### Fixed

- NotificationCenter auth — use api client, not localStorage
- Kill the per-render DDoS on api/v1/demo/status
- Disable mDNS in unattended/headless mode
- Dashboard skeleton matches Kinetic Observatory layout
- 3 P0 bugs found by UX audit subagent
- Return 404 JSON for unknown api/* routes + E2E cleanups
- Add PARKHUB_DISABLE_RATE_LIMITS escape hatch for E2E
- Pin distroless/cc-debian13 digest to silence DS-0001
- COPY data with --chown to preserve nonroot ownership
- Use distroless/cc-debian13 runtime to match builder
- Bridge --dt-* tokens into --theme-* so body actually repaints
- I18n keys, NaN guards, Material You theme, demo banner, page titles
- PWA icons, notification guard, CSP header, footer landmark
- Add Stripe webhook signature verification with HMAC-SHA256
- Cookie secure default, env example, OAuth CSRF, JWT revocation, TOTP SHA-256
- Revert broken OccupancyPrediction split + cargo fmt updates.rs
- Bust stale Docker cache v3 — v2 inherited broken builder layer
- Re-copy Cargo.toml in builder stage, remove error-swallowing || true
- Bust Docker build cache — stale cache served headless= binary
- Add healthCheckPath and demo password to render.yaml
- Enable all API modules in headless Docker build + guard optional-chain array ops
- Remove unused imports in TeamLeaderboard
- Wire selectedLot to stats API in OccupancyPrediction
- Prevent SSRF in self-update API via version input validation


### Performance

- O(1) API key lookup via reverse prefix index


### Tests

- Health-JSON test probes health/ready + system/version
- Health-endpoint JSON probe for Rust layout
- Remaining Rust-specific nightly fixes
- Sync nightly-suite fixes from parkhub-php 10773bf
- Sync NotificationCenter test with api-client refactor
- Admin-crud tolerant of rust vs php endpoint differences
- Webhook URLs → admin/webhooks-v2 (real route)
- Update nonexistent-route assertion to match new 404 behavior
- Unblock concurrent-users, devtools, theme switcher tests
- Fix racy admin-route guard + skip visual on non-chromium
- Use CARGO_PKG_VERSION in test_current_version assertion
- Replace networkidle with domcontentloaded, cut retries to 1
- Mark network-failure catch blocks as istanbul ignore
- Uptime branches + defensive guards istanbul-ignore
- GuestPass error paths, QRCheckIn status fallback + defensive guards
- Profile.tsx — mark defensive guards as istanbul ignore
- Dashboard WS events, LoginHistory UA branches
- Notifications.tsx error paths, timeAgo branches, success type
- Bookings.tsx WS events, cancel errors, filters
- Translations.tsx 87% → 97% (+7 tests)
- Final coverage push — 96.46% statements, 1994 tests
- Comprehensive theme switching audit + use api client
- Fix flaky smtp_config test — remove unsafe env var manipulation
- Achieve >90% coverage across all frontend files
- Final coverage push — batch 2
- Final coverage push — batch 1
- 100% coverage for notification, visitors, approval, admin views
- 100% coverage for calendar, translations, admin CRUD, user views
- Boost coverage for admin views and critical paths to 100%
- 100% coverage for zero-coverage components and locales
- Update cookie secure test to match new fail-safe default
- Add coverage for critical untested components


## [4.9.0] - 2026-04-13

### Added

- Wire up update API routes — all 5 endpoints live
- Self-update system with version history + rollback
- 4 new premium themes + enhanced animations
- Add Team Leaderboard + Smart Predictions views


### Fixed

- Rebuild docker-publish — amd64-only, push-to-main trigger, Render auto-deploy
- Make team/leaderboard E2E test tolerant of feature flags
- Clean updates.rs — remove utoipa annotations, fix check_admin API
- Add utoipa::ToSchema derive to update API types


### Release

- V4.9.0 — API resilience, React 19 useOptimistic, security hardening


## [4.8.0] - 2026-04-13

### Added

- Add QR Check-In, Swap Requests, Guest Pass, Occupancy Heatmap
- Translate remaining untranslated strings across all 9 locales
- CODEOWNERS, SEO meta tags, X-RateLimit headers


### Chore

- Bump version badges to v4.8.0


### Documentation

- Add What's New in v4.8.0 section to README


### Fixed

- Add getInMemoryToken mock + QR ok:true in tests (CI fix)
- Address all pre-release review findings (7 agents)
- Add missing nav.favorites key to all 10 locales
- Resolve all clippy warnings + add cutting-edge CSS patterns + DESIGN.md
- Clippy --fix collapsible_if + auth_flow CI retry


### Release

- V4.8.0 — QR Check-In, Swap Requests, Guest Pass, Heatmap


## [4.7.0] - 2026-04-12

### Build

- Bump the cargo-minor-patch group across 1 directory with 3 updates (#321)
- Bump the actions group across 1 directory with 3 updates (#320)
- Bump react-i18next from 16.6.1 to 17.0.2 in parkhub-web (#317)
- Bump i18next from 25.10.4 to 26.0.3 in parkhub-web (#316)
- Pin Rust 1.94.1 via rust-toolchain.toml, update all CI workflows
- Bump vite from 7.3.1 to 7.3.2 in parkhub-web (#319)
- Bump the npm-minor-patch group in parkhub-web with 5 updates (#315)
- Bump defu from 6.1.4 to 6.1.6 in parkhub-web (#314)


### CI

- Rename gate job CI → Required checks (fix auto-merge blocking)
- Add nightly assurance workflow (parity with PHP)
- Add docker-compose.test.yml + integration tests in CI
- Update actionlint v1.7.11→v1.7.12, setup-qemu-action v3→v4


### Chore

- Remove.claude from tracking, fix deny.toml stale advisory
- Align parkhub-web version to 4.5.0


### Documentation

- Update all MD files to v4.5.0


### Fixed

- Fix release workflow — OpenSSL for Windows, tolerant release job
- Fix docker-publish skipping on tag push (same validate-tag issue)
- Fix release workflow skipping on tag push
- Remaining cargo fmt diffs + nightly.yml shellcheck (SC2015, SC2034)
- Fix api.spec.ts token extraction + bad-creds field name
- Resolve all 28 clippy warnings in test files
- Router.test.tsx require → direct mock, add RUSTSEC-2026-0097 ignore
- Remove stale RUSTSEC-2025-0141 advisory ignore (bincode no longer in tree)
- Fix integration test API contract mismatches
- Landing page infinite loop -- event-driven 401 handling (synced from PHP #265)


### Tests

- Add frontend Vitest expansion (hooks, validation, router, error boundary)
- Add 6 E2E Playwright suites (multi-lang, offline, concurrent, admin CRUD, edge cases, security)
- Add 1-month booking simulation engine (3 profiles)
- Add 10 integration test suites (78 tests)
- Add a11y, visual regression, and Lighthouse LCP threshold


### Release

- V4.7.0 — Rust edition 2024, full test pyramid, installer


### Security

- Remove program.md (exposed local paths), update compliance docs
- Remove internal references from public repo


## [4.6.0] - 2026-03-27

### Added

- Finish desktop admin user flows
- Harden rust push, compliance pdf, and admin reset flows


### Build

- Switch Docker runtime to distroless, move entrypoint logic into binary (#305)
- Bump picomatch in parkhub-web (#301)
- Refresh Cargo.lock for 4.5.0 crates


### CI

- Align rust workflows on 1.88
- Make clippy advisory on main


### Documentation

- Align readme with current toolchain and test counts
- Add FEATURES.md feature showcase with API examples and use cases (#295)
- Add CONTRIBUTING.md with development setup and PR guidelines (#294)
- Overhaul README for public audience with badges, screenshots, quick-start (#293)
- Add GitHub issue templates for bug reports and feature requests


### Fixed

- Gate SMS/WhatsApp notification toggles (opacity + pointer-events-none)
- Guard theme fetch against non-200 response to prevent retry loop
- Auto-fix clippy warnings
- Resolve all clippy warnings (cargo clippy --fix + manual allows)
- Clear main seed security and clippy drift
- Replace map_or with is_some_and for clippy
- Update admin_list_all_bookings test for pagination envelope
- Use clamp instead of min/max in admin analytics occupancy rate


### Performance

- Add pagination to admin_list_bookings and admin_list_users (#304)


### Security

- Fix OAuth signup bypass of allow_self_registration and missing CSRF state (#299)


### Tests

- Expand E2E Playwright coverage to 20 spec files (#289)
- Satisfy clippy on headless CI


### Merge

- Product-truth-cleanup (clippy fixes, forge-operator init)


### Security

- Document instant advisory exception
- Reorder password validation before DB calls in register; cap username dedup loop (#300)


## [4.5.0] - 2026-03-25

### Added

- Add Google Calendar and iCal sync for bookings
- Add admin analytics API with occupancy, revenue, popular-lots


### CI

- Add auto-merge workflow for low-risk Copilot PRs
- Trigger fresh CI run
- Fix rust workflow toolchain and security checks
- Make actionlint advisory, not blocking gate
- Audit and harden GitHub Actions for Rust, security, and releases


### Tests

- Add integration tests for mobile booking endpoints


### Release

- V4.5.0


## [4.4.0] - 2026-03-25

### Added

- Merge Copilot test PR #278 + fix GDPR test assumptions + notification data field
- Smart notification center with real-time badge count


### Chore

- Initial plan for comprehensive test coverage
- Add aoe repo defaults


### Fixed

- Resolve all clippy warnings, remove unused imports
- Resolve all clippy warnings across workspace
- Restore rust workflow health


### Tests

- Add webhooks_v2 test module + mobile test integration
- Add ~60 tests across 6 zero-coverage modules
- Add comprehensive tests for common types, protocol, branding, lots_ext, team, translations


### Release

- V4.4.0 — mobile booking, notification center, test coverage boost


## [4.3.0] - 2026-03-23

### Added

- Zone pricing tiers with economy/standard/premium/vip
- Multi-format audit log export with signed URLs
- Role-based access control with fine-grained permissions


### Fixed

- Remove unused fireEvent import from PWAEnhanced test
- Restore waitFor import in PWAEnhanced test
- Resolve CodeQL alerts — unused import + property injection
- Remove temp script, fix WebhooksLogo mock in test
- I18n brace nesting, cargo-deny ignores, async test waitFor, phosphor icon rename


### Release

- V4.3.0 — RBAC, Audit Export, Parking Zones


## [4.2.0] - 2026-03-23

### Added

- Mobile experience with offline support
- Outgoing event subscriptions with delivery tracking
- SAML/SSO enterprise authentication


### Documentation

- Update README for v4.1.0 — 57 modules, expanded features, NIS2 compliance, CI scanning
- Update README for v4.1.0 — 57 modules, expanded features, NIS2 compliance, CI scanning


### Fixed

- Resolve remaining CodeQL alerts (property injection, unused vars)
- Resolve CodeQL alerts — prototype pollution, unused imports
- Resolve all CodeQL JS/TS security scanning alerts in parkhub-web
- Eliminate Docker image CVEs by removing python3 and upgrading base


### Release

- V4.2.0 — SSO, Webhooks v2, Enhanced PWA


## [4.1.0] - 2026-03-23

### Added

- Version headers, deprecation notices, changelog
- Automated email digest system
- Booking sharing & guest invites


### Build

- Bump the actions group with 7 updates (#255)
- Bump the npm-minor-patch group in parkhub-web with 5 updates (#254)
- Bump rust from 1.93-slim to 1.94-slim (#253)


### Fixed

- Bump frontend version to 4.0.0
- Div_ceil + writeln instead of write with newline
- Const fn default_waitlist_status
- Add missing plugins section to EN locale
- Add missing icon mocks (ShieldCheck, PuzzlePiece, GraphicsCard)
- Cargo fmt all v3.7-v4.0 modules
- Add workflow_dispatch to CodeQL, disable default setup


### Release

- V4.1.0 — Booking Sharing, Scheduled Reports, API Versioning


## [4.0.0] - 2026-03-23

### Added

- GDPR/DSGVO audit trail export and compliance reports
- GraphQL API with query/mutation support and playground
- Modular plugin/extension system with registry and event hooks


### CI

- Best-in-class security tooling for 2026


### Fixed

- Puzzle → PuzzlePiece (valid phosphor icon)
- Prevent theme FOUC with inline pre-hydration script


### Release

- V4.0.0 — Plugin System, GraphQL API, Compliance Reports


## [3.9.0] - 2026-03-23

### Added

- Auto-generated Postman collection endpoint + static collection
- Load testing scripts for smoke, load, stress, and spike scenarios
- Kubernetes Helm chart with full module flag support


### Release

- V3.9.0 — Helm chart, k6 load tests, Postman collection


## [3.8.0] - 2026-03-22

### Added

- Customizable admin dashboard widget system
- Drag-to-reschedule bookings on calendar
- Absence request workflow with admin approval


### Release

- V3.8.0 — absence approval, calendar drag, admin widgets


## [3.7.0] - 2026-03-22

### Added

- Interactive API documentation with Swagger UI
- Digital parking pass with QR verification
- Enhanced waitlist with notifications and accept/decline


### Release

- V3.7.0 — waitlist notifications, parking passes, API docs


## [3.6.0] - 2026-03-22

### Added

- Geofencing with auto check-in (#239)
- Personal parking history with stats (#238)


### Fixed

- Sync test mocks with component imports (#237)
- BatteryCharging test mock (#236)
- Deduplicate imports + fix Battery icon (#235)
- Remove duplicate model imports in db.rs (#234)


### Release

- V3.6.0 (#240)


## [3.5.0] - 2026-03-22

### Added

- Enhanced smart slot recommendations with scoring algorithm
- EV charging station management with session tracking
- Visitor pre-registration with QR codes


### Release

- V3.5.0 — visitor pre-registration, EV charging, smart recommendations


## [3.4.0] - 2026-03-22

### Added

- Cost center billing with CSV export and credit allocation
- Maintenance scheduling with slot blocking
- Accessible parking system with priority booking


### Documentation

- Add comprehensive legal compliance documentation


### Release

- V3.4.0 — accessible parking, maintenance scheduling, cost center billing


## [3.3.0] - 2026-03-22

### Added

- Vehicle fleet management with stats and flagging
- Import/export suite with CSV/JSON bulk operations
- Paginated audit log UI with CSV export and filters


### Release

- V3.3.0 — audit log, data import/export, fleet management


## [3.2.0] - 2026-03-22

### Added

- Multi-tenant isolation with admin CRUD and data-scoping fields
- Admin rate limiting dashboard with blocked-request history
- Calendar subscription with personal tokens and subscribe UI


### Release

- V3.2.0 — iCal sync, rate dashboard, multi-tenant


## [3.1.0] - 2026-03-22

### Added

- Checkout sessions, webhook handler, payment history
- Structured push payloads, service worker notifications, useNotifications hook
- Interactive Leaflet map view with lot markers


### Fixed

- Auto-format analytics.rs


### Release

- V3.1.0 — map view, web push, Stripe payments


## [3.0.0] - 2026-03-22

### Added

- HTML email templates with inline CSS
- Admin analytics dashboard with backend API
- Complete 10-language support with full translations


### Fixed

- Cap API key expires_in_days to 365 (CodeQL alert #56)
- Add permissions block to lighthouse.yml


### Release

- V3.0.0 — 10 languages, analytics, email templates


## [2.9.0] - 2026-03-22

### Added

- Interactive Onboarding Wizard (#200) (#205)
- Lobby Display Kiosk Mode (#198) (#204)


### Release

- V2.9.0 (#206)


## [2.8.0] - 2026-03-22

### Added

- Enhance WebSocket with auth, heartbeat, occupancy snapshot


### Changed

- Extract handlers from mod.rs into dedicated modules


### Documentation

- Update README + CHANGELOG for v2.7.0 (12 themes, pricing, hours, SMS)


### Fixed

- Workspace lint override, ThemeSwitcher 6→12 themes test
- Add Palette/Check icon mocks to Layout, getDynamicPrice mock to Book
- Allow unsafe in parkhub-client for Slint FFI on Windows
- Auto-format mod.rs imports


### Release

- V2.8.0 — WebSocket real-time + API extraction


## [2.7.0] - 2026-03-22

### Added

- Add SMS/WhatsApp channel stubs
- Add per-lot operating hours with booking validation
- Add occupancy-based dynamic pricing
- Synthwave + Zen themes (12 total) (#193)
- 4 new design themes — Liquid, Mono, Ocean, Forest (#192)


### Fixed

- Clippy unused imports, frontend test mocks for theme/2FA/notifications
- Use username field for Rust API, fix token extraction


### Tests

- Add full user+admin workflow E2E with 12-theme cycle and booking simulation
- Update theme tests for all 12 themes (was 6)


### Stitch

- Add Wabi-Sabi design screenshot


## [2.6.0] - 2026-03-22

### Added

- Add PDF invoice generation for bookings
- Add Google + GitHub social login


### Changed

- Extract booking handlers from mod.rs into bookings.rs


### Documentation

- Update README to v2.5.0 — themes, httpOnly cookies, OAuth, test counts


### Fixed

- Gate quick_book and booking_checkin routes behind mod-bookings feature


### Tests

- Add comprehensive Playwright E2E test suite


### Security

- Remove hardcoded password from docker-compose.yml


## [2.5.0] - 2026-03-22

### Fixed

- Replace localStorage JWT with httpOnly cookie auth (#154)


## [2.4.0] - 2026-03-22

### Added

- Design theme switcher with 6 visual themes (#179)


### Fixed

- Len_zero in coverage tests


### Stitch

- Improved generator v2 (separate projects, DESIGN.md context, verified discussion IDs)
- Enhanced v3 designs with DESIGN.md + pro UX prompts
- 5 unique AI-generated designs (separate projects per screen)


## [2.3.0] - 2026-03-22

### CI

- Downgrade Lighthouse performance to warn (static SPA shell)
- Add Lighthouse CI workflow with quality gates


### Chore

- GitOps polish — README, CHANGELOG, SECURITY.md, templates


### Fixed

- Fmt coverage_tests.rs + update lockfile
- Close accessibility gap from 93 to 100


### Tests

- Add 104 coverage tests for security, admin, and edge cases (#175)


### Stitch

- Add 4 AI-generated design proposals (mobile-pass, admin-analytics, onboarding, lobby)


## [2.2.0] - 2026-03-22

### Added

- Modern UI 2026 redesign — glass morphism, animated counters, bento grid


### Fixed

- Align test assertions with actual i18n translations
- Resolve 13 QA issues (#151-#165)


## [2.1.0] - 2026-03-22

### Added

- Add frontend components - 2FA, notifications, login history, bulk actions, print
- Add admin QoS features - bulk ops, advanced reports, notifications, booking policies, health
- Add security improvements - 2FA/TOTP, password policy, login history, sessions, API keys


### Fixed

- Clippy let-else, fmt, frontend test mocks


## [2.0.0] - 2026-03-22

### Added

- Add test coverage for background jobs (#112)
- Add compile-time module feature flags for all API modules
- System endpoints, iCal absence import, webhook retry
- Add background jobs system (closes #42)
- Implement PATCH bookings/{id}, PUT recurring-bookings/{id}, PUT absences/{id}
- Per-slot QR code, PWA manifest, branding endpoints
- Add GET admin/credits/transactions endpoint with filters
- Add PUT lots/{lot_id}/zones/{zone_id} update endpoint (closes #48)
- Dedicated pricing endpoint + daily_max cap in price calculation (closes #37)
- Add BookingReminder and WaitlistSlotAvailable notifications (closes #35)
- Add slot_type, status, and feature filters to GET lots/{id}/slots (closes #38)
- Add bulk user import via CSV (closes #40)
- Add welcome + cancellation emails, mark SMTP as Done


### CI

- Stop spam — security scanning weekly only, remove MSRV/Docker from PRs
- Stop spam — security scanning weekly only, remove MSRV/Docker from PRs
- Add MSRV check (1.82) and Docker build test to CI
- Add --all-targets and doc tests to test job (closes #68)
- Fix action versions, add fmt check and security scanning


### Changed

- Add admin_middleware layer for admin routes (#109)
- Extract 10 handler modules from api/mod.rs (8168→5353 lines)
- Replace unnecessary write locks with read locks in submodules (#62)
- Replace unnecessary write locks with read locks in submodules (#62)
- Extract vehicles module, replace write locks with read locks


### Chore

- Update deps — patch CVEs


### Dependencies

- Bump h3 from 1.15.6 to 1.15.9 in parkhub-web


### Documentation

- Redesign README for professional presentation
- Update test counts to 1,390 (v1.9.0 actuals)
- Update README + CHANGELOG for v1.9.0 features


### Fixed

- Remove unsafe-inline from CSP style-src (#106)
- Use constant-time comparison for metrics token (#114)
- Replace mem::forget with TestHarness for test isolation (#108)
- Remove unused rustls-pemfile dependency (#102)
- Add input length limits on key endpoints (#115)
- Invalidate all sessions on password change (#116)
- Un-ignore max_bookings_per_day enforcement test (#103)
- Make MSRV informational, fix checkout action version
- Bump MSRV from 1.82 to 1.85
- Move confidence-threshold before exceptions in deny.toml
- Add all GTK3/Slint advisory ignores and renderer license exceptions to deny.toml
- Update deny.toml with correct license exceptions for Slint and ring
- CI doc tests and add deny.toml for cargo-deny policy
- Resolve CI failures — compilation errors, clippy warnings, and broken tests
- Correct clippy lint names for Rust 1.94
- Allow remaining Rust 1.94 pedantic/nursery lints
- Allow new Rust 1.94 pedantic lints (CI clippy failure)
- Mask credentials in seed_demo.py output (CodeQL alert)
- Resolve 42 clippy warnings in parkhub-client
- Deadlock in create_booking, XSS in public_display, quick_book rollback


### Performance

- Wrap Argon2 password hashing in spawn_blocking (#117)
- Use LazyLock for vehicle city codes map (closes #111)
- Reduce write-lock scope in create_booking, add booking-by-user index, re-validate user on auth


### Tests

- Add booking workflow, admin, and rate-limit integration tests


### Quality

- Zero clippy warnings, security hardening, simplify pass


### Security

- Reduce JWT TTL to 1h, add token revocation, rate-limit auth endpoints, explicit Argon2 params, extend audit logging


## [1.9.0] - 2026-03-21

### Added

- Favorites UI — view, nav, i18n for 10 locales
- OpenAPI docs for translations/recommendations, fix demo drain
- A11y audit, analytics charts, clippy pedantic fixes
- Smart recommendations, typed API, CSV export, runtime i18n overrides
- Translation management system + UI/UX 2026 overhaul


### Chore

- Bump version to v1.9.0, update README badges


### Fixed

- Consolidate demo mode env var tests to prevent race condition
- Include devDependencies in Docker web build stage
- Skip Astro font fetch in CI/Docker builds


## [1.8.0] - 2026-03-21

### Added

- Add Stripe payment stub for demo showcase
- Add admin CSV export for bookings, users, and revenue
- Add QR code parking pass generation
- Add Stripe payment integration stub
- Add WebSocket real-time event support
- Add occupancy heatmap visualization to admin reports
- Add structured observability to ParkHub backend


### Chore

- Bump version to v1.8.0, update README with new features


### Fixed

- Correct export paths in OpenAPI test assertion
- Resolve all CI clippy/test failures for registration and code quality
- Add password_confirmation to client RegisterRequest constructor
- Heatmap test used future booking time at midnight, use yesterday
- Correct test assertions for ws broadcast lag and reconnect
- Add password confirmation to registration form (#21)


## [1.7.1] - 2026-03-20

### Added

- Polish dark mode with OKLCH palette, glass morphism, and reactive system theme
- Enhance PWA with offline support, background sync, and install prompt


### Documentation

- Annotate remaining 23 handlers and register in OpenAPI spec
- Annotate 66 handlers in mod.rs with utoipa::path and register in OpenAPI spec
- Add OpenAPI annotations to webhooks, zones, favorites, push, setup, and export endpoints
- Annotate 66 handlers in mod.rs with utoipa::path and register in OpenAPI spec


### Fixed

- Add Debug derive to AuthUser for clippy compliance


## [1.7.0] - 2026-03-20

### Added

- Complete i18n audit — replace all hardcoded strings with t calls
- Production-grade middleware stack


### Documentation

- Add OpenAPI annotations to webhooks, zones, favorites, push, setup, and export endpoints


### Fixed

- Replace is_ok/unwrap_err with match for clippy compliance


## [1.6.1] - 2026-03-20

### Added

- Comprehensive accessibility improvements across all views


### CI

- Add gate job matching required "CI" status check


### Documentation

- Update README and CHANGELOG for v1.6.0


### Performance

- Reduce main bundle 627K → 129K with code splitting


### Tests

- Add 31 HTTP API integration tests for parkhub-server
- Add 9 Playwright E2E specs (admin, booking, responsive, dark-mode, etc.)
- Add 101 Vitest unit tests for 13 untested views/components


## [1.6.0] - 2026-03-20

### Added

- Demo reset actually wipes DB and re-seeds
- Typed error handling, React 19 patterns, TW4 @utility, admin search


### Fixed

- Demo reset race conditions and error handling


### Security

- Rate-limit demo vote/reset endpoints (3/min per IP)


## [1.5.5] - 2026-03-20

### Added

- Occupancy donut chart in AdminReports, build hash via Vite define


### Performance

- React.lazy admin routes, memoize Team/Calendar, DemoOverlay guard — 350 Rust + 197 Vitest passing


### Tests

- 421 Rust tests — credits, lots, bookings, requests serde + edge cases


## [1.5.4] - 2026-03-20

### Changed

- Code review fixes — shared constants, N+1 elimination, i18n


### Fixed

- Make Lighthouse CI non-blocking (continue-on-error)
- Clippy redundant import + Lighthouse CI server startup


## [1.5.3] - 2026-03-20

### Added

- Command palette (Ctrl+K), admin charts, Lighthouse CI, 727 tests


## [1.5.2] - 2026-03-20

### Fixed

- Cargo fmt on webhook + db tests


### Tests

- 631 total tests — email, health, jwt, waitlist, zones, system, register, calendar, 404
- 196 Rust + 163 vitest + 180 PHP = 539 total tests


### Design

- Clean up Register + ErrorBoundary (remove mesh-gradient, glass-card)


## [1.5.1] - 2026-03-20

### Added

- Add Book a Spot page — 3-step booking flow


## [1.5.0] - 2026-03-20

### Added

- Add 404 Not Found page instead of silent redirect
- Add Forgot Password page + forgotPassword/resetPassword API methods


### Fixed

- Maestro login-failure flow use pressKey:Enter
- Cargo fmt + Maestro pressKey:Enter flows


## [1.4.9] - 2026-03-19

### Chore

- Gitignore Playwright test-results
- Update README badge + CHANGELOG to v1.4.8


### Tests

- Add Playwright E2E tests (welcome, login, dashboard, dark mode, mobile)


## [1.4.8] - 2026-03-19

### Fixed

- Add missing nav.team/calendar/notifications i18n keys


## [1.4.7] - 2026-03-19

### Fixed

- Dynamic version from package.json, bump to v1.4.7


## [1.4.6] - 2026-03-19

### Tests

- 434 total tests — full coverage across all layers


### Design

- Apply UI/UX Pro Max design system — system font, tight tracking


## [1.4.5] - 2026-03-19

### Copy

- Replace generic AI marketing copy with specific product description


## [1.4.4] - 2026-03-19

### Design

- Clean up Admin views + refine global CSS


## [1.4.3] - 2026-03-19

### Design

- Full AI slop removal across all views


## [1.4.2] - 2026-03-19

### Added

- 90 Rust tests, 106 vitest, 5 Maestro E2E flows


### Fixed

- Version badge test uses regex instead of hardcoded version


### Security

- Fix rate limiter panic on zero config + password via env var


### Design

- Eliminate AI slop from Welcome + Login pages


## [1.4.1] - 2026-03-19

### Chore

- Bump version to v1.4.0, add Maestro E2E tests


### Fixed

- Add id to login submit button for E2E testing


## [1.4.0] - 2026-03-19

### Added

- Micro-interactions, animated stats, empty state polish
- Skeleton loading, i18n coverage, Layout test, UI polish


## [1.3.17] - 2026-03-19

### Fixed

- Remove api.getFeatures/updateFeatures calls (no backend endpoint)


## [1.3.16] - 2026-03-19

### Documentation

- Add AGENTS.md for agent-assisted development


### Fixed

- Redirect first-time visitors to welcome language screen


## [1.3.13] - 2026-03-19

### Fixed

- Use per-key PUT for Render env vars API


## [1.3.12] - 2026-03-19

### Fixed

- Use Render API GET+merge+PUT for env vars


## [1.3.11] - 2026-03-19

### Fixed

- Normalize DemoStatus API response + set Render env vars in deploy


## [1.3.9] - 2026-03-19

### Fixed

- Use 'demo' as default admin password everywhere


## [1.3.8] - 2026-03-19

### CI

- Add vitest job, fix clippy allows, fix Docker npm ci


### Dependencies

- Bump the npm-minor-patch group across 1 directory with 6 updates


### Documentation

- Redirect docs/CHANGELOG.md to root CHANGELOG


### Fixed

- Allow unused_mut on data_dir for headless builds


## [1.3.7] - 2026-03-19

### V1.3.7

- Prometheus metrics, global rate limiting, OpenAPI, Vitest


## [1.3.6] - 2026-03-19

### Added

- Wire UseCaseSelector route + full PWA support


## [1.3.5] - 2026-03-19

### Added

- Wire use-case CSS theme via ThemeLoader component
- Add SEED_DEMO_DATA mode + deployment modes docs


### Fixed

- Update smoke test to use admin@parkhub.test demo


## [1.3.4] - 2026-03-19

### Added

- Use-case CSS theme overrides + fix.test TLD


## [1.3.3] - 2026-03-19

### Fixed

- Change demo credentials to admin@parkhub.demo demo


## [1.3.2] - 2026-03-19

### Added

- Use-case theming system with 5 presets


### Fixed

- Remove unused Slint components and spurious Icon export


## [1.3.1] - 2026-03-19

### Added

- Redesign login page — split-screen layout with hero panel


## [1.3.0] - 2026-03-19

### Added

- Wire webhook dispatch + audit log persistence into handlers
- Add 12 remaining admin + user endpoints for full API coverage
- Vehicle photos, city codes, QR, dashboard charts — 100% parity
- Add audit log, team list, autoresearch program.md
- Add zones + user favorites
- Add web push notification endpoints (#23)
- CSV export, iCal, public display, change password
- Add webhook system with SSRF protection (#22)
- Add setup wizard API endpoints (#21)
- Add slot CRUD + vehicle update endpoints (#19, #20)
- DemoOverlay shows reset status, countdown, and resetting indicator
- Demo reset — actual DB wipe + 6h auto-reset scheduler
- Full feature parity — 32 new API endpoints, 8 models, 7 DB tables, complete frontend


### CI

- Auto-deploy to Render after Docker image push


### Changed

- Suppress dead code warning on server_connection module (WIP client)
- Suppress dead code warnings on scaffolding modules
- Remove unnecessary clone on User in auth responses
- Split api.rs into modules (5901→4628 lines)


### Dependencies

- Merge dependabot — directories 6, toml 1.0, actions bumps
- Bump directories from 5.0.1 to 6.0.0
- Bump toml from 0.9.12+spec-1.1.0 to 1.0.6+spec-1.1.0 (#16)
- Bump docker/build-push-action from 6 to 7 (#14)
- Bump actions/setup-node from 4 to 6 (#13)


### Documentation

- Add v1.3.0 changelog and live demo section to README


### Fixed

- GDPR Art. 15 export now includes absences, credit transactions, notifications
- Resolve clippy field_reassign_with_default in tests
- Allow dead_code on send_push_notification placeholder
- Design audit P0 fixes — disabled states, tabular nums, reduced motion
- Default total_slots to 10 when creating a lot via API
- Remove unused std::io::Write import in config tests
- DemoOverlay accessibility and UX improvements
- Resolve all 14 clippy warnings (clean -D warnings)
- Code review fixes — safety, logging, GDPR
- Metrics auth, font fix, ErrorBoundary, icon fixes, admin mobile nav
- Hardcoded pricing, cargo fmt, WCAG accessibility
- Admin lot creation + credit quota system (closes #15)
- Add port to render.yaml, respect PORT env var in entrypoint


### Tests

- Add 20 integration tests — 60 total, full DB coverage
- Add 5 integration tests for new DB operations


### A11y

- Add reduced motion support + improve input focus indicators


### Legal

- Add Art. 28(3)(h) audit rights clause to AVV template


### Release

- V1.3.0 — version bump, smoke test, API docs update


## [1.2.5] - 2026-03-14

### Fixed

- API deserialization + version bump to v1.2.5


## [1.2.4] - 2026-03-14

### Fixed

- Correct docs — password, ports, build cmd, versions


## [1.2.3] - 2026-03-14

### Fixed

- Add Node.js frontend build step to release workflow


## [1.2.2] - 2026-03-14

### Fixed

- Use parkhub.test domain for admin email instead of localhost


## [1.2.1] - 2026-03-14

### Added

- Toggleable UX experience modules + PWA + i18n for all 10 locales
- Integrate demo seeding into Docker startup
- Solo reset mode with countdown + cancel
- Use-case selector with adaptive theming


### CI

- Add Dependabot grouped updates for minor/patch versions
- Add workflow_dispatch trigger to all workflows
- Add Dependabot version update config


### Dependencies

- Bump tray-icon from 0.18.0 to 0.21.3 (#10)
- Bump toml from 0.8.2 to 0.9.12+spec-1.1.0 (#9)
- Bump windows-sys from 0.59.0 to 0.61.2 (#8)
- Bump actions/checkout from 4 to 6 (#7)
- Bump docker/metadata-action from 5 to 6 (#6)
- Bump actions/download-artifact from 4 to 8 (#5)
- Bump actions/upload-artifact from 4 to 7 (#4)
- Bump docker/login-action from 3 to 4 (#3)


### Documentation

- Add compliance badges and regulatory coverage table to README
- Legal compliance audit — GDPR, TTDSG, BFSG, international


### Fixed

- Rebrand all securanido references to parkhub
- Resolve TypeScript errors in Framer Motion ease types and FeaturesContext
- Update workspace version to 1.2.0 and Axum docs to 0.8
- Resolve all CI failures (clippy, fmt, docker-publish triggers)
- Resolve CI failures (fmt, clippy, rust-embed placeholder)
- Accessibility, reduced-motion, i18n completeness, and service worker versioning
- Cargo update — aws-lc-sys 0.37.1→0.38.0 (3 high CVEs)
- Update dependencies — resolve 3 high-severity aws-lc-sys CVEs
- Move global declaration before usage in seed_demo.py
- Use 127.0.0.1 instead of localhost in container
- Add missing CSS classes, refine Welcome page decorations
- Whitelist Docker scripts in.dockerignore, fix login placeholder
- Replace parkhub-demo.de with parkhub.test domain
- Add missing auth.passwordConfirmation to all locales
- Align DemoOverlay with actual API response shape
- Update quinn-proto to 0.11.14 to resolve DoS vulnerability
- Correct dtolnay/rust-toolchain action name


### Design

- Industrial-luxury aesthetic rework


## [1.2.0] - 2026-03-13

### Added

- Astro 6 frontend, credits system, API parity with PHP edition
- Add demo overlay with 30-min countdown, collaborative vote reset, viewer count
- V1.2.0 — audit logging, booking email, profile editing, admin UI, search filters, Koyeb
- Add Render.com demo deployment config


### Chore

- Comprehensive audit — fix docs, env vars, CI, docker-compose, and config accuracy
- Link footer legal pages to centralized legal hub


### Documentation

- Sync v1.2.0 — update badge, backfill docs/CHANGELOG.md
- Add v1.1.1 changelog entry


### Fixed

- Security audit + industrial precision UI redesign
- Axum 0.8 path params and Dockerfile for Render deployment
- Enable rust_crypto feature for jsonwebtoken 10
- GHCR image build pipeline + Dockerfile/render.yaml corrections
- Commit Cargo.lock for reproducible Docker builds


### Devops

- Dockerfile hardening, compose cleanup, CI permissions fixes


### Security

- Comprehensive audit — fix critical vulns, update deps to 2026
- Restrict GitHub Actions to minimal permissions


## [1.1.1] - 2026-02-28

### Fixed

- Enforce allow_self_registration flag + fix floor_name UUID display


## [1.1.0] - 2026-02-28

### Added

- Wire per-endpoint rate limiting middleware
- Add cookie policy, Widerrufsbelehrung, transparency page; security audit updated
- SMTP email, password reset, invoice endpoint, token refresh
- Legal pages, cookie consent, GDPR portal, admin UIs


### CI

- Add release build and Docker image build steps
- Add GitHub Actions workflows (CI + release)


### Documentation

- Prepare v1.1.0 release — CHANGELOG, legal templates, installation docs, VVT template
- Comprehensive documentation suite — installation, API, GDPR, security, changelog
- World-class README, issue templates, PR template


### Fixed

- P0+P1 security hardening — JWT entropy, HSTS, CSP, past-booking validation, slot race fix, X-ForwardedFor trust, email hooks
- Add curl to Alpine build deps for utoipa-swagger-ui download
- Build parkhub-server with headless feature flag
- Exclude parkhub-client from server Docker build workspace
- Use rust:alpine (latest) to satisfy workspace MSRV requirements
- Upgrade Rust builder to 1.85 for edition2024 support
- Add parkhub-client workspace member to Dockerfile
- Suppress unused parameter warning in getLotDetailed mock
- Comprehensive UX polish — empty states, loading states, error handling, mobile, a11y
- Health checks, named volumes, restart policy,.env.example, override example
- Deep audit fixes — password reset pages, admin endpoints, UX polish


## [1.0.0] - 2026-02-27

### Added

- V1.0.0 release preparation — security, accessibility, docs
- Frontend redesign WIP
- Embed web frontend + Docker support
- Add web frontend (React + TailwindCSS)
- Integrate production infrastructure into API
- Add production-ready infrastructure modules (Phase 1-2)


### Chore

- Sync changes
- Trigger CI test


### Documentation

- Add root CHANGELOG.md, fix version 0.1.0 → 1.0.0


### Fixed

- Restore db.rs module with full implementation
- Add rand_core feature for argon2 password hashing


### Tests

- Webhook trigger



