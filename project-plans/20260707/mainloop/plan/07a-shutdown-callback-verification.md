# Phase 07a: P07 Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P07a`

## Verifies
P07 (Shutdown Sequence + C-to-Rust Callback + Wiring)

## Requirements Verified
REQ-ML-008, REQ-ML-009

## Verification Gate
```bash
cargo test --workspace -- mainloop::shutdown mainloop::game_loop::rust_dispatch
cargo clippy --workspace --all-targets -- -D warnings
grep -RIn "TODO\|FIXME\|HACK" rust/src/mainloop/shutdown.rs rust/src/mainloop/game_loop.rs
# Backward compat: build with flag OFF
cd sc2 && ./build.sh uqm
```

## Checklist
- [ ] `test_shutdown_calls_in_order` PASS (UninitGameKernel → FreeMasterShipList → FreeKernel)
- [ ] `test_rust_dispatch_activity_callable` PASS
- [ ] `rust_dispatch_activity` is `#[no_mangle] pub extern "C"`
- [ ] C `Starcon2Main` has `#ifdef USE_RUST_MAINLOOP` delegating to `rust_game_loop`
- [ ] Binary builds with flag OFF (backward compat preserved)
- [ ] Zero deferred-implementation hits

## Decision
- [ ] PASS → proceed to P08
- [ ] FAIL → remediate P07 before continuing
