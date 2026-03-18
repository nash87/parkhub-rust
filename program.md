# ParkHub Autoresearch Program

Autonomous improvement loop for ParkHub Rust, inspired by karpathy/autoresearch.

## Oracle

The fixed evaluation oracle is the autoimprove harness:

```bash
flatpak-spawn --host bash /var/home/florian/dev/autoimprove/harness.sh --service parkhub-rust
```

**Metric**: `score` from harness output. Higher is better.
**Score formula**: `(tests * 10) - (fails * 20) - (clippy_warn * 2) - (clippy_err * 10)`

## Rules

1. **Never modify** this file or `harness.sh`
2. **One change per iteration** — add one test, fix one warning, or improve one handler
3. **Run harness before AND after** every change
4. **Keep if score improves**, `git restore` if it drops
5. **Log every iteration** to `results.tsv` regardless of outcome
6. **Commit every kept change** with prefix `autoimprove:`

## High-ROI Targets

### Tests (each new passing test = +10 points)
- `parkhub-server/src/api/setup.rs` — test setup_status, setup_init
- `parkhub-server/src/api/webhooks.rs` — test SSRF validation, HMAC signing
- `parkhub-server/src/api/push.rs` — test subscribe/unsubscribe
- `parkhub-server/src/api/export.rs` — test CSV injection protection
- `parkhub-server/src/api/lots.rs` — test slot CRUD, lot create with defaults
- `parkhub-server/src/db.rs` — test clear_all_data, webhook CRUD, push CRUD

### Clippy (each warning fixed = +2 points)
- Check `cargo clippy --workspace` for new warnings after changes

### Quality (no direct score impact, but prevents regressions)
- Error handling: replace remaining `unwrap()` with proper error propagation
- Input validation: add bounds checking on user inputs
- Documentation: add `///` doc comments to public API handlers

## Iteration Template

```bash
# 1. Record baseline
BASELINE=$(flatpak-spawn --host bash /var/home/florian/dev/autoimprove/harness.sh --service parkhub-rust 2>&1 | grep "score:" | awk '{print $2}')

# 2. Make ONE change (e.g., add a test)
# Edit file...

# 3. Verify it compiles + tests pass
cargo test --package parkhub-server

# 4. Rerun harness
NEW=$(flatpak-spawn --host bash /var/home/florian/dev/autoimprove/harness.sh --service parkhub-rust 2>&1 | grep "score:" | awk '{print $2}')

# 5. Keep or revert
if [ "$NEW" -gt "$BASELINE" ]; then
  git add -A && git commit -m "autoimprove: <description>"
  echo -e "$(date -Is)\t$NEW\t+$((NEW-BASELINE))\tkeep\t<description>" >> results.tsv
else
  git restore .
  echo -e "$(date -Is)\t$BASELINE\t$((NEW-BASELINE))\trevert\t<description>" >> results.tsv
fi
```

## Build Command

```bash
RUSTC_WRAPPER="" RUSTC=/home/florian/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc cargo test --workspace
```

## Current Score Baseline

- Tests: 35 passing, 0 failing
- Clippy: 0 errors (server crate)
- Last harness score: ~84,181 (full harness, all services)
